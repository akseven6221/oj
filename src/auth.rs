use crate::database::{SessionRepo, UserRepo};
use crate::models::AppState;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use tower_cookies::Cookies;

// 认证中间件
pub async fn auth_middleware<B>(
    State(state): State<AppState>,
    cookies: Cookies,
    mut request: Request<B>,
    next: Next<B>,
) -> Result<impl IntoResponse, StatusCode> {
    // 添加请求路径调试信息
    let path = request.uri().path();
    tracing::debug!("Processing request for path: {}", path);
    
    // 登录页面和静态资源不需要认证
    if path == "/login" || path.starts_with("/static/") {
        tracing::debug!("Skipping auth for path: {}", path);
        return Ok(next.run(request).await);
    }
    
    // 检查会话Cookie
    if let Some(session_cookie) = cookies.get("session_id") {
        let session_id = session_cookie.value().to_string();
        tracing::debug!("Found session_id cookie: {}", session_id);
        
        // 从数据库查询会话
        match SessionRepo::get_session(&state.db_pool, &session_id).await {
            Ok(Some(user_id)) => {
                // 从数据库查询用户
                match UserRepo::get_user_by_id(&state.db_pool, user_id).await {
                    Ok(Some(user)) => {
                        tracing::debug!("User authenticated: {}", user.username);
                        request.extensions_mut().insert(user);
                        return Ok(next.run(request).await);
                    }
                    Ok(None) => {
                        tracing::warn!("Session exists but user not found. User ID: {}", user_id);
                    }
                    Err(e) => {
                        tracing::error!("Database error while fetching user: {}", e);
                        return Err(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
            }
            Ok(None) => {
                tracing::debug!("Session ID not found in database");
            }
            Err(e) => {
                tracing::error!("Database error while fetching session: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        tracing::debug!("No session cookie found");
    }
    
    // 未认证，重定向到登录页面
    tracing::debug!("Redirecting to login page");
    Ok(Redirect::to("/login").into_response())
}