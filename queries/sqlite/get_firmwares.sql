SELECT
    id,
    version_major || '.' || version_middle || '.' || version_minor AS version,
    length (firmware),
    firmware,
    strftime ('%d.%m.%Y %H:%M:%S', added_date) AS added_date,
    restrict_nodes
FROM
    firmwares