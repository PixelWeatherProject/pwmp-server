-- 
-- TABLES
-- 

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
        "when" TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW (),
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

-- 
-- HELPER FUNCTIONS
-- 

-- Calculates the dew point based on the temperature in Celsius and relative humidity percentage.
CREATE OR REPLACE FUNCTION pwmp_calc_dew_point(temp_c REAL, humidity SMALLINT)
RETURNS REAL AS $$
DECLARE
    alpha REAL;
    dew_point REAL;
BEGIN
    -- Prevent log of zero if humidity sensor glitches and reads 0
    IF humidity <= 0 THEN
        RETURN NULL; 
    END IF;

    -- Calculate the intermediate alpha value
    alpha := ((17.27 * temp_c) / (237.3 + temp_c)) + LN(humidity / 100.0);
    
    -- Calculate final dew point
    dew_point := (237.3 * alpha) / (17.27 - alpha);
    
    RETURN dew_point;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Categorizes the dew point into human comfort levels based on standard thresholds.
CREATE OR REPLACE FUNCTION pwmp_categorize_dew_point(dew_point REAL)
RETURNS VARCHAR AS $$
BEGIN
    IF dew_point IS NULL THEN
        RETURN NULL;
    ELSIF dew_point < 10 THEN
        RETURN 'Dry';
    ELSIF dew_point <= 15 THEN
        RETURN 'Comfortable';
    ELSIF dew_point <= 18 THEN
        RETURN 'Humid';
    ELSIF dew_point <= 21 THEN
        RETURN 'Muggy';
    ELSIF dew_point <= 24 THEN
        RETURN 'Oppressive';
    ELSE
        RETURN 'Dangerous';
    END IF;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Calculates the sea level pressure from the absolute air pressure, temperature in Celsius, and altitude in meters using the barometric formula.
CREATE OR REPLACE FUNCTION pwmp_calc_sea_level_pressure(abs_air_pressure SMALLINT, temp_c REAL, altitude_m REAL)
RETURNS REAL AS $$
DECLARE
    sea_level_pressure REAL;
BEGIN
    -- Protect against absolute zero math errors
    IF temp_c < -273 THEN
        RETURN NULL; 
    END IF;

    -- The Barometric Formula
    sea_level_pressure := abs_air_pressure * POWER(1.0 - (0.0065 * altitude_m) / (temp_c + (0.0065 * altitude_m) + 273.15), -5.257);

    RETURN sea_level_pressure;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Calculates the time difference between a Node's first and last entry.
CREATE OR REPLACE FUNCTION pwmp_get_node_total_runtime(target_node INT4)
RETURNS TABLE (
    earliest_time TIMESTAMP,
    latest_time TIMESTAMP,
    diff_interval INTERVAL
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        MIN("when"), 
        MAX("when"), 
        age(MAX("when"), MIN("when"))
    FROM measurements
    WHERE "node" = target_node;
END;
$$ LANGUAGE plpgsql;