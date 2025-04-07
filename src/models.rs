use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sqlx::mysql::MySqlPool;

use crate::tester::TestQueue;

// 用户角色枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UserRole {
    Admin,
    Regular,
}

// 用户结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String, // 实际应用中应该存储密码哈希
    pub role: UserRole,
}

// 会话结构
#[derive(Debug, Clone)]
pub struct Session {
    pub user: User,
}

// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub db_pool: Arc<MySqlPool>,
    pub test_queue: Arc<TestQueue>, // 新增
}

// 登录表单
#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

// 添加用户管理相关的结构体
#[derive(Deserialize)]
pub struct UserCreateForm {
    pub username: String,
    pub password: String,
    pub role: String,  // "admin" 或 "regular"
}

#[derive(Deserialize)]
pub struct UserUpdateForm {
    pub password: Option<String>,
    pub role: Option<String>,
}

// 文件上传记录
#[derive(Debug, Clone, Serialize)]
pub struct UploadRecord {
    pub id: i32,
    pub filename: String,
    pub file_path: String,
    pub file_size: i64,
    pub uploaded_at: chrono::DateTime<chrono::Utc>,
    pub username: Option<String>,
}

// 评测状态枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TestStatus {
    Pending,    // 等待评测
    Running,    // 评测中
    Passed,     // 通过
    Failed,     // 失败
    Error       // 发生错误
}

// 评测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub id: i32,
    pub user_id: i32,
    pub username: String,
    pub status: TestStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// 评测任务
#[derive(Debug, Clone)]
pub struct TestTask {
    pub id: i32,
    pub user_id: i32,
    pub username: String,
    pub work_dir: String,
}

impl AppState {
    pub fn new(pool: MySqlPool, test_queue: Arc<TestQueue>) -> Self {
        AppState {
            db_pool: Arc::new(pool),
            test_queue,
        }
    }
}