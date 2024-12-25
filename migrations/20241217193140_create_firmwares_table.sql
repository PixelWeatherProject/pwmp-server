-- Add migration script here
CREATE TABLE
    firmwares (
        id SMALLSERIAL PRIMARY KEY,
        version_major SMALLINT NOT NULL CHECK (version_major >= 0),
        version_middle SMALLINT NOT NULL CHECK (version_middle >= 0),
        version_minor SMALLINT NOT NULL CHECK (version_minor >= 0),
        firmware BYTEA NOT NULL CHECK (length(firmware) > 0),
        added_date TIMESTAMP UNIQUE NOT NULL DEFAULT NOW (),
        restrict_nodes SMALLINT[]
    );