SELECT u.id, u.filename, u.file_path, u.file_size, u.uploaded_at, us.username
FROM uploads u
JOIN users us ON u.user_id = us.id
ORDER BY u.uploaded_at DESC