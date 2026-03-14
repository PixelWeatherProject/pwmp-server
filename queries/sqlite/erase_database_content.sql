DELETE FROM statistics;

DELETE FROM measurements;

DELETE FROM settings;

DELETE FROM notifications;

DELETE FROM firmware_stats;

DELETE FROM firmwares;

DELETE FROM devices;

DELETE FROM sqlite_sequence
WHERE
    name IN (
        'statistics',
        'measurements',
        'settings',
        'notifications',
        'firmware_stats',
        'firmwares',
        'devices'
    );