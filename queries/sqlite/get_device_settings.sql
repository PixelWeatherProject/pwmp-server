SELECT
    battery_ignore,
    ota,
    sleep_time,
    sbop,
    mute_notifications,
    diagnostics
FROM
    settings
WHERE
    node = ?1;