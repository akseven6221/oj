use crate::database::UploadRepo;
use crate::models::{AppState, User, UserRole, UploadRecord};
use crate::templates::{index_template, uploads_template};
use axum::{
    extract::{Extension, Multipart, State},
    response::{Html, IntoResponse},
};
use sqlx::Row;
use std::{io::Write, path::PathBuf};

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
                        // 创建测试记录并添加到队列
                        match crate::database::TestRepo::create_test(&state.db_pool, user.id).await {
                            Ok(test_id) => {
                                // 添加到��试队列
                                let task = crate::models::TestTask {
                                    id: test_id,
                                    user_id: user.id,
                                    username: user.username.clone(),
                                    work_dir: format!("uploads/{}/{}", user.username, extract_dir_name),
                                };
                                
                                state.test_queue.add_task(task).await;
                                
                                return Html(format!(
                                    r#"<script>alert('文件上传成功！已加入测试队列，请稍后查看测试结果。'); window.location.href='/test_results';</script>"#
                                )).into_response();
                            }
                            Err(e) => {
                                tracing::error!("Failed to create test record: {}", e);
                                return Html(format!(
                                    r#"<script>alert('文件上传成功，但创建测试记录失败: {}'); window.location.href='/';</script>"#,
                                    e
                                )).into_response();
                            }
                        }
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

// 解压ZIP文件并替换user目录的函数
fn extract_zip_and_replace_user_dir(zip_path: &PathBuf, username: &str, extract_dir_name: &str) -> Result<(), String> {
    // 创建解压时目录，改为使用传入的目录名
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
        // 将项目中的user目录复制到解压后的目录中
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