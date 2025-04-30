-- Add migration script here
CREATE TABLE
    measurements (
        id SERIAL PRIMARY KEY,
        node INT4 NOT NULL REFERENCES devices (id),
        "when" TIMESTAMP NOT NULL DEFAULT NOW (),
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