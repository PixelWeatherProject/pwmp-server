UPDATE
  firmware_stats
SET
  success = $2
WHERE
  id = (
    SELECT
      id
    FROM
      firmware_stats
    WHERE
      success IS NULL
      AND node = $1
    ORDER BY
      "when" DESC
    LIMIT
      1
  );
