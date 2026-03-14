DELETE FROM statistics;

DELETE FROM measurements;

DELETE FROM notifications;

DELETE FROM firmware_stats;

DELETE FROM firmwares;

DELETE FROM sqlite_sequence
WHERE
    name IN (
        'statistics',
        'measurements',
        'notifications',
        'firmware_stats',
        'firmwares'
    );