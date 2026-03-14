INSERT INTO measurements(
        "node",
        "temperature",
        "humidity",
        "air_pressure"
    )
VALUES ($1, $2, $3, $4)
RETURNING id;