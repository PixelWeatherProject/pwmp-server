SELECT devices.id
FROM devices
WHERE mac_address = $1;