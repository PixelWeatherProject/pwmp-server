INSERT INTO
    firmware_stats (
        node,
        from_version_major,
        from_version_middle,
        from_version_minor,
        to_version_major,
        to_version_middle,
        to_version_minor
    )
VALUES
    ($1, $2, $3, $4, $5, $6, $7)
RETURNING id;