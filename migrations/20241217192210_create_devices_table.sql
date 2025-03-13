-- Add migration script here
CREATE TABLE devices (
    id SERIAL PRIMARY KEY,
    mac_address VARCHAR(17) UNIQUE NOT NULL CHECK (mac_address ~ E'^([0-9A-F]{2}:){5}[0-9A-F]{2}$'),
    location POINT DEFAULT NULL,
    note VARCHAR(1024) DEFAULT NULL
);