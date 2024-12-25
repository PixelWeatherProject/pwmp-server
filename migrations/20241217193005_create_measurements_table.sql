-- Add migration script here
CREATE TABLE
    measurements (
        id SERIAL PRIMARY KEY,
        node INT2 NOT NULL REFERENCES devices (id),
        "when" TIMESTAMP UNIQUE NOT NULL DEFAULT NOW (),
        temperature DECIMAL(4, 2) NOT NULL,
        humidity SMALLINT NOT NULL CHECK (
            humidity >= 0
            AND humidity <= 100
        ),
        air_pressure SMALLINT DEFAULT NULL
    );