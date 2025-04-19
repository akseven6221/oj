use crate::database::UserRepo;
use crate::models::{AppState, User, UserRole};
// Import new template functions and alert_redirect_template
use crate::templates::{alert_redirect_template, files_list_template, build_files_list_content_html, upload_page_template};
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
        return Html(alert_redirect_template("您没有权限查看此用户的文件", "/")).into_response();
    }
    
    // 获取目标用户信息，加上下划线前缀避免未使用警告
    let _target_user = match UserRepo::get_user_by_username(&state.db_pool, &target_username).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html(alert_redirect_template("用户不存在", "/")).into_response(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Html(alert_redirect_template("数据库错误", "/")).into_response();
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
            return Html(alert_redirect_template("无法访问用户目录", "/")).into_response();
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
            return Html(alert_redirect_template("读取用户目录失败", "/")).into_response();
        }
    }
    
    // 按文件名排序
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // 构建文件列表内容的 HTML - 使用模板函数
    let files_content_html = build_files_list_content_html(&entries, &target_username);

    // 返回文件列表页面 - 使用模板函数
    Html(files_list_template(&target_username, &files_content_html)).into_response()
}

// 下载文件处理函数
pub async fn download_file(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path((username, filename)): Path<(String, String)>,
) -> impl IntoResponse {
    // 检查权限：只能下载自己的或者管理员可以下载所有人的
    if user.username != username && !matches!(user.role, UserRole::Admin) {
        return Html(alert_redirect_template("您没有权限下载此文件", "/")).into_response();
    }
    
    // 获取目标用户信息
    let _target_user = match UserRepo::get_user_by_username(&state.db_pool, &username).await {
        Ok(Some(u)) => u,
        Ok(None) => return Html(alert_redirect_template("用户不存在", "/")).into_response(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            return Html(alert_redirect_template("数据库错误", "/")).into_response();
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
                        Html(alert_redirect_template("读取文件失败", "/")).into_response()
                    }
                }
            }
        }
        Err(_) => {
            Html(alert_redirect_template("文件不存在", "/")).into_response()
        }
    }
}

// 上传文件页面处理函数
pub async fn upload_page(
    Extension(_user): Extension<User>,
) -> impl IntoResponse {
    // 使用模板函数
    Html(upload_page_template()).into_response()
}