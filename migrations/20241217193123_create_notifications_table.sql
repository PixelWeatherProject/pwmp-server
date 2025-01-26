-- Add migration script here
CREATE TABLE
    notifications (
        id SMALLSERIAL PRIMARY KEY,
        node INT2 NOT NULL REFERENCES devices (id),
        "when" TIMESTAMP NOT NULL DEFAULT NOW (),
        content VARCHAR(64) NOT NULL,
        read BOOLEAN NOT NULL DEFAULT FALSE
    );