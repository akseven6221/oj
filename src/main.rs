mod auth;
mod database;
mod handler;
mod models;
mod templates;
mod tester; // 新模块

use auth::auth_middleware;
use database::init_db;
use handler::{
    admin_panel, create_user, delete_user, download_file, index_handler, login_handler, login_page,
    logout_handler, update_user, upload_handler, view_uploads, view_user_files,
    view_results, view_result_detail, // 新的处理函数
};
use models::AppState;
use tester::TestQueue;
use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, post},
    Router,
    response::{Html, IntoResponse, Json},
};

use tower_cookies::{Cookie, CookieManagerLayer, Cookies};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// 添加一个测试处理程序 - 修复迭代方法
async fn test_cookies(cookies: Cookies) -> Json<Vec<String>> {
    let cookie_list: Vec<String> = cookies
        .list()
        .iter()
        .map(|c| format!("{}: {}", c.name(), c.value()))
        .collect();
    
    Json(cookie_list)
}

// 添加一个简单的 Cookie 设置处理程序
async fn set_test_cookie(cookies: Cookies) -> impl IntoResponse {
    let cookie = Cookie::new("test_cookie", "test_value");
    cookies.add(cookie);
    Html("<p>Cookie set! <a href='/check-cookie'>Check Cookie</a></p>".to_string())
}

// 添加一个 Cookie 检查处理程序
async fn check_test_cookie(cookies: Cookies) -> impl IntoResponse {
    if let Some(cookie) = cookies.get("test_cookie") {
        Html(format!("<p>Cookie found: {} = {}</p>", cookie.name(), cookie.value()))
    } else {
        Html("<p>No cookie found!</p>".to_string())
    }
}

#[tokio::main]
async fn main() {
    // 加载环境变量
    dotenv::dotenv().ok();
    
    // 设置调试级别日志
    std::env::set_var("RUST_LOG", "debug");
    
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 只创建上传根目录，不再创建admin和regular子目录
    tokio::fs::create_dir_all("uploads").await.unwrap();
    tokio::fs::create_dir_all("templates").await.unwrap();
    
    // 确保模板目录存在
    templates::ensure_templates_exist().unwrap();
    
    // 确保user目录存在
    tokio::fs::create_dir_all("user").await.unwrap();
    
    // 检查模板文件是否存在，如果不存在则创建默认模板
    check_and_create_template_files().await;

    // 初始化数据库
    let db_pool = match init_db().await {
        Ok(pool) => {
            tracing::info!("数据库连接成功");
            pool
        }
        Err(e) => {
            tracing::error!("数据库连接失败: {}", e);
            std::process::exit(1);
        }
    };

    // 初始化测试队列
    let test_queue = Arc::new(TestQueue::new(Arc::new(db_pool.clone())));
    
    // 启动测试工作器
    let worker_queue = test_queue.clone();
    tokio::spawn(async move {
        worker_queue.start_worker().await;
    });

    // 初始化应用状态
    let state = AppState::new(db_pool, test_queue);

    // 创建需要认证的路由
    let protected_routes = Router::new()
        .route("/", get(index_handler))
        .route("/upload", post(upload_handler))
        .route("/uploads", get(view_uploads))
        .route("/files/:username", get(view_user_files))
        .route("/files/:username/:filename", get(download_file))
        // 添加测试结果路由
        .route("/test_results", get(view_results))
        .route("/test_results/:id", get(view_result_detail))
        // 添加管理员路由
        .route("/admin/users", get(admin_panel))
        .route("/admin/users/create", post(create_user))
        .route("/admin/users/:username/update", post(update_user))
        .route("/admin/users/:username/delete", post(delete_user))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware
        ));

    // 创建不需要认证的路由
    let public_routes = Router::new()
        .route("/login", get(login_page).post(login_handler))
        .route("/logout", get(logout_handler))  // 这是退出登录的路由
        .route("/test-cookies", get(test_cookies))
        .route("/set-cookie", get(set_test_cookie))
        .route("/check-cookie", get(check_test_cookie));
        
    // 合并路由并添加全局中间件
    let app = protected_routes
        .merge(public_routes)
        .layer(CookieManagerLayer::new())
        .with_state(state);

    // 绑定到 localhost 而不是 0.0.0.0
    let addr = "127.0.0.1:3000";
    tracing::info!("服务器已启动：");
    tracing::info!("- 本地访问：http://localhost:3000");

    if let Ok(hostname) = std::env::var("HOSTNAME") {
        tracing::info!("- 远程访问：http://{}:3000", hostname);
    } else {
        tracing::info!("- 远程访问：使用服务器IP地址:3000");
    }
    
    // 开始监听
    axum::Server::bind(&addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// 检查并创建默认模板文件
async fn check_and_create_template_files() {
    let templates = [
        ("templates/login.html", include_str!("../templates/login.html")),
        ("templates/index.html", include_str!("../templates/index.html")),
        ("templates/admin_panel.html", include_str!("../templates/admin_panel.html")),
    ];

    for (path, content) in templates {
        if !std::path::Path::new(path).exists() {
            tracing::info!("创建默认模板文件: {}", path);
            if let Err(e) = tokio::fs::write(path, content).await {
                tracing::error!("无法创建模板文件 {}: {}", path, e);
            }
        }
    }
}
