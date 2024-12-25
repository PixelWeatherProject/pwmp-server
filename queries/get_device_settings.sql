SELECT battery_ignore, ota, sleep_time, sbop, mute_notifications
FROM settings
WHERE node = $1;