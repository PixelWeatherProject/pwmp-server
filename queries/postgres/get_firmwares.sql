SELECT
    id,
    concat_ws ('.', version_major, version_middle, version_minor) AS version,
    length (firmware),
    firmware,
    to_char (added_date, 'DD.MM.YYYY HH24:MI:SS') AS added_date,
    restrict_nodes
FROM
    firmwares