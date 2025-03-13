-- Add migration script here
CREATE TABLE
    statistics (
        id SERIAL PRIMARY KEY,
        measurement INT4 NOT NULL REFERENCES measurements (id),
        battery DECIMAL(3, 2) NOT NULL CHECK (battery > 0),
        wifi_ssid VARCHAR(32) NOT NULL,
        wifi_rssi INT2 NOT NULL
    );