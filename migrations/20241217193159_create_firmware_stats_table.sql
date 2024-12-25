-- Add migration script here
CREATE TABLE
    firmware_stats (
        id SERIAL PRIMARY KEY,
        node INT2 NOT NULL REFERENCES devices (id),
        from_version_major SMALLINT NOT NULL CHECK (from_version_major >= 0),
        from_version_middle SMALLINT NOT NULL CHECK (from_version_middle >= 0),
        from_version_minor SMALLINT NOT NULL CHECK (from_version_minor >= 0),
        to_version_major SMALLINT NOT NULL CHECK (to_version_major >= 0),
        to_version_middle SMALLINT NOT NULL CHECK (to_version_middle >= 0),
        to_version_minor SMALLINT NOT NULL CHECK (to_version_minor >= 0),
        "when" TIMESTAMP UNIQUE NOT NULL DEFAULT NOW (),
        success BOOLEAN DEFAULT NULL
    );