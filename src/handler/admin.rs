use crate::database::UserRepo;
use crate::models::{AppState, User, UserRole, UserCreateForm, UserUpdateForm};
use crate::templates::admin_panel_template;
use axum::{
    extract::{Extension, Form, Path, State},
    response::{Html, IntoResponse},
};

// 管理员面板
pub async fn admin_panel(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // 检查是否是管理员
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
    // 检查是否是管理员
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
    // 检查是否是管理员
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
    // 检查是否是管理员
    if !matches!(user.role, UserRole::Admin) {
        return Html(
            "<script>alert('只有管理员才能删除用户'); window.location.href='/';</script>".to_string(),
        ).into_response();
    }
    
    // 不能删除自己
    if user.username == username {
        match UserRepo::get_all_users(&state.db_pool).await {
            Ok(users) => return Html(admin_panel_template(&users, Some("不能删除当前登录的用户"), None)).into_response(),
            Err(_) => {
                return Html("<script>alert('不能删除当前登录的用户'); window.location.href='/admin/users';</script>".to_string()).into_response()
            }
        }
    }
    
    // 不能删除主管理员
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