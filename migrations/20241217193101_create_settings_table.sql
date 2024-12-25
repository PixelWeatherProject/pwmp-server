-- Add migration script here
CREATE TABLE settings (
    id SMALLSERIAL PRIMARY KEY,
    node INT2 UNIQUE NOT NULL REFERENCES devices(id),
    battery_ignore BOOLEAN NOT NULL DEFAULT FALSE,
    ota BOOLEAN NOT NULL DEFAULT FALSE,
    sleep_time INT2 NOT NULL DEFAULT 60 CHECK (sleep_time > 0),
    sbop BOOLEAN NOT NULL DEFAULT TRUE,
    mute_notifications BOOLEAN NOT NULL DEFAULT FALSE,
    device_specific JSON NOT NULL DEFAULT '{}'::json
);