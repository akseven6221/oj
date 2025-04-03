use crate::models::{User, UserRole};
use sqlx::{mysql::MySqlPool, Row};
use std::env;

pub type DbPool = MySqlPool;
pub type DbError = sqlx::Error;

// 初始化数据库连接池
pub async fn init_db() -> Result<DbPool, DbError> {
    // 从环境变量获取数据库URL
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in environment variables or .env file");
    
    let pool = MySqlPool::connect(&database_url).await?;
    
    // 初始化数据库表
    init_tables(&pool).await?;
    
    // 初始化默认用户
    init_default_users(&pool).await?;
    
    Ok(pool)
}

// 初始化数据库表
async fn init_tables(pool: &DbPool) -> Result<(), DbError> {
    // 创建用户表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INT AUTO_INCREMENT PRIMARY KEY,
            username VARCHAR(50) NOT NULL UNIQUE,
            password VARCHAR(255) NOT NULL,
            role VARCHAR(20) NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;
    
    // 创建会话表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id VARCHAR(36) PRIMARY KEY,
            user_id INT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;
    
    // 创建上传记录表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS uploads (
            id INT AUTO_INCREMENT PRIMARY KEY,
            user_id INT NOT NULL,
            filename VARCHAR(255) NOT NULL,
            file_path VARCHAR(255) NOT NULL,
            file_size BIGINT NOT NULL,
            uploaded_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

// 初始化默认用户
async fn init_default_users(pool: &DbPool) -> Result<(), DbError> {
    // 检查管理员是否存在
    let admin_exists = sqlx::query("SELECT 1 FROM users WHERE username = 'admin'")
        .fetch_optional(pool)
        .await?
        .is_some();
    
    // 如果管理员不存在，则创建默认管理员
    if !admin_exists {
        sqlx::query(
            r#"
            INSERT INTO users (username, password, role)
            VALUES ('admin', 'adminpass', 'admin')
            "#,
        )
        .execute(pool)
        .await?;
    }
    
    // 检查普通用户是否存在
    let user_exists = sqlx::query("SELECT 1 FROM users WHERE username = 'user'")
        .fetch_optional(pool)
        .await?
        .is_some();
    
    // 如果普通用户不存在，则创建默认普通用户
    if !user_exists {
        sqlx::query(
            r#"
            INSERT INTO users (username, password, role)
            VALUES ('user', 'userpass', 'regular')
            "#,
        )
        .execute(pool)
        .await?;
    }
    
    Ok(())
}

// 用户相关的数据库操作
pub struct UserRepo;

impl UserRepo {
    // 获取所有用户
    pub async fn get_all_users(pool: &DbPool) -> Result<Vec<User>, DbError> {
        let users = sqlx::query(
            r#"
            SELECT id, username, password, role FROM users
            "#,
        )
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|row| {
            let role = match row.get::<String, _>("role").as_str() {
                "admin" => UserRole::Admin,
                _ => UserRole::Regular,
            };
            
            User {
                id: row.get("id"),
                username: row.get("username"),
                password: row.get("password"),
                role,
            }
        })
        .collect();
        
        Ok(users)
    }
    
    // 根据用户名获取用户
    pub async fn get_user_by_username(pool: &DbPool, username: &str) -> Result<Option<User>, DbError> {
        let user = sqlx::query(
            r#"
            SELECT id, username, password, role FROM users
            WHERE username = ?
            "#,
        )
        .bind(username)
        .fetch_optional(pool)
        .await?
        .map(|row| {
            let role = match row.get::<String, _>("role").as_str() {
                "admin" => UserRole::Admin,
                _ => UserRole::Regular,
            };
            
            User {
                id: row.get("id"),
                username: row.get("username"),
                password: row.get("password"),
                role,
            }
        });
        
        Ok(user)
    }
    
    // 根据ID获取用户
    pub async fn get_user_by_id(pool: &DbPool, id: i32) -> Result<Option<User>, DbError> {
        let user = sqlx::query(
            r#"
            SELECT id, username, password, role FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .map(|row| {
            let role = match row.get::<String, _>("role").as_str() {
                "admin" => UserRole::Admin,
                _ => UserRole::Regular,
            };
            
            User {
                id: row.get("id"),
                username: row.get("username"),
                password: row.get("password"),
                role,
            }
        });
        
        Ok(user)
    }
    
    // 创建新用户
    pub async fn create_user(pool: &DbPool, username: &str, password: &str, role: UserRole) -> Result<i32, DbError> {
        let role_str = match role {
            UserRole::Admin => "admin",
            UserRole::Regular => "regular",
        };
        
        let result = sqlx::query(
            r#"
            INSERT INTO users (username, password, role)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(username)
        .bind(password)
        .bind(role_str)
        .execute(pool)
        .await?;
        
        Ok(result.last_insert_id() as i32)
    }
    
    // 更新用户
    pub async fn update_user(pool: &DbPool, username: &str, password: Option<&str>, role: Option<UserRole>) -> Result<bool, DbError> {
        // 先获取用户ID
        let user = Self::get_user_by_username(pool, username).await?;
        
        if let Some(user) = user {
            // 更新密码
            if let Some(password) = password {
                sqlx::query(
                    r#"
                    UPDATE users
                    SET password = ?
                    WHERE id = ?
                    "#,
                )
                .bind(password)
                .bind(user.id)
                .execute(pool)
                .await?;
            }
            
            // 更新角色
            if let Some(role) = role {
                let role_str = match role {
                    UserRole::Admin => "admin",
                    UserRole::Regular => "regular",
                };
                
                sqlx::query(
                    r#"
                    UPDATE users
                    SET role = ?
                    WHERE id = ?
                    "#,
                )
                .bind(role_str)
                .bind(user.id)
                .execute(pool)
                .await?;
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    // 删除用户
    pub async fn delete_user(pool: &DbPool, username: &str) -> Result<bool, DbError> {
        let result = sqlx::query(
            r#"
            DELETE FROM users
            WHERE username = ?
            "#,
        )
        .bind(username)
        .execute(pool)
        .await?;
        
        Ok(result.rows_affected() > 0)
    }
}

// 会话相关的数据库操作
pub struct SessionRepo;

impl SessionRepo {
    // 创建新会话
    pub async fn create_session(pool: &DbPool, session_id: &str, user_id: i32) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id)
            VALUES (?, ?)
            "#,
        )
        .bind(session_id)
        .bind(user_id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    // 获取会话
    pub async fn get_session(pool: &DbPool, session_id: &str) -> Result<Option<i32>, DbError> {
        let user_id = sqlx::query(
            r#"
            SELECT user_id FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(pool)
        .await?
        .map(|row| row.get::<i32, _>("user_id"));
        
        Ok(user_id)
    }
    
    // 删除会话
    pub async fn delete_session(pool: &DbPool, session_id: &str) -> Result<(), DbError> {
        sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(session_id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
    
    // 清理指定用户的所有会话
    pub async fn clear_user_sessions(pool: &DbPool, user_id: i32) -> Result<(), DbError> {
        sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE user_id = ?
            "#,
        )
        .bind(user_id)
        .execute(pool)
        .await?;
        
        Ok(())
    }
}

// 上传记录相关的数据库操作
pub struct UploadRepo;

impl UploadRepo {
    // 记录文件上传
    pub async fn record_upload(
        pool: &DbPool,
        user_id: i32,
        filename: &str,
        file_path: &str,
        file_size: i64,
    ) -> Result<i32, DbError> {
        let result = sqlx::query(
            r#"
            INSERT INTO uploads (user_id, filename, file_path, file_size)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(user_id)
        .bind(filename)
        .bind(file_path)
        .bind(file_size)
        .execute(pool)
        .await?;
        
        Ok(result.last_insert_id() as i32)
    }
    
    // 获取用户的上传记录
    pub async fn get_user_uploads(pool: &DbPool, user_id: i32) -> Result<Vec<sqlx::mysql::MySqlRow>, DbError> {
        let uploads = sqlx::query(
            r#"
            SELECT id, filename, file_path, file_size, uploaded_at
            FROM uploads
            WHERE user_id = ?
            ORDER BY uploaded_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;
        
        Ok(uploads)
    }
    
    // 获取所有上传记录（管理员使用）
    pub async fn get_all_uploads(pool: &DbPool) -> Result<Vec<sqlx::mysql::MySqlRow>, DbError> {
        let uploads = sqlx::query(
            r#"
            SELECT u.id, u.filename, u.file_path, u.file_size, u.uploaded_at, us.username
            FROM uploads u
            JOIN users us ON u.user_id = us.id
            ORDER BY u.uploaded_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;
        
        Ok(uploads)
    }
}