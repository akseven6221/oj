UPDATE test_results
SET status = ?, updated_at = CURRENT_TIMESTAMP
WHERE id = ?