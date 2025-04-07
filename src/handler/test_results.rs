use crate::database::TestRepo;
use crate::models::{AppState, User, UserRole, TestStatus};
use axum::{
    extract::{Extension, State, Path},
    response::{Html, IntoResponse},
};

// 查看所有测试结果
pub async fn view_results(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 根据用户角色获取测试结果
    let results = if matches!(user.role, UserRole::Admin) {
        // 管理员可以查看所有测试结果
        TestRepo::get_all_tests(&state.db_pool).await
    } else {
        // 普通用户只能查看自己的测试结果
        TestRepo::get_user_tests(&state.db_pool, user.id).await
    };
    
    match results {
        Ok(results) => {
            // 构建测试结果列表页面
            let html = format!(
                r#"<!DOCTYPE html>
                <html>
                <head>
                    <title>测试结果</title>
                    <style>
                        body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
                        h1 {{ color: #333; }}
                        .result-table {{ width: 100%; border-collapse: collapse; margin-top: 20px; }}
                        .result-table th, .result-table td {{ text-align: left; padding: 12px; border-bottom: 1px solid #ddd; }}
                        .result-table th {{ background-color: #4CAF50; color: white; }}
                        .result-table tr:hover {{ background-color: #f5f5f5; }}
                        .status-pending {{ color: #ff9800; }}
                        .status-running {{ color: #2196F3; }}
                        .status-passed {{ color: #4CAF50; }}
                        .status-failed {{ color: #f44336; }}
                        .status-error {{ color: #9c27b0; }}
                        .empty {{ padding: 20px; text-align: center; color: #757575; }}
                        .back {{ display: inline-block; margin-top: 20px; color: #2196F3; text-decoration: none; }}
                        .view-btn {{ display: inline-block; padding: 5px 10px; background-color: #2196F3; color: white; text-decoration: none; border-radius: 3px; }}
                        .view-btn:hover {{ background-color: #0b7dda; }}
                    </style>
                </head>
                <body>
                    <h1>测试结果</h1>
                    
                    {}
                    
                    <a href="/" class="back">返回主页</a>
                </body>
                </html>"#,
                if results.is_empty() {
                    r#"<div class="empty">暂无测试结果</div>"#.to_string()
                } else {
                    let mut html = String::from(r#"<table class="result-table">
                        <thead>
                            <tr>
                                <th>ID</th>
                                <th>用户</th>
                                <th>状态</th>
                                <th>提交时间</th>
                                <th>操作</th>
                            </tr>
                        </thead>
                        <tbody>"#);
                        
                    for result in &results {
                        let status_class = match result.status {
                            TestStatus::Pending => "status-pending",
                            TestStatus::Running => "status-running",
                            TestStatus::Passed => "status-passed",
                            TestStatus::Failed => "status-failed",
                            TestStatus::Error => "status-error",
                        };
                        
                        html.push_str(&format!(
                            r#"<tr>
                                <td>{}</td>
                                <td>{}</td>
                                <td class="{}">{:?}</td>
                                <td>{}</td>
                                <td><a href="/test_results/{}" class="view-btn">查看详情</a></td>
                            </tr>"#,
                            result.id, result.username, status_class, result.status,
                            result.created_at.format("%Y-%m-%d %H:%M:%S"),
                            result.id
                        ));
                    }
                    
                    html.push_str("</tbody></table>");
                    html
                }
            );
            
            Html(html).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get test results: {}", e);
            Html("<script>alert('获取测试结果失败'); window.location.href='/';</script>".to_string()).into_response()
        }
    }
}

// 查看单个测试结果详情
pub async fn view_result_detail(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match TestRepo::get_test_by_id(&state.db_pool, id).await {
        Ok(Some(result)) => {
            // 检查权限：只能查看自己的结果或者管理员可以查看所有结果
            if result.user_id != user.id && !matches!(user.role, UserRole::Admin) {
                return Html("<script>alert('您没��权限查看此测试结果'); window.location.href='/test_results';</script>".to_string()).into_response();
            }
            
            // 构建测试结果详情页面
            let status_class = match result.status {
                TestStatus::Pending => "status-pending",
                TestStatus::Running => "status-running",
                TestStatus::Passed => "status-passed",
                TestStatus::Failed => "status-failed",
                TestStatus::Error => "status-error",
            };
            
            let html = format!(
                r#"<!DOCTYPE html>
                <html>
                <head>
                    <title>测试结果详情 #{}</title>
                    <style>
                        body {{ font-family: Arial, sans-serif; max-width: 1000px; margin: 0 auto; padding: 20px; }}
                        h1 {{ color: #333; }}
                        .info {{ margin: 20px 0; }}
                        .info p {{ margin: 10px 0; }}
                        .status-pending {{ color: #ff9800; }}
                        .status-running {{ color: #2196F3; }}
                        .status-passed {{ color: #4CAF50; }}
                        .status-failed {{ color: #f44336; }}
                        .status-error {{ color: #9c27b0; }}
                        .output {{ background: #f5f5f5; padding: 15px; border-radius: 5px; white-space: pre-wrap; overflow-x: auto; max-height: 500px; overflow-y: auto; }}
                        .error {{ background: #ffebee; padding: 15px; border-radius: 5px; white-space: pre-wrap; overflow-x: auto; }}
                        .back {{ display: inline-block; margin-top: 20px; color: #2196F3; text-decoration: none; }}
                    </style>
                </head>
                <body>
                    <h1>测试结果详情 #{}</h1>
                    
                    <div class="info">
                        <p><strong>用户:</strong> {}</p>
                        <p><strong>状态:</strong> <span class="{}">{:?}</span></p>
                        <p><strong>提交时间:</strong> {}</p>
                        <p><strong>更新时间:</strong> {}</p>
                    </div>
                    
                    <h2>输出</h2>
                    <div class="output">{}</div>
                    
                    {}
                    
                    <a href="/test_results" class="back">返回测试结果列表</a>
                </body>
                </html>"#,
                result.id, result.id, result.username, status_class, result.status,
                result.created_at.format("%Y-%m-%d %H:%M:%S"),
                result.updated_at.format("%Y-%m-%d %H:%M:%S"),
                result.output.unwrap_or_else(|| "无输出".to_string()),
                if let Some(error) = result.error {
                    format!("<h2>错误</h2><div class=\"error\">{}</div>", error)
                } else {
                    String::new()
                }
            );
            
            Html(html).into_response()
        }
        Ok(None) => {
            Html("<script>alert('测试结果不存在'); window.location.href='/test_results';</script>".to_string()).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get test result: {}", e);
            Html("<script>alert('获取测试结果失败'); window.location.href='/test_results';</script>".to_string()).into_response()
        }
    }
}