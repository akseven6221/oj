sudo mysql -u root -p

-- 创建数据库
CREATE DATABASE oj_db;

-- 创建用户并授权
CREATE USER 'oj_user'@'localhost' IDENTIFIED BY 'yourpassword';
GRANT ALL PRIVILEGES ON oj_db.* TO 'oj_user'@'localhost';
FLUSH PRIVILEGES;

-- 退出 MySQL
EXIT;