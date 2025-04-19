use crate::database::TestRepo;
use crate::models::{AppState, User, UserRole}; // 移除 TestStatus, TestResult
// Import new template functions and alert_redirect_template
use crate::templates::{alert_redirect_template, test_results_list_template, build_test_results_content_html, test_results_detail_template};
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
            // 构建测试结果列表内容的 HTML - 使用模板函数
            let results_content_html = build_test_results_content_html(&results);
            // 返回测试结果列表页面 - 使用模板函数
            Html(test_results_list_template(&results_content_html)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get test results: {}", e);
            // 使用模板
            Html(alert_redirect_template("获取测试结果失败", "/")).into_response()
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
                // 使用模板
                return Html(alert_redirect_template("您没有权限查看此测试结果", "/test_results")).into_response();
            }

            // 构建测试结果详情页面 - 使用模板函数
            Html(test_results_detail_template(&result)).into_response()
        }
        Ok(None) => {
            // 使用模板
            Html(alert_redirect_template("测试结果不存在", "/test_results")).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get test result: {}", e);
            // 使用模板
            Html(alert_redirect_template("获取测试结果失败", "/test_results")).into_response()
        }
    }
}