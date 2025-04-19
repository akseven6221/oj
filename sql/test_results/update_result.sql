UPDATE test_results
SET status = ?, output = ?, error = ?, updated_at = CURRENT_TIMESTAMP
WHERE id = ?