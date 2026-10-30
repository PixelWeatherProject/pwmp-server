DELETE FROM measurements;

DELETE FROM notifications;

DELETE FROM diagnostic_artifacts;

DELETE FROM firmware_stats;

DELETE FROM firmwares;

DELETE FROM sqlite_sequence
WHERE
    name IN (
        'measurements',
        'notifications',
        'diagnostic_artifacts',
        'firmware_stats',
        'firmwares'
    );