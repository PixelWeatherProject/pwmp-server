INSERT INTO
    measurements ("node", "temperature", "humidity", "air_pressure", "measurement", "battery", "wifi_ssid", "wifi_rssi")
VALUES
    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) RETURNING id;