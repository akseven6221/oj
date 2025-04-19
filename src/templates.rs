use crate::models::{User, UserRole, UploadRecord, TestResult, TestStatus}; // Add TestResult, TestStatus
use std::fs;
use std::path::Path;

// HTML 模板文件路径
const LOGIN_TEMPLATE_PATH: &str = "templates/login.html";
const INDEX_TEMPLATE_PATH: &str = "templates/index.html";
const ADMIN_PANEL_TEMPLATE_PATH: &str = "templates/admin_panel.html";
const UPLOADS_TEMPLATE_PATH: &str = "templates/uploads.html"; // 新增上传模板路径
const ADMIN_PANEL_USER_ROW_TEMPLATE_PATH: &str = "templates/admin_panel_user_row.html"; // 新增用户行模板路径
const MESSAGE_TEMPLATE_PATH: &str = "templates/message.html";
const UPLOADS_EMPTY_TEMPLATE_PATH: &str = "templates/uploads_empty.html";
const INDEX_ADMIN_LINK_TEMPLATE_PATH: &str = "templates/index_admin_link.html";
const LOGIN_ERROR_TEMPLATE_PATH: &str = "templates/login_error.html";
const INDEX_ERROR_TEMPLATE_PATH: &str = "templates/index_error.html";
const ADMIN_PANEL_ERROR_TEMPLATE_PATH: &str = "templates/admin_panel_error.html";
const UPLOADS_ERROR_TEMPLATE_PATH: &str = "templates/uploads_error.html";
const ALERT_REDIRECT_TEMPLATE_PATH: &str = "templates/alert_redirect.html"; // 新增
const FILES_LIST_TEMPLATE_PATH: &str = "templates/files_list.html"; // 新增
const FILES_LIST_ROW_TEMPLATE_PATH: &str = "templates/files_list_row.html"; // 新增
const FILES_LIST_EMPTY_TEMPLATE_PATH: &str = "templates/files_list_empty.html"; // 新增
const UPLOAD_PAGE_TEMPLATE_PATH: &str = "templates/upload_page.html"; // 新增
const TEST_RESULTS_LIST_TEMPLATE_PATH: &str = "templates/test_results_list.html"; // 新增
const TEST_RESULTS_LIST_ROW_TEMPLATE_PATH: &str = "templates/test_results_list_row.html"; // 新增
const TEST_RESULTS_LIST_EMPTY_TEMPLATE_PATH: &str = "templates/test_results_list_empty.html"; // 新增
const TEST_RESULTS_DETAIL_TEMPLATE_PATH: &str = "templates/test_results_detail.html"; // 新增
const UPLOADS_TABLE_PATH: &str = "templates/uploads_table.html"; // 新增
const FILES_LIST_TABLE_PATH: &str = "templates/files_list_table.html"; // 新增
const TEST_RESULTS_LIST_TABLE_PATH: &str = "templates/test_results_list_table.html"; // 新增

// 确保模板目录存在
pub fn ensure_templates_exist() -> std::io::Result<()> {
    let templates_dir = Path::new("templates");
    if !templates_dir.exists() {
        fs::create_dir_all(templates_dir)?;
    }

    // 检查并创建各个模板文件
    if !Path::new(LOGIN_TEMPLATE_PATH).exists() {
        fs::write(LOGIN_TEMPLATE_PATH, include_str!("../templates/login.html"))?;
    }
    if !Path::new(INDEX_TEMPLATE_PATH).exists() {
        fs::write(INDEX_TEMPLATE_PATH, include_str!("../templates/index.html"))?;
    }
    if !Path::new(ADMIN_PANEL_TEMPLATE_PATH).exists() {
        fs::write(ADMIN_PANEL_TEMPLATE_PATH, include_str!("../templates/admin_panel.html"))?;
    }
    if !Path::new(UPLOADS_TEMPLATE_PATH).exists() {
        fs::write(UPLOADS_TEMPLATE_PATH, include_str!("../templates/uploads.html"))?;
    }
    if !Path::new(ADMIN_PANEL_USER_ROW_TEMPLATE_PATH).exists() {
        fs::write(ADMIN_PANEL_USER_ROW_TEMPLATE_PATH, include_str!("../templates/admin_panel_user_row.html"))?;
    }
    if !Path::new(MESSAGE_TEMPLATE_PATH).exists() {
        fs::write(MESSAGE_TEMPLATE_PATH, include_str!("../templates/message.html"))?;
    }
    if !Path::new(UPLOADS_EMPTY_TEMPLATE_PATH).exists() {
        fs::write(UPLOADS_EMPTY_TEMPLATE_PATH, include_str!("../templates/uploads_empty.html"))?;
    }
    if !Path::new(INDEX_ADMIN_LINK_TEMPLATE_PATH).exists() {
        fs::write(INDEX_ADMIN_LINK_TEMPLATE_PATH, include_str!("../templates/index_admin_link.html"))?;
    }
    if !Path::new(LOGIN_ERROR_TEMPLATE_PATH).exists() {
        fs::write(LOGIN_ERROR_TEMPLATE_PATH, include_str!("../templates/login_error.html"))?;
    }
    if !Path::new(INDEX_ERROR_TEMPLATE_PATH).exists() {
        fs::write(INDEX_ERROR_TEMPLATE_PATH, include_str!("../templates/index_error.html"))?;
    }
    if !Path::new(ADMIN_PANEL_ERROR_TEMPLATE_PATH).exists() {
        fs::write(ADMIN_PANEL_ERROR_TEMPLATE_PATH, include_str!("../templates/admin_panel_error.html"))?;
    }
    if !Path::new(UPLOADS_ERROR_TEMPLATE_PATH).exists() {
        fs::write(UPLOADS_ERROR_TEMPLATE_PATH, include_str!("../templates/uploads_error.html"))?;
    }
    if !Path::new(ALERT_REDIRECT_TEMPLATE_PATH).exists() {
        fs::write(ALERT_REDIRECT_TEMPLATE_PATH, include_str!("../templates/alert_redirect.html"))?;
    }
    if !Path::new(FILES_LIST_TEMPLATE_PATH).exists() {
        fs::write(FILES_LIST_TEMPLATE_PATH, include_str!("../templates/files_list.html"))?;
    }
    if !Path::new(FILES_LIST_ROW_TEMPLATE_PATH).exists() {
        fs::write(FILES_LIST_ROW_TEMPLATE_PATH, include_str!("../templates/files_list_row.html"))?;
    }
    if !Path::new(FILES_LIST_EMPTY_TEMPLATE_PATH).exists() {
        fs::write(FILES_LIST_EMPTY_TEMPLATE_PATH, include_str!("../templates/files_list_empty.html"))?;
    }
    if !Path::new(UPLOAD_PAGE_TEMPLATE_PATH).exists() {
        fs::write(UPLOAD_PAGE_TEMPLATE_PATH, include_str!("../templates/upload_page.html"))?;
    }
    if !Path::new(TEST_RESULTS_LIST_TEMPLATE_PATH).exists() {
        fs::write(TEST_RESULTS_LIST_TEMPLATE_PATH, include_str!("../templates/test_results_list.html"))?;
    }
    if !Path::new(TEST_RESULTS_LIST_ROW_TEMPLATE_PATH).exists() {
        fs::write(TEST_RESULTS_LIST_ROW_TEMPLATE_PATH, include_str!("../templates/test_results_list_row.html"))?;
    }
    if !Path::new(TEST_RESULTS_LIST_EMPTY_TEMPLATE_PATH).exists() {
        fs::write(TEST_RESULTS_LIST_EMPTY_TEMPLATE_PATH, include_str!("../templates/test_results_list_empty.html"))?;
    }
    if !Path::new(TEST_RESULTS_DETAIL_TEMPLATE_PATH).exists() {
        fs::write(TEST_RESULTS_DETAIL_TEMPLATE_PATH, include_str!("../templates/test_results_detail.html"))?;
    }
    // ... add checks for new templates like uploads_table.html and uploads_table_row.html if needed ...

    Ok(())
}

// 登录页面模板
pub fn login_template() -> String {
    read_template(LOGIN_TEMPLATE_PATH).unwrap_or_else(|e| {
        tracing::error!("无法读取登录模板文件: {}", e);
        read_template(LOGIN_ERROR_TEMPLATE_PATH).unwrap_or_else(|_| "Login template error".to_string())
    })
}

// 首页模板
pub fn index_template(username: &str, role: &UserRole) -> String {
    let role_text = match role {
        UserRole::Admin => "管理员",
        UserRole::Regular => "普通用户",
    };

    // 管理员入口 - 从模板读取
    let admin_panel_link = if matches!(role, UserRole::Admin) {
        read_template(INDEX_ADMIN_LINK_TEMPLATE_PATH).unwrap_or_else(|e| {
            tracing::warn!("无法读取首页管理员链接模板: {}", e);
            "".to_string() // Fallback to empty string
        })
    } else {
        "".to_string()
    };

    read_template(INDEX_TEMPLATE_PATH)
        .map(|template| {
            template
                .replace("{{username}}", username)
                .replace("{{role}}", role_text)
                .replace("{{admin_panel}}", &admin_panel_link)
        })
        .unwrap_or_else(|e| {
            tracing::error!("无法读取首页模板文件: {}", e);
            // 使用错误模板
            read_template(INDEX_ERROR_TEMPLATE_PATH)
                .map(|err_template| {
                    err_template
                        .replace("{{username}}", username)
                        .replace("{{role}}", role_text)
                })
                .unwrap_or_else(|_| format!("Index template error for {}", username)) // Fallback
        })
}

// 管理面板模板
pub fn admin_panel_template(users: &[User], error_message: Option<&str>, success_message: Option<&str>) -> String {
    // 构建用户列表 HTML
    let users_html = match build_users_html(users) {
        Ok(html) => html,
        Err(e) => {
            tracing::error!("构建用户列表 HTML 时出错: {}", e);
            "<p>加载用户列表时出错</p>".to_string()
        }
    };

    // 构建消息 HTML - 使用通用消息模板
    let error_html = error_message.map_or("".to_string(), |msg| {
        render_message_template("error-message", msg)
    });
    let success_html = success_message.map_or("".to_string(), |msg| {
        render_message_template("success-message", msg)
    });

    // 加载模板并填充变量
    read_template(ADMIN_PANEL_TEMPLATE_PATH)
        .map(|template| {
            template
                .replace("{{users_html}}", &users_html)
                .replace("{{error_html}}", &error_html)
                .replace("{{success_html}}", &success_html)
        })
        .unwrap_or_else(|e| {
            tracing::error!("无法读取管理面板模板文件: {}", e);
            // 使用错误模板
            read_template(ADMIN_PANEL_ERROR_TEMPLATE_PATH)
                .map(|err_template| {
                    err_template
                        .replace("{{error_html}}", &error_html)
                        .replace("{{success_html}}", &success_html)
                        .replace("{{users_html}}", &users_html)
                })
                .unwrap_or_else(|_| "Admin panel template error".to_string()) // Fallback
        })
}

// 构建用户列表 HTML - 修改为读取模板文件
fn build_users_html(users: &[User]) -> Result<String, std::io::Error> {
    let row_template = read_template(ADMIN_PANEL_USER_ROW_TEMPLATE_PATH)?;

    let rows_html = users
        .iter()
        .map(|user| {
            let role_text = match user.role {
                UserRole::Admin => "管理员",
                UserRole::Regular => "普通用户",
            };

            let is_admin_selected = if matches!(user.role, UserRole::Admin) { "selected" } else { "" };
            let is_regular_selected = if matches!(user.role, UserRole::Regular) { "selected" } else { "" };

            row_template
                .replace("{{username}}", &user.username)
                .replace("{{role_text}}", role_text)
                .replace("{{is_admin_selected}}", is_admin_selected)
                .replace("{{is_regular_selected}}", is_regular_selected)
        })
        .collect::<Vec<String>>()
        .join("\n");

    Ok(rows_html)
}

// 上传记录模板 - 修改为读取模板文件
pub fn uploads_template(user: &User, uploads: &[UploadRecord]) -> String {
    let uploads_table_html = if uploads.is_empty() {
        // 使用空模板
        read_template(UPLOADS_EMPTY_TEMPLATE_PATH).unwrap_or_else(|e| {
            tracing::warn!("无法读取上传空模板: {}", e);
            "<p>暂无上传记录</p>".to_string() // Fallback
        })
    } else {
        // 读取行模板
        match read_template("templates/uploads_table_row.html") { // Assuming uploads_table_row.html exists
            Ok(row_template) => {
                let rows_html = uploads.iter().map(|upload| {
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

                    row_template
                        .replace("{{filename}}", &upload.filename)
                        .replace("{{size}}", &size_display)
                        .replace("{{time}}", &time_display)
                        .replace("{{username_cell}}", &username_cell) // Assuming {{username_cell}} in row template
                }).collect::<Vec<String>>().join("\n");

                // 读取表格框架模板
                read_template(UPLOADS_TABLE_PATH)
                    .map(|table_template| {
                        table_template
                            .replace("{{admin_header}}", if matches!(user.role, UserRole::Admin) { "<th>用户</th>" } else { "" })
                            .replace("{{rows_html}}", &rows_html)
                    })
                    .unwrap_or_else(|e| {
                        tracing::error!("无法读取上传表格模板: {}", e);
                        "<p>加载上传列表时出错</p>".to_string()
                    })
            }
            Err(e) => {
                tracing::error!("无法读取上传行模板: {}", e);
                "<p>加载上传列表时出错</p>".to_string()
            }
        }
    };

    read_template(UPLOADS_TEMPLATE_PATH)
        .map(|template| {
            template
                .replace("{{username}}", &user.username)
                .replace("{{role}}", if matches!(user.role, UserRole::Admin) { "管理员" } else { "普通用户" })
                .replace("{{uploads_table}}", &uploads_table_html)
        })
        .unwrap_or_else(|e| {
            tracing::error!("无法读取上传记录模板文件 {}: {}", UPLOADS_TEMPLATE_PATH, e);
            // 使用错误模板
            read_template(UPLOADS_ERROR_TEMPLATE_PATH)
                .map(|err_template| {
                    err_template
                        .replace("{{username}}", &user.username)
                        .replace("{{uploads_table}}", &uploads_table_html)
                })
                .unwrap_or_else(|_| format!("Uploads template error for {}", user.username)) // Fallback
        })
}

// 新增：渲染 Alert Redirect 模板
pub fn alert_redirect_template(message: &str, redirect_url: &str) -> String {
    read_template(ALERT_REDIRECT_TEMPLATE_PATH)
        .map(|template| {
            template
                .replace("{{message}}", message)
                .replace("{{redirect_url}}", redirect_url)
        })
        .unwrap_or_else(|e| {
            tracing::error!("无法读取 Alert Redirect 模板文件: {}", e);
            // Fallback to basic script if template fails
            format!("<script>alert('{}'); window.location.href='{}';</script>", message, redirect_url)
        })
}

// 新增：文件列表模板
pub fn files_list_template(target_username: &str, files_content_html: &str) -> String {
    read_template(FILES_LIST_TEMPLATE_PATH)
        .map(|template| {
            template
                .replace("{{target_username}}", target_username)
                .replace("{{files_content}}", files_content_html)
        })
        .unwrap_or_else(|e| {
            tracing::error!("无法读取文件列表模板文件: {}", e);
            // Fallback to a simple error message
            format!("Error loading file list template for {}", target_username)
        })
}

// 新增：构建文件列表内容的 HTML
pub fn build_files_list_content_html(
    entries: &[(String, u64, Option<u64>, bool)],
    target_username: &str,
) -> String {
    if entries.is_empty() {
        read_template(FILES_LIST_EMPTY_TEMPLATE_PATH).unwrap_or_else(|e| {
            tracing::warn!("无法读取文件列表空模板: {}", e);
            "<p>此用户没有上传任何文件</p>".to_string() // Fallback
        })
    } else {
        match read_template(FILES_LIST_ROW_TEMPLATE_PATH) {
            Ok(row_template) => {
                let rows_html = entries
                    .iter()
                    .map(|(filename, size, modified, is_dir)| {
                        let size_str = if *size < 1024 {
                            format!("{} B", size)
                        } else if *size < 1024 * 1024 {
                            format!("{:.2} KB", *size as f64 / 1024.0)
                        } else {
                            format!("{:.2} MB", *size as f64 / (1024.0 * 1024.0))
                        };

                        let time_str = match modified {
                            Some(timestamp) => {
                                let dt = chrono::DateTime::from_timestamp(*timestamp as i64, 0)
                                    .unwrap_or_else(|| chrono::Utc::now());
                                dt.format("%Y-%m-%d %H:%M:%S").to_string()
                            }
                            None => "未知".to_owned(),
                        };

                        let type_str = if *is_dir { "目录" } else { "文件" };
                        let download_url = format!("/files/{}/{}", target_username, filename);
                        let action_cell = if *is_dir {
                            format!(r#"<a href="{}" class="view-btn">查看目录</a>"#, download_url)
                        } else {
                            format!(r#"<a href="{}" class="download-btn">下载</a>"#, download_url)
                        };

                        row_template
                            .replace("{{filename}}", filename)
                            .replace("{{type_str}}", type_str)
                            .replace("{{size_str}}", &size_str)
                            .replace("{{time_str}}", &time_str)
                            .replace("{{action_cell}}", &action_cell)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                // 读取表格框架模板
                read_template(FILES_LIST_TABLE_PATH)
                    .map(|table_template| table_template.replace("{{rows_html}}", &rows_html))
                    .unwrap_or_else(|e| {
                        tracing::error!("无法读取文件列表表格模板: {}", e);
                        "<p>加载文件列表时出错</p>".to_string()
                    })
            }
            Err(e) => {
                tracing::error!("无法读取文件列表行模板: {}", e);
                "<p>加载文件列表时出错</p>".to_string()
            }
        }
    }
}

// 新增：上传页面模板
pub fn upload_page_template() -> String {
    read_template(UPLOAD_PAGE_TEMPLATE_PATH).unwrap_or_else(|e| {
        tracing::error!("无法读取上传页面模板文件: {}", e);
        // Fallback to a simple error message
        "Error loading upload page template".to_string()
    })
}

// 新增：测试结果列表模板
pub fn test_results_list_template(results_content_html: &str) -> String {
    read_template(TEST_RESULTS_LIST_TEMPLATE_PATH)
        .map(|template| template.replace("{{results_content}}", results_content_html))
        .unwrap_or_else(|e| {
            tracing::error!("无法读取测试结果列表模板文件: {}", e);
            "Error loading test results list template".to_string()
        })
}

// 新增：构建测试结果列表内容的 HTML
pub fn build_test_results_content_html(results: &[TestResult]) -> String {
    if results.is_empty() {
        read_template(TEST_RESULTS_LIST_EMPTY_TEMPLATE_PATH).unwrap_or_else(|e| {
            tracing::warn!("无法读取测试结果列表空模板: {}", e);
            "<p>暂无测试结果</p>".to_string() // Fallback
        })
    } else {
        match read_template(TEST_RESULTS_LIST_ROW_TEMPLATE_PATH) {
            Ok(row_template) => {
                let rows_html = results
                    .iter()
                    .map(|result| {
                        let status_class = match result.status {
                            TestStatus::Pending => "status-pending",
                            TestStatus::Running => "status-running",
                            TestStatus::Passed => "status-passed",
                            TestStatus::Failed => "status-failed",
                            TestStatus::Error => "status-error",
                        };
                        let status_text = format!("{:?}", result.status);
                        let created_at_str = result.created_at.format("%Y-%m-%d %H:%M:%S").to_string();

                        row_template
                            .replace("{{id}}", &result.id.to_string())
                            .replace("{{username}}", &result.username)
                            .replace("{{status_class}}", status_class)
                            .replace("{{status}}", &status_text)
                            .replace("{{created_at}}", &created_at_str)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                // 读取表格框架模板
                read_template(TEST_RESULTS_LIST_TABLE_PATH)
                    .map(|table_template| table_template.replace("{{rows_html}}", &rows_html))
                    .unwrap_or_else(|e| {
                        tracing::error!("无法读取测试结果列表表格模板: {}", e);
                        "<p>加载测试结果列表时出错</p>".to_string()
                    })
            }
            Err(e) => {
                tracing::error!("无法读取测试结果列表行模板: {}", e);
                "<p>加载测试结果列表时出错</p>".to_string()
            }
        }
    }
}

// 新增：测试结果详情模板
pub fn test_results_detail_template(result: &TestResult) -> String {
    let status_class = match result.status {
        TestStatus::Pending => "status-pending",
        TestStatus::Running => "status-running",
        TestStatus::Passed => "status-passed",
        TestStatus::Failed => "status-failed",
        TestStatus::Error => "status-error",
    };
    let status_text = format!("{:?}", result.status);
    let created_at_str = result.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let updated_at_str = result.updated_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let output_text = result.output.as_deref().unwrap_or("无输出");
    let error_section_html = result.error.as_ref().map_or(String::new(), |err| {
        format!("<h2>错误</h2><div class=\"error\">{}</div>", err)
    });

    read_template(TEST_RESULTS_DETAIL_TEMPLATE_PATH)
        .map(|template| {
            template
                .replace("{{id}}", &result.id.to_string())
                .replace("{{username}}", &result.username)
                .replace("{{status_class}}", status_class)
                .replace("{{status}}", &status_text)
                .replace("{{created_at}}", &created_at_str)
                .replace("{{updated_at}}", &updated_at_str)
                .replace("{{output}}", output_text)
                .replace("{{error_section}}", &error_section_html)
        })
        .unwrap_or_else(|e| {
            tracing::error!("无法读取测试结果详情模板文件: {}", e);
            format!("Error loading test result detail template for ID {}", result.id)
        })
}

// 辅助函数：读取模板文件内容
fn read_template(path: &str) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}

// 辅助函数：渲染消息模板
fn render_message_template(message_class: &str, message_text: &str) -> String {
    read_template(MESSAGE_TEMPLATE_PATH)
        .map(|template| {
            template
                .replace("{{message_class}}", message_class)
                .replace("{{message_text}}", message_text)
        })
        .unwrap_or_else(|e| {
            tracing::warn!("无法读取消息模板: {}", e);
            // Fallback to simple div
            format!("<div class=\"{}\">{}</div>", message_class, message_text)
        })
}