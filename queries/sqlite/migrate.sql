CREATE TABLE
    devices (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mac_address TEXT UNIQUE NOT NULL,
        location TEXT DEFAULT NULL,
        note TEXT DEFAULT NULL
    ) STRICT;

CREATE TABLE
    measurements (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        node INTEGER NOT NULL REFERENCES devices (id),
        "when" TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        temperature REAL NOT NULL CHECK (
            temperature > -100.00
            AND temperature < 100.00
        ),
        humidity INTEGER NOT NULL CHECK (
            humidity >= 0
            AND humidity <= 100
        ),
        air_pressure INTEGER DEFAULT NULL
    ) STRICT;

CREATE TABLE
    statistics (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        measurement INTEGER NOT NULL REFERENCES measurements (id),
        battery REAL NOT NULL CHECK (
            battery > 0
            AND battery < 5.00
        ),
        wifi_ssid TEXT NOT NULL,
        wifi_rssi INTEGER NOT NULL
    ) STRICT;

CREATE TABLE
    settings (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        node INTEGER UNIQUE NOT NULL REFERENCES devices (id),
        battery_ignore INTEGER NOT NULL DEFAULT 0,
        ota INTEGER NOT NULL DEFAULT 0,
        sleep_time INTEGER NOT NULL DEFAULT 60 CHECK (sleep_time > 0),
        sbop INTEGER NOT NULL DEFAULT 1,
        mute_notifications INTEGER NOT NULL DEFAULT 0,
        device_specific TEXT NOT NULL DEFAULT '{}'
    ) STRICT;

CREATE TABLE
    notifications (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        node INTEGER NOT NULL REFERENCES devices (id),
        "when" TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        content TEXT NOT NULL,
        read INTEGER NOT NULL DEFAULT 0
    ) STRICT;

CREATE TABLE
    firmwares (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        version_major INTEGER NOT NULL CHECK (version_major >= 0),
        version_middle INTEGER NOT NULL CHECK (version_middle >= 0),
        version_minor INTEGER NOT NULL CHECK (version_minor >= 0),
        firmware BLOB NOT NULL CHECK (length (firmware) > 0),
        added_date TEXT UNIQUE NOT NULL DEFAULT CURRENT_TIMESTAMP,
        restrict_nodes TEXT DEFAULT NULL
    ) STRICT;

CREATE TABLE
    firmware_stats (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        node INTEGER NOT NULL REFERENCES devices (id),
        from_version_major INTEGER NOT NULL CHECK (from_version_major >= 0),
        from_version_middle INTEGER NOT NULL CHECK (from_version_middle >= 0),
        from_version_minor INTEGER NOT NULL CHECK (from_version_minor >= 0),
        to_version_major INTEGER NOT NULL CHECK (to_version_major >= 0),
        to_version_middle INTEGER NOT NULL CHECK (to_version_middle >= 0),
        to_version_minor INTEGER NOT NULL CHECK (to_version_minor >= 0),
        "when" TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        success INTEGER DEFAULT NULL
    ) STRICT;