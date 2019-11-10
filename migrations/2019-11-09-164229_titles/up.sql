CREATE TABLE titles (
    provider TEXT NOT NULL,
    id TEXT NOT NULL,
    title TEXT,
    PRIMARY KEY(provider, id)
);