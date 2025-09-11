-- Add migration script here
CREATE TABLE artists (
    id uuid PRIMARY KEY,
    sync BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE releases (
    id uuid PRIMARY KEY,
    sync BOOLEAN NOT NULL DEFAULT FALSE
);
