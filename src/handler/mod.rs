mod admin;
mod auth;
mod files;
mod upload;

// 重新导出所有公开函数
pub use admin::{admin_panel, create_user, delete_user, update_user};
pub use auth::{login_handler, login_page, logout_handler};
pub use files::{download_file, view_user_files}; // 添加 view_user_files
pub use upload::{index_handler, upload_handler, view_uploads};