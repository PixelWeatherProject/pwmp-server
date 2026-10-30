DELETE FROM measurements;

DELETE FROM settings;

DELETE FROM notifications;

DELETE FROM diagnostic_artifacts;

DELETE FROM firmware_stats;

DELETE FROM firmwares;

DELETE FROM devices;

DELETE FROM sqlite_sequence
WHERE
    name IN (
        'measurements',
        'settings',
        'notifications',
        'diagnostic_artifacts',
        'firmware_stats',
        'firmwares',
        'devices'
    );