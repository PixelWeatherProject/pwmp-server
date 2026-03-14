CREATE TABLE devices (
    id SERIAL PRIMARY KEY,
    mac_address VARCHAR(17) UNIQUE NOT NULL CHECK (mac_address ~ E'^([0-9A-F]{2}:){5}[0-9A-F]{2}$'),
    location POINT DEFAULT NULL,
    note VARCHAR(1024) DEFAULT NULL
);

CREATE TABLE
    measurements (
        id SERIAL PRIMARY KEY,
        node INT4 NOT NULL REFERENCES devices (id),
        "when" TIMESTAMP
        WITH
            TIME ZONE NOT NULL DEFAULT NOW (),
            temperature REAL NOT NULL CHECK (
                temperature > -100.00
                AND temperature < 100.00
            ),
            humidity SMALLINT NOT NULL CHECK (
                humidity >= 0
                AND humidity <= 100
            ),
            air_pressure SMALLINT DEFAULT NULL
    );

CREATE TABLE
    statistics (
        id SERIAL PRIMARY KEY,
        measurement INT4 NOT NULL REFERENCES measurements (id),
        battery REAL NOT NULL CHECK (
            battery > 0
            AND battery < 5.00
        ),
        wifi_ssid VARCHAR(32) NOT NULL,
        wifi_rssi INT2 NOT NULL
    );

CREATE TABLE settings (
    id SERIAL PRIMARY KEY,
    node INT4 UNIQUE NOT NULL REFERENCES devices(id),
    battery_ignore BOOLEAN NOT NULL DEFAULT FALSE,
    ota BOOLEAN NOT NULL DEFAULT FALSE,
    sleep_time INT2 NOT NULL DEFAULT 60 CHECK (sleep_time > 0),
    sbop BOOLEAN NOT NULL DEFAULT TRUE,
    mute_notifications BOOLEAN NOT NULL DEFAULT FALSE,
    device_specific JSON NOT NULL DEFAULT '{}'::json
);

CREATE TABLE
    notifications (
        id SERIAL PRIMARY KEY,
        node INT4 NOT NULL REFERENCES devices (id),
        "when" TIMESTAMP NOT NULL DEFAULT NOW (),
        content VARCHAR(1024) NOT NULL,
        read BOOLEAN NOT NULL DEFAULT FALSE
    );

CREATE TABLE
    firmwares (
        id SERIAL PRIMARY KEY,
        version_major SMALLINT NOT NULL CHECK (version_major >= 0),
        version_middle SMALLINT NOT NULL CHECK (version_middle >= 0),
        version_minor SMALLINT NOT NULL CHECK (version_minor >= 0),
        firmware BYTEA NOT NULL CHECK (length(firmware) > 0),
        added_date TIMESTAMP UNIQUE NOT NULL DEFAULT NOW (),
        restrict_nodes INT4[] DEFAULT NULL
    );

CREATE TABLE
    firmware_stats (
        id SERIAL PRIMARY KEY,
        node INT4 NOT NULL REFERENCES devices (id),
        from_version_major SMALLINT NOT NULL CHECK (from_version_major >= 0),
        from_version_middle SMALLINT NOT NULL CHECK (from_version_middle >= 0),
        from_version_minor SMALLINT NOT NULL CHECK (from_version_minor >= 0),
        to_version_major SMALLINT NOT NULL CHECK (to_version_major >= 0),
        to_version_middle SMALLINT NOT NULL CHECK (to_version_middle >= 0),
        to_version_minor SMALLINT NOT NULL CHECK (to_version_minor >= 0),
        "when" TIMESTAMP NOT NULL DEFAULT NOW (),
        success BOOLEAN DEFAULT NULL
    );