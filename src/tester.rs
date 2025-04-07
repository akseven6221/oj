use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::process::Command;
use std::collections::VecDeque;
use crate::models::{TestTask, TestStatus};
use crate::database::TestRepo;
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
                let result = self.run_test(&task).await;
                let status_clone = result.0.clone(); // 克隆状态以便后续使用

                // 更新测试结果
                if let Err(e) = TestRepo::update_test_result(
                    &self.db_pool,
                    task.id,
                    status_clone, // 使用克隆的状态
                    Some(result.1),
                    result.2,
                ).await {
                    tracing::error!("Failed to save test result: {}", e);
                }
                
                let status_clone2 = result.0.clone(); // 再次克隆状态用于日志
                tracing::info!("Test for user {} completed with status: {:?}", 
                              task.username, status_clone2);
            } else {
                // 如果队列为空，等待一段时间
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }

    // 运行测试
    async fn run_test(&self, task: &TestTask) -> (TestStatus, String, Option<String>) {
        let work_dir = std::path::Path::new(&task.work_dir);
        
        // 检查工作目录是否存在
        if !work_dir.exists() {
            return (
                TestStatus::Error,
                String::new(),
                Some(format!("工作目录不存在: {}", task.work_dir)),
            );
        }

        // 检查OS目录是否存在
        let os_dir = work_dir.join("os");
        if !os_dir.exists() {
            return (
                TestStatus::Error,
                String::new(),
                Some(format!("OS目录不存在: {}/os", task.work_dir)),
            );
        }

        // 使用超时设置运行make命令
        let mut child = Command::new("make")
            .arg("run")
            .env("RUSTUP_TOOLCHAIN", "nightly-2024-04-29")
            .current_dir(&os_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                return (
                    TestStatus::Error,
                    String::new(),
                    Some(format!("进程启动失败: {}", e))
                );
            })?;

        // 发送测试指令到标准输入
        if let Some(mut stdin) = child.stdin.take() {
            tokio::spawn(async move {
                stdin.write_all(b"usertests\n").await
                    .map_err(|e| format!("输入指令失败: {}", e))
            });
        }

        let result = tokio::time::timeout(
            Duration::from_secs(300), // 延长到5分钟超时
            child.wait_with_output()
        ).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                // 检查输出中是否包含成功标记
                if stdout.contains("Usertests passed!") {
                    (TestStatus::Passed, stdout, None)
                } else {
                    (
                        TestStatus::Failed,
                        stdout,
                        if stderr.is_empty() { None } else { Some(stderr) },
                    )
                }
            },
            Ok(Err(e)) => (
                TestStatus::Error,
                String::new(),
                Some(format!("执行make命令失败: {}", e)),
            ),
            Err(_) => (
                TestStatus::Error,
                String::new(),
                Some("测试执行超时".to_string()),
            ),
        }
    }
}