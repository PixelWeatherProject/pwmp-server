SELECT
    measurements.id,
    node,
    "when",
    temperature,
    humidity,
    air_pressure,
    battery,
    wifi_ssid,
    wifi_rssi
FROM
    measurements
    JOIN statistics ON statistics.measurement = measurements.id
WHERE
    node = $1