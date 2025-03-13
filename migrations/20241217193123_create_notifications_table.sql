-- Add migration script here
CREATE TABLE
    notifications (
        id SERIAL PRIMARY KEY,
        node INT4 NOT NULL REFERENCES devices (id),
        "when" TIMESTAMP NOT NULL DEFAULT NOW (),
        content VARCHAR(1024) NOT NULL,
        read BOOLEAN NOT NULL DEFAULT FALSE
    );