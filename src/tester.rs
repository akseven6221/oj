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
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        // 发送测试指令到标准输入
        if let Some(mut stdin) = child.stdin.take() {
            tokio::spawn(async move {
                stdin.write_all(b"usertests\n").await?;
                stdin.flush().await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            });
        }

        // 获取stdout以便后续读取
        let stdout = child.stdout.take();

        let result = tokio::time::timeout(
            Duration::from_secs(300), // 延长到5分钟超时
            child.wait_with_output()
        ).await;

        // 等待1秒确保输出完成
        tokio::time::sleep(Duration::from_secs(1)).await;

        match result {
            Ok(Ok(output)) => {
                // 读取之前保存的stdout中的剩余输出
                let mut remaining = String::new();
                if let Some(mut stdout) = stdout {
                    let mut reader = tokio::io::BufReader::new(stdout);
                    reader.read_to_string(&mut remaining).await?;
                }

                let mut final_output = output.stdout;
                final_output.extend_from_slice(remaining.as_bytes());
                
                let stdout_str = String::from_utf8_lossy(&final_output).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                // 检查输出中是否包含成功标记
                if stdout_str.contains("Usertests passed!") {
                    Ok((TestStatus::Passed, stdout_str, None))
                } else {
                    Ok((
                        TestStatus::Failed,
                        stdout_str,
                        if stderr.is_empty() { None } else { Some(stderr) },
                    ))
                }
            },
            Ok(Err(e)) => return Err(format!("执行make命令失败: {}", e).into()),
            Err(_) => return Err("测试执行超时".into()),
        }
    }
}