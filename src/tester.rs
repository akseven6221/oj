use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::process::Command;
use std::collections::VecDeque;
use crate::models::{TestTask, TestStatus};
use crate::database::TestRepo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use sqlx::mysql::MySqlPool;

pub struct TestQueue {
    queue: Mutex<VecDeque<TestTask>>,
    db_pool: Arc<MySqlPool>,
}

impl TestQueue {
    pub fn new(db_pool: Arc<MySqlPool>) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            db_pool,
        }
    }

    // 添加任务到队列
    pub async fn add_task(&self, task: TestTask) {
        let username = task.username.clone(); // 克隆用户名以备后用
        let mut queue = self.queue.lock().await;
        queue.push_back(task);
        tracing::info!("Added test task for user {} to queue", username);
    }

    // 启动评测工作器
    pub async fn start_worker(self: Arc<Self>) {
        tracing::info!("Test worker started");
        loop {
            // 尝试获取任务
            let task = {
                let mut queue = self.queue.lock().await;
                queue.pop_front()
            };

            if let Some(task) = task {
                tracing::info!("Processing test task for user {}", task.username);
                
                // 更新状态为运行中
                if let Err(e) = TestRepo::update_test_status(
                    &self.db_pool,
                    task.id,
                    TestStatus::Running,
                ).await {
                    tracing::error!("Failed to update test status: {}", e);
                    continue;
                }

                // 运行测试
                let (status, output, error) = match self.run_test(&task).await {
                    Ok(res) => res,
                    Err(e) => {
                        tracing::error!("测试执行错误: {}", e);
                        continue;
                    }
                };
                let status_clone = status.clone();

                // 更新测试结果
                if let Err(e) = TestRepo::update_test_result(
                    &self.db_pool,
                    task.id,
                    status_clone, // 使用克隆的状态
                    Some(output),
                    error,
                ).await {
                    tracing::error!("Failed to save test result: {}", e);
                }
                
                let status_clone2 = status.clone(); // 再次克隆状态用于日志
                tracing::info!("Test for user {} completed with status: {:?}", 
                              task.username, status_clone2);
            } else {
                // 如果队列为空，等待一段时间
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    // 运行测试
    async fn run_test(&self, task: &TestTask) -> Result<(TestStatus, String, Option<String>), Box<dyn std::error::Error + Send + Sync>> {
        let work_dir = std::path::Path::new(&task.work_dir);
        
        // 检查工作目录是否存在
        if !work_dir.exists() {
            return Err(format!("工作目录不存在: {}", task.work_dir).into());
        }

        // 检查OS目录是否存在
        let os_dir = work_dir.join("os");
        if !os_dir.exists() {
            return Err(format!("OS目录不存在: {}/os", task.work_dir).into());
        }

        // 使用超时设置运行make命令
        let mut child = Command::new("make")
            .arg("run")
            .env("RUSTUP_TOOLCHAIN", "nightly-2024-04-29")
            .current_dir(&os_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("进程启动失败: {}", e))?;

        // 发送测试指令到标准输入
        if let Some(mut stdin) = child.stdin.take() {
            tokio::spawn(async move {
                stdin.write_all(b"usertests\n").await?;
                stdin.flush().await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            });
        }

        // 获取stdout并设置缓冲读取
        let mut stdout = tokio::io::BufReader::new(child.stdout.take().unwrap());
        let mut output = String::new();
        let mut buffer = [0; 1024];
        let mut passed = false;

        // 设置超时时间
        let timeout = tokio::time::sleep(Duration::from_secs(300));
        tokio::pin!(timeout);

        loop {
            tokio::select! {
                read_result = stdout.read(&mut buffer) => {
                    match read_result {
                        Ok(n) if n > 0 => {
                            let chunk = String::from_utf8_lossy(&buffer[..n]).to_string();
                            output.push_str(&chunk);
                            
                            // 实时更新输出到数据库
                            if let Err(e) = TestRepo::update_test_result(
                                &self.db_pool,
                                task.id,
                                if output.contains("Usertests passed!") {
                                    passed = true;
                                    TestStatus::Passed
                                } else {
                                    TestStatus::Running
                                },
                                Some(output.clone()),
                                None,
                            ).await {
                                tracing::error!("Failed to update test output: {}", e);
                            }

                            // 如果测试通过或失败，立即终止qemu进程
                            if passed || output.contains("FAILED") {
                                // 获取所有qemu进程的PID并终止它们
                                if let Ok(pgrep_output) = Command::new("pgrep")
                                    .arg("-a")
                                    .arg("qemu")
                                    .output()
                                    .await {
                                    if let Ok(pids_str) = String::from_utf8(pgrep_output.stdout) {
                                        for line in pids_str.lines() {
                                            if let Some(pid) = line.split_whitespace().next() {
                                                if let Err(e) = Command::new("kill")
                                                    .arg("-9")
                                                    .arg(pid)
                                                    .output()
                                                    .await {
                                                    tracing::warn!("Failed to kill qemu process {}: {}", pid, e);
                                                } else {
                                                    tracing::info!("Successfully terminated qemu process {}", pid);
                                                }
                                            }
                                        }
                                    }
                                }
                                break;
                            }
                        },
                        Ok(_) => break, // EOF
                        Err(e) => return Err(format!("读取输出失败: {}", e).into()),
                    }
                }
                _ = &mut timeout => {
                    return Err("测试执行超时".into());
                }
            }
        }

        // 获取所有qemu进程的PID并终止它们
        let pgrep_output = match Command::new("pgrep")
            .arg("-a")
            .arg("qemu")
            .output()
            .await {
            Ok(output) => output,
            Err(e) => {
                tracing::warn!("Failed to get qemu processes: {}", e);
                return Ok((TestStatus::Failed, output, Some(format!("Failed to get qemu processes: {}", e))));
            }
        };

        // 解析pgrep输出获取PID列表
        if let Ok(pids_str) = String::from_utf8(pgrep_output.stdout) {
            for line in pids_str.lines() {
                if let Some(pid) = line.split_whitespace().next() {
                    // 对每个PID执行kill命令
                    if let Err(e) = Command::new("kill")
                        .arg("-9")
                        .arg(pid)
                        .output()
                        .await {
                        tracing::warn!("Failed to kill qemu process {}: {}", pid, e);
                    } else {
                        tracing::info!("Successfully terminated qemu process {}", pid);
                    }
                }
            }
        }

        // 根据测试输出结果判断状态
        if passed {
            Ok((TestStatus::Passed, output, None))
        } else {
            Ok((TestStatus::Failed, output, None))
        }
    }
}