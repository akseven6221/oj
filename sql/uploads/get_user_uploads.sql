SELECT id, filename, file_path, file_size, uploaded_at
FROM uploads
WHERE user_id = ?
ORDER BY uploaded_at DESC