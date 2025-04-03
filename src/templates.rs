use crate::models::{User, UserRole, UploadRecord};
use std::fs;
use std::path::Path;

// HTML 模板文件路径
const LOGIN_TEMPLATE_PATH: &str = "templates/login.html";
const INDEX_TEMPLATE_PATH: &str = "templates/index.html";
const ADMIN_PANEL_TEMPLATE_PATH: &str = "templates/admin_panel.html";

// 确保模板目录存在
pub fn ensure_templates_exist() -> std::io::Result<()> {
    // 创建模板目录
    fs::create_dir_all("templates")?;
    
    // 检查是否需要创建登录模板
    if !Path::new(LOGIN_TEMPLATE_PATH).exists() {
        // 如果模板不存在，就发出警告
        tracing::warn!("登录模板文件 {} 不存在，请创建此文件", LOGIN_TEMPLATE_PATH);
    }
    
    // 检查是否需要创建首页模板
    if !Path::new(INDEX_TEMPLATE_PATH).exists() {
        tracing::warn!("首页模板文件 {} 不存在，请创建此文件", INDEX_TEMPLATE_PATH);
    }
    
    // 检查是否需要创建管理面板模板
    if !Path::new(ADMIN_PANEL_TEMPLATE_PATH).exists() {
        tracing::warn!("管理面板模板文件 {} 不存在，请创建此文件", ADMIN_PANEL_TEMPLATE_PATH);
    }
    
    Ok(())
}

// 登录页面模板
pub fn login_template() -> String {
    match fs::read_to_string(LOGIN_TEMPLATE_PATH) {
        Ok(template) => template,
        Err(e) => {
            tracing::error!("无法读取登录模板文件: {}", e);
            r#"<html><body><h1>错误</h1><p>无法加载登录模板</p></body></html>"#.to_string()
        }
    }
}

// 首页模板
pub fn index_template(username: &str, role: &UserRole) -> String {
    let role_text = match role {
        UserRole::Admin => "管理员",
        UserRole::Regular => "普通用户",
    };
    
    // 管理员入口
    let admin_panel_link = if matches!(role, UserRole::Admin) {
        r#"<a href="/admin/users" class="action-btn">用户管理</a>"#
    } else {
        ""
    };
    
    match fs::read_to_string(INDEX_TEMPLATE_PATH) {
        Ok(template) => template
            .replace("{{username}}", username)
            .replace("{{role}}", role_text)
            .replace("{{admin_panel}}", admin_panel_link),
        Err(e) => {
            tracing::error!("无法读取首页模板文件: {}", e);
            format!(
                r#"<html><body><h1>错误</h1><p>无法加载首页模板</p><p>用户: {} ({})</p></body></html>"#,
                username, role_text
            )
        }
    }
}

// 管理面板模板
pub fn admin_panel_template(users: &[User], error_message: Option<&str>, success_message: Option<&str>) -> String {
    // 构建用户列表 HTML
    let users_html = build_users_html(users);
    
    // 构建错误消息 HTML
    let error_html = if let Some(msg) = error_message {
        format!(r#"<div class="error-message">{}</div>"#, msg)
    } else {
        "".to_string()
    };
    
    // 构建成功消息 HTML
    let success_html = if let Some(msg) = success_message {
        format!(r#"<div class="success-message">{}</div>"#, msg)
    } else {
        "".to_string()
    };
    
    // 加载模板并填充变量
    match fs::read_to_string(ADMIN_PANEL_TEMPLATE_PATH) {
        Ok(template) => template
            .replace("{{users_html}}", &users_html)
            .replace("{{error_html}}", &error_html)
            .replace("{{success_html}}", &success_html),
        Err(e) => {
            tracing::error!("无法读取管理面板模板文件: {}", e);
            format!(
                r#"<html><body><h1>错误</h1><p>无法加载管理面板模板</p>
                <div>{}</div><div>{}</div><div>{}</div></body></html>"#,
                error_html, success_html, users_html
            )
        }
    }
}

// 构建用户列表 HTML
fn build_users_html(users: &[User]) -> String {
    users
        .iter()
        .map(|user| {
            let role_text = match user.role {
                UserRole::Admin => "管理员",
                UserRole::Regular => "普通用户",
            };
            
            let is_admin_selected = if matches!(user.role, UserRole::Admin) { "selected" } else { "" };
            let is_regular_selected = if matches!(user.role, UserRole::Regular) { "selected" } else { "" };
            
            format!(
                r#"<tr>
                    <td>{}</td>
                    <td>{}</td>
                    <td>
                        <form action="/admin/users/{}/update" method="post" class="inline-form">
                            <select name="role">
                                <option value="admin" {}>管理员</option>
                                <option value="regular" {}>普通用户</option>
                            </select>
                            <input type="password" name="password" placeholder="新密码（留空不修改）">
                            <button type="submit" class="small-button">更新</button>
                        </form>
                        <a href="/files/{}" class="small-button file-btn">查看文件</a>
                    </td>
                    <td>
                        <form action="/admin/users/{}/delete" method="post" class="inline-form"
                              onsubmit="return confirm('确定要删除用户 {} 吗？');">
                            <button type="submit" class="small-button danger">删除</button>
                        </form>
                    </td>
                </tr>"#,
                user.username,
                role_text,
                user.username,
                is_admin_selected,
                is_regular_selected,
                user.username, // 新增：��户文件链接
                user.username,
                user.username
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

// 上传记录模板
pub fn uploads_template(user: &User, uploads: &[UploadRecord]) -> String {
    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>上传记录</title>
            <style>
                body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
                .user-info {{ background: #f5f5f5; padding: 10px; margin-bottom: 20px; border-radius: 5px; }}
                table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
                th, td {{ text-align: left; padding: 12px; border-bottom: 1px solid #ddd; }}
                th {{ background-color: #4CAF50; color: white; }}
                tr:hover {{ background-color: #f5f5f5; }}
                .back {{ display: inline-block; margin-top: 20px; color: #2196F3; text-decoration: none; }}
                .empty {{ padding: 20px; text-align: center; color: #757575; }}
            </style>
        </head>
        <body>
            <div class="user-info">
                <p>欢迎，{} ({})</p>
            </div>
            
            <h1>上传记录</h1>
            
            {}
            
            <a href="/" class="back">返回主页</a>
        </body>
        </html>
        "#,
        user.username,
        if matches!(user.role, UserRole::Admin) { "管理员" } else { "普通用户" },
        if uploads.is_empty() {
            r#"<div class="empty">暂无上传记录</div>"#.to_string()
        } else {
            format!(
                r#"
                <table>
                    <thead>
                        <tr>
                            <th>文件名</th>
                            <th>大小</th>
                            <th>上传时间</th>
                            {}
                        </tr>
                    </thead>
                    <tbody>
                        {}
                    </tbody>
                </table>
                "#,
                if matches!(user.role, UserRole::Admin) { "<th>���户</th>" } else { "" },
                uploads.iter().map(|upload| {
                    let size_display = if upload.file_size < 1024 {
                        format!("{} B", upload.file_size)
                    } else if upload.file_size < 1024 * 1024 {
                        format!("{:.2} KB", upload.file_size as f64 / 1024.0)
                    } else {
                        format!("{:.2} MB", upload.file_size as f64 / (1024.0 * 1024.0))
                    };
                    
                    let time_display = upload.uploaded_at.format("%Y-%m-%d %H:%M:%S").to_string();
                    
                    let username_cell = if matches!(user.role, UserRole::Admin) {
                        format!("<td>{}</td>", upload.username.as_deref().unwrap_or("unknown"))
                    } else {
                        "".to_string()
                    };
                    
                    format!(
                        r#"<tr>
                            <td>{}</td>
                            <td>{}</td>
                            <td>{}</td>
                            {}
                        </tr>"#,
                        upload.filename,
                        size_display,
                        time_display,
                        username_cell
                    )
                }).collect::<Vec<String>>().join("\n")
            )
        }
    );
    
    html_content
}