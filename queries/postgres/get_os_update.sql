SELECT
  f.version_major,
  f.version_middle,
  f.version_minor,
  f.firmware
FROM
  firmwares f
WHERE
  (
    f.restrict_nodes IS NULL
    OR $1 = ANY (f.restrict_nodes)
  )
  AND (
    (f.version_major > $2)
    OR (
      f.version_major = $2
      AND f.version_middle > $3
    )
    OR (
      f.version_major = $2
      AND f.version_middle = $3
      AND f.version_minor > $4
    )
  )
  AND NOT EXISTS (
    SELECT
      1
    FROM
      firmware_stats fs
    WHERE
      fs.node = $1
      AND fs.to_version_major = f.version_major
      AND fs.to_version_middle = f.version_middle
      AND fs.to_version_minor = f.version_minor
  );