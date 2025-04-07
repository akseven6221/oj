use crate::database::{SessionRepo, UserRepo};
use crate::models::{AppState, LoginForm};
use crate::templates::login_template;
use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Redirect},
};
use tower_cookies::{Cookie, Cookies};
use uuid::Uuid;

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