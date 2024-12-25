UPDATE
  firmware_stats
SET
  success = $2
WHERE
  id = $1;