SELECT
  version_major,
  version_middle,
  version_minor,
  firmware
FROM
  firmwares
WHERE
  (
    restrict_nodes IS NULL
    OR $1 = ANY (restrict_nodes)
  )
  AND (
(version_major > $2)
    OR (
      version_major = $2
      AND version_middle > $3
    )
    OR (
      version_major = $2
      AND version_middle = $3
      AND version_minor > $4
    )
  );