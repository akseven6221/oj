use crate::database::{SessionRepo, UploadRepo, UserRepo};
use crate::models::{AppState, LoginForm, User, UserRole, UserCreateForm, UserUpdateForm, UploadRecord};
use crate::templates::{index_template, login_template, admin_panel_template, uploads_template};
use axum::{
    extract::{Extension, Form, Multipart, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use std::{io::Write, path::PathBuf};
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;
use sqlx::Row;

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
                
                // 在数据库中记录��话
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
    // 使用用户名创建用户专属文件夹
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
                                upload_info = Some((file_name.clone(), data.len() as i64, upload_path));
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
    
    // 如果文件上传成功且是zip文件，尝试解压并替换user目录
    if uploaded {
        if let Some((filename, size, upload_path)) = upload_info {
            // 检查是否为zip文件
            if filename.ends_with(".zip") {
                // 计算不带扩展名的文件名
                let file_stem = std::path::Path::new(&filename)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                
                // 创建解压目录名称：filename_out
                let extract_dir_name = format!("{}_out", file_stem);
                
                match extract_zip_and_replace_user_dir(&upload_path, &user.username, &extract_dir_name) {
                    Ok(_) => {
                        return Html(format!(
                            r#"<script>alert('文件 {} ({} 字节) 上传并解压成功！已保存到 {}'); window.location.href='/';</script>"#,
                            filename, size, extract_dir_name
                        )).into_response();
                    }
                    Err(e) => {
                        tracing::error!("解压缩失败: {}", e);
                        return Html(format!(
                            r#"<script>alert('文件上传成功，但解压失败: {}'); window.location.href='/';</script>"#,
                            e
                        )).into_response();
                    }
                }
            } else {
                return Html(format!(
                    r#"<script>alert('文件 {} ({} 字节) 上传成功！'); window.location.href='/';</script>"#,
                    filename, size
                )).into_response();
            }
        } else {
            return Html("<script>alert('上传成功！'); window.location.href='/';</script>".to_string()).into_response();
        }
    } else {
        return Html("<script>alert('上传失败！'); window.location.href='/';</script>".to_string()).into_response();
    }
}

// 解压ZIP文件并替换user目录的函数
fn extract_zip_and_replace_user_dir(zip_path: &PathBuf, username: &str, extract_dir_name: &str) -> Result<(), String> {
    // 创建解压临时目录，改为使用传入的目录名
    let extract_dir = format!("uploads/{}/{}", username, extract_dir_name);
    
    // 确保解压目录存在并为空
    if std::path::Path::new(&extract_dir).exists() {
        std::fs::remove_dir_all(&extract_dir).map_err(|e| format!("无法清理解压目录: {}", e))?;
    }
    std::fs::create_dir_all(&extract_dir).map_err(|e| format!("无法创建解压目录: {}", e))?;
    
    // 打开zip文件
    let file = std::fs::File::open(zip_path).map_err(|e| format!("无法打开ZIP文件: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("无法解析ZIP文件: {}", e))?;
    
    // 解压文件
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("解压错误: {}", e))?;
        let outpath = std::path::Path::new(&extract_dir).join(file.name());
        
        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath).map_err(|e| format!("无法创建目录 {}: {}", outpath.display(), e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).map_err(|e| format!("无法创建目录 {}: {}", p.display(), e))?;
                }
            }
            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| format!("无法创建文件 {}: {}", outpath.display(), e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("无法写入文件 {}: {}", outpath.display(), e))?;
        }
    }
    
    tracing::info!("解压完成: {}", extract_dir);
    
    // 查找并替换user目录
    let user_dir_in_zip = find_user_dir(&extract_dir)?;
    
    // 确保项目当前目录中的user目录存在
    let project_user_dir = "user";
    if !std::path::Path::new(project_user_dir).exists() {
        std::fs::create_dir_all(project_user_dir).map_err(|e| format!("无法创建项目user目录: {}", e))?;
    }
    
    // 替换解压后的user目录
    if user_dir_in_zip.exists() && std::path::Path::new(project_user_dir).exists() {
        // 将项目中的user目录复制到解压后���目录中
        copy_dir_recursively(project_user_dir, user_dir_in_zip.parent().unwrap())
            .map_err(|e| format!("无法替换user目录: {}", e))?;
        tracing::info!("已成功替换user目录");
    } else {
        tracing::warn!("在解压的文件中未找到user目录或项目user目录不存在");
    }
    
    Ok(())
}

// 查找解压目录中的user目录
fn find_user_dir(extract_dir: &str) -> Result<std::path::PathBuf, String> {
    // 首先尝试在根目录查找
    let direct_user_dir = std::path::Path::new(extract_dir).join("user");
    if direct_user_dir.exists() && direct_user_dir.is_dir() {
        return Ok(direct_user_dir);
    }
    
    // 尝试查找第一级子目录中的user目录
    let entries = std::fs::read_dir(extract_dir)
        .map_err(|e| format!("无法读取解压目录: {}", e))?;
    
    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                let potential_user_dir = path.join("user");
                if potential_user_dir.exists() && potential_user_dir.is_dir() {
                    return Ok(potential_user_dir);
                }
            }
        }
    }
    
    // 如果未找到，返回根目录下的user路径（即使它不存在）
    Ok(direct_user_dir)
}

// 递归复制目录
fn copy_dir_recursively(src: &str, dst: &std::path::Path) -> std::io::Result<()> {
    let src_path = std::path::Path::new(src);
    let dst_path = dst.join(src_path.file_name().unwrap());
    
    // 如果目标已存在，先删除
    if dst_path.exists() {
        std::fs::remove_dir_all(&dst_path)?;
    }
    
    // 创建目标目录
    std::fs::create_dir_all(&dst_path)?;
    
    // 遍历源目录中的所有条目
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let dst_file = dst_path.join(file_name);
        
        if entry_path.is_dir() {
            // 递归复制子目录
            copy_dir_recursively(entry_path.to_str().unwrap(), &dst_path)?;
        } else {
            // 复制文件
            std::fs::copy(&entry_path, &dst_file)?;
        }
    }
    
    Ok(())
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
    // 检查是否是管理员 - 注意！修复条件语法
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
    // 检查是否是管理员 - 注意！修复条件语法
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
    // 检查是否是管理员 - 注意！修复条件语法
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
    // 检查是否是管理员 - 注意！修复条件语法
    if !matches!(user.role, UserRole::Admin) {
        return Html(
            "<script>alert('只有管理员才能删除用户'); window.location.href='/';</script>".to_string(),
        ).into_response();
    }
    
    // 不能删除自己 - 注意！修复条件语法
    if user.username == username {
        match UserRepo::get_all_users(&state.db_pool).await {
            Ok(users) => return Html(admin_panel_template(&users, Some("不能删除当前登录的用户"), None)).into_response(),
            Err(_) => {
                return Html("<script>alert('不能删除当前登录的用户'); window.location.href='/admin/users';</script>".to_string()).into_response()
            }
        }
    }
    
    // 不能删除主管理员 - 注意！修复条件语法
    if username == "admin" {
        match UserRepo::get_all_users(&state.db_pool).await {
            Ok(users) => return Html(admin_panel_template(&users, Some("不能删除主管理员"), None)).into_response(),
            Err(_) => {
                return Html("<script>alert('不能删除主管理员'); window.location.href='/admin/users';</script>".to_string()).into_response()
            }
        }
    }
    
    // 删除用户
    match UserRepo::delete_user(&state.db_pool, &username).await {
        Ok(true) => {
            match UserRepo::get_all_users(&state.db_pool).await {
                Ok(users) => Html(admin_panel_template(&users, None, Some(&format!("用户 {} 删除成功！", username)))).into_response(),
                Err(e) => {
                    tracing::error!("Failed to get users: {}", e);
                    Html("<script>alert('用户删除成功，但获取用户列表失败！'); window.location.href='/admin/users';</script>".to_string()).into_response()
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
            tracing::error!("Failed to delete user: {}", e);
            
            match UserRepo::get_all_users(&state.db_pool).await {
                Ok(users) => Html(admin_panel_template(&users, Some(&format!("删除用户失败: {}", e)), None)).into_response(),
                Err(_) => {
                    Html(format!(
                        "<script>alert('删除用户失败: {}'); window.location.href='/admin/users';</script>",
                        e
                    )).into_response()
                }
            }
        }
    }
}

// 获取目录大小的辅助函数
fn get_dir_size(dir_path: &std::path::Path) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64, std::io::Error>> + Send + '_>> {
    Box::pin(async move {
        let mut total_size = 0;
        let mut read_dir = tokio::fs::read_dir(dir_path).await?;
        
        while let Some(entry) = read_dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            if metadata.is_file() {
                total_size += metadata.len();
            } else if metadata.is_dir() {
                total_size += get_dir_size(&entry.path()).await?;
            }
        }
        
        Ok(total_size)
    })
}

// 查看用户所有文件列表
#[axum::debug_handler]
pub async fn view_user_files(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(target_username): Path<String>,
) -> impl IntoResponse {
    // 检查权限：只能查看自己的或者管理员可以查看所有人的 - 注意！修复条件语法
    if user.username != target_username && !matches!(user.role, UserRole::Admin) {
        return Html("<script>alert('您没有权限查看此用户的文件'); window.location.href='/';</script>".to_string()).into_response();
    }
    
    // 获取目标用户信息，加上下划线前缀避免未使用警告
    let _target_user = match UserRepo::get_user_by_username(&state.db_pool, &target_username).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html("<script>alert('用户不存在'); window.location.href='/';</script>".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
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
            return Html("<script>alert('无法访问用户目录'); window.location.href='/';</script>".to_string()).into_response();
        }
    }
    
    // 读取目录内容
    match tokio::fs::read_dir(&user_dir).await {
        Ok(mut dir) => {
            while let Ok(Some(entry)) = dir.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(filename) = entry.file_name().into_string() {
                        if metadata.is_file() {
                            let size = metadata.len();
                            let modified = metadata.modified().ok().and_then(|time| {
                                time.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
                            });
                            
                            entries.push((filename, size, modified, false));
                        } else if metadata.is_dir() {
                            // 添加目录项
                            let dir_size = get_dir_size(&entry.path()).await.unwrap_or(0);
                            entries.push((filename, dir_size, None, true));
                        }
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to read user directory: {}", e);
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
                    <th>名称</th>
                    <th>类型</th>
                    <th>大小</th>
                    <th>修改时间</th>
                    <th>操作</th>
                </tr>
            </thead>
            <tbody>"#);
        
        for (filename, size, modified, is_dir) in entries {
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
                    let dt = chrono::NaiveDateTime::from_timestamp_opt(timestamp as i64, 0)
                        .map(|ndt| chrono::DateTime::<chrono::Utc>::from_utc(ndt, chrono::Utc))
                        .unwrap_or_else(|| chrono::Utc::now());
                    dt.format("%Y-%m-%d %H:%M:%S").to_string()
                }
                None => "未知".to_owned(),
            };
            
            let type_str = if is_dir { "目录" } else { "文件" };
            
            let download_url = format!("/files/{}/{}", target_username, filename);
            
            let action_cell = if is_dir {
                format!(r#"<a href="{}" class="view-btn">查看目录</a>"#, download_url)
            } else {
                format!(r#"<a href="{}" class="download-btn">下载</a>"#, download_url)
            };
            
            files_html.push_str(&format!(
                r#"<tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{}</td>
                </tr>"#,
                filename, type_str, size_str, time_str, action_cell
            ));
        }
        
        files_html.push_str("</tbody></table>");
    }
    
    // 返回文件列表页面
    let html = format!(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>文件列表 - {}</title>
            <style>
                body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
                h1 {{ color: #333; }}
                .file-table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
                .file-table th, .file-table td {{ text-align: left; padding: 12px; border-bottom: 1px solid #ddd; }}
                .file-table th {{ background-color: #4CAF50; color: white; }}
                .file-table tr:hover {{ background-color: #f5f5f5; }}
                .empty {{ padding: 20px; text-align: center; color: #757575; }}
                .back {{ display: inline-block; margin-top: 20px; color: #2196F3; text-decoration: none; }}
                .view-btn, .download-btn {{ display: inline-block; padding: 5px 10px; color: white; text-decoration: none; border-radius: 3px; }}
                .view-btn {{ background-color: #2196F3; }}
                .download-btn {{ background-color: #4CAF50; }}
            </style>
        </head>
        <body>
            <h1>{} 的文件列表</h1>
            
            {}
            
            <a href="/" class="back">返回主页</a>
        </body>
        </html>"#,
        target_username, target_username, files_html
    );
    
    Html(html).into_response()
}

// 下载文件处理函数
pub async fn download_file(
    Extension(user): Extension<User>,
    Path((target_username, filename)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 检查权限：只能下载自己的或者管理员可以下载所有人的
    if user.username != target_username && !matches!(user.role, UserRole::Admin) {
        return Html("<script>alert('您没有权限下载此文件'); window.location.href='/';</script>".to_string()).into_response();
    }
    
    // 获取目标用户信息
    let _target_user = match UserRepo::get_user_by_username(&state.db_pool, &target_username).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html("<script>alert('用户不存在'); window.location.href='/';</script>".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Html("<script>alert('数据库错误'); window.location.href='/';</script>".to_string()).into_response();
        }
    };
    
    // 构建文件路径
    let file_path = format!("uploads/{}/{}", target_username, filename);
    
    // 检查文件是否存在
    if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
        if metadata.is_dir() {
            // 如果是目录，重定向到目录列表
            return Redirect::to(&format!("/files/{}/{}", target_username, filename)).into_response();
        } else {
            // 如果是文件，提供下载
            match tokio::fs::read(&file_path).await {
                Ok(content) => {
                    // 添加必要的响应头
                    let filename_encoded = urlencoding::encode(&filename);
                    let headers = [
                        (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
                        (axum::http::header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename_encoded)),
                    ];
                    
                    // 返回文件内容
                    (headers, content).into_response()
                }
                Err(e) => {
                    tracing::error!("Failed to read file: {}", e);
                    Html("<script>alert('读取文件失败'); window.location.href='/';</script>".to_string()).into_response()
                }
            }
        }
    } else {
        Html("<script>alert('文件不存在'); window.location.href='/';</script>".to_string()).into_response()
    }
}