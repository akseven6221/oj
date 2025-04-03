use crate::database::{SessionRepo, UploadRepo, UserRepo};
// 删除未使用的 Session 导入
use crate::models::{AppState, LoginForm, User, UserRole, UserCreateForm, UserUpdateForm, UploadRecord};
use crate::templates::{index_template, login_template, admin_panel_template, uploads_template};
use axum::{
    extract::{Extension, Form, Multipart, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use std::{io::Write, path::PathBuf};
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;
use sqlx::Row; // 添加这一行导入

// 登录页面
pub async fn login_page() -> impl IntoResponse {
    Html(login_template())
}

// 登录处理
pub async fn login_handler(
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    tracing::info!("Login attempt for user: {}", form.username);
    
    // 查询用户
    let user_result = UserRepo::get_user_by_username(&state.db_pool, &form.username).await;
    
    match user_result {
        Ok(Some(user)) => {
            if user.password == form.password {
                // 创建会话ID
                let session_id = Uuid::new_v4().to_string();
                
                // 在数据库中记录会话
                if let Err(e) = SessionRepo::create_session(&state.db_pool, &session_id, user.id).await {
                    tracing::error!("Failed to create session: {}", e);
                    return Html("<script>alert('登录失败，请重试'); window.location.href='/login';</script>".to_string()).into_response();
                }
                
                // 设置cookie
                let mut cookie = Cookie::new("session_id", session_id);
                cookie.set_http_only(true);
                cookie.set_path("/");
                cookies.add(cookie);
                
                tracing::info!("User {} logged in successfully", user.username);
                
                Html(r#"<script>alert('登录成功!'); window.location.href='/';</script>"#.to_string()).into_response()
            } else {
                tracing::warn!("Invalid password for user: {}", form.username);
                Html("<script>alert('用户名或密码错误'); window.location.href='/login';</script>".to_string()).into_response()
            }
        }
        Ok(None) => {
            tracing::warn!("User not found: {}", form.username);
            Html("<script>alert('用户名或密码错误'); window.location.href='/login';</script>".to_string()).into_response()
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Html("<script>alert('登录失败，请重试'); window.location.href='/login';</script>".to_string()).into_response()
        }
    }
}

// 注销处理
pub async fn logout_handler(cookies: Cookies, State(state): State<AppState>) -> impl IntoResponse {
    if let Some(cookie) = cookies.get("session_id") {
        let session_id = cookie.value().to_string();
        
        // 从数据库中删除会话
        if let Err(e) = SessionRepo::delete_session(&state.db_pool, &session_id).await {
            tracing::error!("Failed to delete session: {}", e);
        }
        
        // 删除cookie
        let mut removal_cookie = Cookie::new("session_id", "");
        removal_cookie.set_path("/");
        removal_cookie.set_max_age(time::Duration::seconds(-1));
        cookies.add(removal_cookie);
        
        tracing::info!("User logged out");
    }
    
    Redirect::to("/login")
}

// 首页处理函数
pub async fn index_handler(Extension(user): Extension<User>) -> impl IntoResponse {
    Html(index_template(&user.username, &user.role))
}

// 文件上传处理函数
pub async fn upload_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // 使用用户名创建用户专属文件夹，而不是基于角色
    let user_folder = format!("uploads/{}", user.username);
    
    // 确保用户文件夹存在
    if let Err(e) = tokio::fs::create_dir_all(&user_folder).await {
        tracing::error!("Failed to create user directory: {}", e);
        return Html("<script>alert('创建用户目录失败！'); window.location.href='/';</script>".to_string()).into_response();
    }
    
    let mut uploaded = false;
    let mut upload_info = None;
    
    while let Ok(Some(field)) = multipart.next_field().await {
        if let Some(file_name) = field.file_name().map(|s| s.to_string()) {
            if let Ok(data) = field.bytes().await {
                let upload_path = PathBuf::from(&user_folder).join(&file_name);
                let file_path_str = upload_path.to_string_lossy().to_string();
                
                if let Ok(mut file) = std::fs::File::create(&upload_path) {
                    if file.write_all(&data).is_ok() {
                        // 记录上传到数据库
                        match UploadRepo::record_upload(
                            &state.db_pool,
                            user.id,
                            &file_name,
                            &file_path_str,
                            data.len() as i64,
                        ).await {
                            Ok(_) => {
                                uploaded = true;
                                upload_info = Some((file_name, data.len() as i64));
                            }
                            Err(e) => {
                                tracing::error!("Failed to record upload: {}", e);
                                // 删除已上传的文件
                                let _ = std::fs::remove_file(upload_path);
                            }
                        }
                    }
                }
            }
        }
    }
    
    if uploaded {
        if let Some((filename, size)) = upload_info {
            // 修复返回类型错误
            return Html(format!(
                r#"<script>alert('文件 {} ({} 字节) 上传成功！'); window.location.href='/';</script>"#,
                filename, size
            )).into_response();
        } else {
            // 修复返回类型错误
            return Html("<script>alert('上传成功！'); window.location.href='/';</script>".to_string()).into_response();
        }
    } else {
        // 修复返回类型错误
        return Html("<script>alert('上传失败！'); window.location.href='/';</script>".to_string()).into_response();
    }
}

// 查看上传记录
pub async fn view_uploads(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let uploads_result = if matches!(user.role, UserRole::Admin) {
        // 管理员可以查看所有上传
        UploadRepo::get_all_uploads(&state.db_pool).await
    } else {
        // 普通用户只能查看自己的上传
        UploadRepo::get_user_uploads(&state.db_pool, user.id).await
    };
    
    match uploads_result {
        Ok(rows) => {
            let uploads: Vec<UploadRecord> = rows.into_iter().map(|row| {
                UploadRecord {
                    id: row.get("id"),
                    filename: row.get("filename"),
                    file_path: row.get("file_path"),
                    file_size: row.get("file_size"),
                    uploaded_at: row.get("uploaded_at"),
                    username: row.try_get("username").ok(),
                }
            }).collect();
            
            Html(uploads_template(&user, &uploads)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get uploads: {}", e);
            Html("<script>alert('获取上传记录失败！'); window.location.href='/';</script>".to_string()).into_response()
        }
    }
}

// 管理员面板
pub async fn admin_panel(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 检查是否是管理员 - 移除不必要的括号
    if !matches!(user.role, UserRole::Admin) {
        return Html(
            "<script>alert('只有管理员才能访问此页面'); window.location.href='/';</script>".to_string(),
        )
        .into_response();
    }
    
    // 从数据库获取所有用户
    match UserRepo::get_all_users(&state.db_pool).await {
        Ok(users) => Html(admin_panel_template(&users, None, None)).into_response(),
        Err(e) => {
            tracing::error!("Failed to get users: {}", e);
            Html("<script>alert('获取用户列表失败！'); window.location.href='/';</script>".to_string()).into_response()
        }
    }
}

// 创建用户
pub async fn create_user(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Form(form): Form<UserCreateForm>,
) -> impl IntoResponse {
    // 检查是否是管理员 - 移除不必要的括号
    if !matches!(user.role, UserRole::Admin) {
        return Html(
            "<script>alert('只有管理员才能创建用户'); window.location.href='/';</script>".to_string(),
        )
        .into_response();
    }
    
    // 转换角色
    let role = match form.role.as_str() {
        "admin" => UserRole::Admin,
        _ => UserRole::Regular,
    };
    
    // 添加用户到数据库
    match UserRepo::create_user(&state.db_pool, &form.username, &form.password, role).await {
        Ok(_) => {
            match UserRepo::get_all_users(&state.db_pool).await {
                Ok(users) => Html(admin_panel_template(&users, None, Some("用户创建成功！"))).into_response(),
                Err(e) => {
                    tracing::error!("Failed to get users: {}", e);
                    Html("<script>alert('用户创建成功，但获取用户列表失败！'); window.location.href='/admin/users';</script>".to_string()).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            
            let error_message = if e.to_string().contains("Duplicate entry") {
                "用户名已存在".to_string()
            } else {
                format!("创建用户失败: {}", e)
            };
            
            match UserRepo::get_all_users(&state.db_pool).await {
                Ok(users) => Html(admin_panel_template(&users, Some(&error_message), None)).into_response(),
                Err(_) => {
                    // 修复 format! 参数错误
                    Html(format!(
                        "<script>alert('{}'); window.location.href='/admin/users';</script>",
                        error_message
                    )).into_response()
                }
            }
        }
    }
}

// 更新用户
pub async fn update_user(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(username): Path<String>,
    Form(form): Form<UserUpdateForm>,
) -> impl IntoResponse {
    // 检查是否是管理员 - 移除不必要的括号
    if !matches!(user.role, UserRole::Admin) {
        return Html(
            "<script>alert('只有管理员才能更新用户'); window.location.href='/';</script>".to_string(),
        )
        .into_response();
    }
    
    // 转换角色
    let role = form.role.as_deref().map(|r| match r {
        "admin" => UserRole::Admin,
        _ => UserRole::Regular,
    });
    
    // 更新用户
    match UserRepo::update_user(
        &state.db_pool,
        &username,
        form.password.as_deref(),
        role,
    ).await {
        Ok(true) => {
            match UserRepo::get_all_users(&state.db_pool).await {
                Ok(users) => Html(admin_panel_template(&users, None, Some(&format!("用户 {} 更新成功！", username)))).into_response(),
                Err(e) => {
                    tracing::error!("Failed to get users: {}", e);
                    Html("<script>alert('用户更新成功，但获取用户列表失败！'); window.location.href='/admin/users';</script>".to_string()).into_response()
                }
            }
        }
        Ok(false) => {
            match UserRepo::get_all_users(&state.db_pool).await {
                Ok(users) => Html(admin_panel_template(&users, Some(&format!("用户 {} 不存在", username)), None)).into_response(),
                Err(e) => {
                    tracing::error!("Failed to get users: {}", e);
                    Html("<script>alert('用户不存在！'); window.location.href='/admin/users';</script>".to_string()).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to update user: {}", e);
            
            match UserRepo::get_all_users(&state.db_pool).await {
                Ok(users) => Html(admin_panel_template(&users, Some(&format!("更新用户失败: {}", e)), None)).into_response(),
                Err(_) => {
                    Html(format!(
                        "<script>alert('更新用户失败: {}'); window.location.href='/admin/users';</script>",
                        e
                    )).into_response()
                }
            }
        }
    }
}

// 删除用户
pub async fn delete_user(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> impl IntoResponse {
    // 检查是否是管理员 - 移除不必要的括号
    if !matches!(user.role, UserRole::Admin) {
        return Html(
            "<script>alert('只有管理员才能删除用户'); window.location.href='/';</script>".to_string(),
        )
        .into_response();
    }
    
    // 不能删除自己 - 移除不必要的括号
    if user.username == username {
        match UserRepo::get_all_users(&state.db_pool).await {
            // 修复 into响应 -> into_response
            Ok(users) => return Html(admin_panel_template(&users, Some("不能删除当前登录的用户"), None)).into_response(),
            Err(_) => {
                // 修复 into响应 -> into_response
                return Html("<script>alert('不能删除当前登录的用户'); window.location.href='/admin/users';</script>".to_string()).into_response()
            }
        }
    }
    
    // 不能删除主管理员 - 移除不必要的括号
    if username == "admin" {
        match UserRepo::get_all_users(&state.db_pool).await {
            // 修复 into响应 -> into_response
            Ok(users) => return Html(admin_panel_template(&users, Some("不能删除主管理员"), None)).into_response(),
            Err(_) => {
                // 修复 into响应 -> into响应
                return Html("<script>alert('不能删除主管理员'); window.location.href='/admin/users';</script>".to_string()).into_response()
            }
        }
    }
    
    // 删除用户
    match UserRepo::delete_user(&state.db_pool, &username).await {
        Ok(true) => {
            match UserRepo::get_all_users(&state.db_pool).await {
                // 修复 into响应 -> into响应
                Ok(users) => Html(admin_panel_template(&users, None, Some(&format!("用户 {} 删除成功！", username)))).into_response(),
                Err(e) => {
                    tracing::error!("Failed to get users: {}", e);
                    // 修复 into响应 -> into响应
                    Html("<script>alert('用户删除成功，但获取用户列表失败！'); window.location.href='/admin/users';</script>".to_string()).into_response()
                }
            }
        }
        Ok(false) => {
            match UserRepo::get_all_users(&state.db_pool).await {
                // 修复 into响应 -> into响应
                Ok(users) => Html(admin_panel_template(&users, Some(&format!("用户 {} 不存在", username)), None)).into_response(),
                Err(e) => {
                    tracing::error!("Failed to get users: {}", e);
                    // 修复 into响应 -> into响应
                    Html("<script>alert('用户不存在！'); window.location.href='/admin/users';</script>".to_string()).into_response()
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete user: {}", e);
            
            match UserRepo::get_all_users(&state.db_pool).await {
                // 修复 into响应 -> into响应 并添加逗号
                Ok(users) => Html(admin_panel_template(&users, Some(&format!("删除用户失败: {}", e)), None)).into_response(),
                Err(_) => {
                    // 修复为 into_response
                    Html(format!(
                        "<script>alert('删除用户失败: {}'); window.location.href='/admin/users';</script>",
                        e
                    )).into_response()
                }
            }
        }
    }
}

// 查看用户所有文件列表
pub async fn view_user_files(
    Extension(user): Extension<User>,
    Path(target_username): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 检查权限：只能查看自己的或者管理员可以查看所有人的
    if user.username != target_username && !matches!(user.role, UserRole::Admin) {
        return Html("<script>alert('您没有权限查看此用户的文件'); window.location.href='/';</script>".to_string()).into_response();
    }
    
    // 获取目标用户信息，加上下划线前缀避免未使用警告
    let _target_user = match UserRepo::get_user_by_username(&state.db_pool, &target_username).await {
        Ok(Some(u)) => u,
        // 修复为 into_response
        Ok(None) => return Html("<script>alert('用户不存在'); window.location.href='/';</script>".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            // 修复为 into_response
            return Html("<script>alert('数据库错误'); window.location.href='/';</script>".to_string()).into_response();
        }
    };
    
    // 读取用户目录内容
    let user_dir = format!("uploads/{}", target_username);
    let mut entries = Vec::new();
    
    // 确保用户目录存在
    match tokio::fs::create_dir_all(&user_dir).await {
        Ok(_) => (),
        Err(e) => {
            tracing::error!("Failed to ensure user directory exists: {}", e);
            // 修复为 into_response
            return Html("<script>alert('无法访问用户目录'); window.location.href='/';</script>".to_string()).into_response();
        }
    }
    
    // 读取目录内容
    match tokio::fs::read_dir(&user_dir).await {
        Ok(mut dir) => {
            while let Ok(Some(entry)) = dir.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_file() {
                        if let Ok(filename) = entry.file_name().into_string() {
                            let size = metadata.len();
                            let modified = metadata.modified().ok().and_then(|time| {
                                time.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
                            });
                            
                            entries.push((filename, size, modified));
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to read user directory: {}", e);
            // 修复为 into_response
            return Html("<script>alert('读取用户目录失败'); window.location.href='/';</script>".to_string()).into_response();
        }
    }
    
    // 构建文件列表 HTML
    let mut files_html = String::new();
    if entries.is_empty() {
        files_html = r#"<div class="empty">此用户没有上传任何文件</div>"#.to_string();
    } else {
        // 按文件名排序
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        
        files_html.push_str(r#"<table class="file-table">
            <thead>
                <tr>
                    <th>文件名</th>
                    <th>大小</th>
                    <th>修改时间</th>
                    <th>操作</th>
                </tr>
            </thead>
            <tbody>"#);
        
        for (filename, size, modified) in entries {
            let size_str = if size < 1024 {
                format!("{} B", size)
            } else if size < 1024 * 1024 {
                format!("{:.2} KB", size as f64 / 1024.0)
            } else {
                format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
            };
            
            // 修复 chrono 废弃方法警告
            let time_str = match modified {
                Some(timestamp) => {
                    // 使用推荐的新方法
                    let dt = match chrono::DateTime::from_timestamp(timestamp as i64, 0) {
                        Some(dt) => dt,
                        None => chrono::Utc::now(),
                    };
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                },
                None => "未知".to_string(),
            };
            
            let download_url = format!("/files/{}/{}", target_username, filename);
            
            files_html.push_str(&format!(r#"
                <tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td><a href="{}" class="download-btn">下载</a></td>
                </tr>
            "#, filename, size_str, time_str, download_url));
        }
        
        files_html.push_str("</tbody></table>");
    }
    
    // 渲染完整页面
    let back_link = if user.username == target_username {
        r#"<a href="/" class="back-btn">返回主页</a>"#
    } else {
        r#"<a href="/admin/users" class="back-btn">返回用户管理</a>"#
    };
    
    let html_content = format!(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>用户文件 - {}</title>
        <style>
            body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
            h1 {{ color: #333; text-align: center; }}
            .user-info {{ background: #f5f5f5; padding: 10px; margin-bottom: 20px; border-radius: 5px; }}
            .file-table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
            .file-table th, .file-table td {{ text-align: left; padding: 12px; border-bottom: 1px solid #ddd; }}
            .file-table th {{ background-color: #4CAF50; color: white; }}
            .file-table tr:hover {{ background-color: #f5f5f5; }}
            .empty {{ padding: 20px; text-align: center; color: #757575; }}
            .download-btn {{ 
                display: inline-block;
                padding: 6px 12px;
                background-color: #2196F3;
                color: white;
                text-decoration: none;
                border-radius: 4px;
            }}
            .download-btn:hover {{ background-color: #0b7dda; }}
            .back-btn {{ 
                display: inline-block;
                margin-top: 20px;
                padding: 8px 16px;
                color: #2196F3;
                text-decoration: none;
                background-color: #e3f2fd;
                border-radius: 4px;
            }}
            .back-btn:hover {{ background-color: #bbdefb; }}
        </style>
    </head>
    <body>
        <div class="user-info">
            <p>您正在查看用户 {} 的文件</p>
        </div>
        
        <h1>用户文件列表</h1>
        
        {}
        
        {}
    </body>
    </html>
    "#, target_username, target_username, files_html, back_link);
    
    // 修复为 into_response
    Html(html_content).into_response()
}

// 文件下载处理程序
pub async fn download_file(
    Extension(user): Extension<User>,
    Path((target_username, filename)): Path<(String, String)>,
) -> impl IntoResponse {
    // 检查权限：只能下载自己的或者管理员可以下载所有人的
    if user.username != target_username && !matches!(user.role, UserRole::Admin) {
        // 修复为 into_response
        return Html("<script>alert('您没有权限下载此文件'); window.location.href='/';</script>".to_string()).into_response();
    }
    
    // 安全检查：防止路径穿越攻击
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        // 修复为 into_response
        return Html("<script>alert('非法的文件名'); window.location.href='/';</script>".to_string()).into_response();
    }
    
    // 文件路径
    let file_path = format!("uploads/{}/{}", target_username, filename);
    
    // 读取文件内容
    match tokio::fs::read(&file_path).await {
        Ok(content) => {
            // 设置 Content-Disposition 头，使浏览器下载文件而不是显示
            let content_disposition_headers = [(
                axum::http::header::CONTENT_DISPOSITION, 
                format!("attachment; filename=\"{}\"", filename)
            )];
            
            // 修复为 into_response
            (content_disposition_headers, content).into_response()
        },
        Err(e) => {
            tracing::error!("Failed to read file {}: {}", file_path, e);
            // 修复为 into_response
            Html("<script>alert('文件不存在或无法读取'); history.back();</script>".to_string()).into_response()
        }
    }
}