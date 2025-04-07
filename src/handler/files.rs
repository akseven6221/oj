use crate::database::UserRepo;
use crate::models::{AppState, User, UserRole};
use axum::{
    extract::{Extension, Path, State},
    response::{Html, IntoResponse, Redirect},
};

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
                    let dt = chrono::DateTime::from_timestamp(timestamp as i64, 0)
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
    State(state): State<AppState>,
    Path((username, filename)): Path<(String, String)>,
) -> impl IntoResponse {
    // 检查权限：只能下载自己的或者管理员可以下载所有人的
    if user.username != username && !matches!(user.role, UserRole::Admin) {
        return Html("<script>alert('您没有权限下载此文件'); window.location.href='/';</script>".to_string()).into_response();
    }
    
    // 获取目标用户信息
    let _target_user = match UserRepo::get_user_by_username(&state.db_pool, &username).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html("<script>alert('用户不存在'); window.location.href='/';</script>".to_string()).into_response(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Html("<script>alert('数据库错误'); window.location.href='/';</script>".to_string()).into_response();
        }
    };
    
    // 构建文件路径
    let file_path = format!("uploads/{}/{}", username, filename);
    
    // 检查文件是否存在
    match tokio::fs::metadata(&file_path).await {
        Ok(metadata) => {
            if metadata.is_dir() {
                // 如果是目录，重定向到目录列表
                Redirect::to(&format!("/files/{}/{}", username, filename)).into_response()
            } else {
                // 如果是文件，提供下载
                match tokio::fs::read(&file_path).await {
                    Ok(content) => {
                        // 设置正确的Content-Type和Content-Disposition头
                        let filename_encoded = urlencoding::encode(&filename);
                        let headers = [
                            (axum::http::header::CONTENT_TYPE, "application/octet-stream"),
                            (axum::http::header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename_encoded)),
                        ];
                        
                        (headers, content).into_response()
                    }
                    Err(e) => {
                        tracing::error!("Failed to read file: {}", e);
                        Html("<script>alert('读取文件失败'); window.location.href='/';</script>".to_string()).into_response()
                    }
                }
            }
        }
        Err(_) => {
            Html("<script>alert('文件不存在'); window.location.href='/';</script>".to_string()).into_response()
        }
    }
}

// 上传文件页面处理函数
pub async fn upload_page(
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    let html = format!(
        r#"<!DOCTYPE html>
        <html>
        <head>
            <title>上传文件</title>
            <style>
                body {{ font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
                h1 {{ color: #333; }}
                .upload-form {{ background-color: #f9f9f9; padding: 20px; border-radius: 5px; }}
                .form-group {{ margin-bottom: 15px; }}
                label {{ display: block; margin-bottom: 5px; font-weight: bold; }}
                input[type="file"] {{ width: 100%; padding: 10px; box-sizing: border-box; }}
                button {{ background-color: #4CAF50; color: white; padding: 10px 15px; border: none; border-radius: 3px; cursor: pointer; }}
                button:hover {{ background-color: #45a049; }}
                .back {{ display: inline-block; margin-top: 20px; color: #2196F3; text-decoration: none; }}
            </style>
        </head>
        <body>
            <h1>上传文件</h1>
            
            <form class="upload-form" action="/upload" method="post" enctype="multipart/form-data">
                <div class="form-group">
                    <label for="file">选择文件：</label>
                    <input type="file" id="file" name="file" required>
                </div>
                <button type="submit">上传</button>
            </form>
            
            <a href="/" class="back">返回主页</a>
        </body>
        </html>"#
    );
    
    Html(html).into_response()
}