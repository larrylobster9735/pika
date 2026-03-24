UPDATE branch_inbox_states
SET state = 'dismissed',
    reason = 'branch_closed',
    dismissed_at = COALESCE(dismissed_at, CURRENT_TIMESTAMP),
    updated_at = CURRENT_TIMESTAMP
WHERE state = 'inbox'
  AND branch_id IN (
      SELECT id
      FROM branch_records
      WHERE state != 'open'
  );
