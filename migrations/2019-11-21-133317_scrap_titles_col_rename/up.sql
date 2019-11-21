CREATE TABLE scraping_targets_rename (
    provider TEXT NOT NULL,
    id TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    last_scraping TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(provider, id COLLATE NOCASE)
);

INSERT INTO scraping_targets_rename
    SELECT * FROM scraping_targets;

DROP TABLE scraping_targets;
ALTER TABLE scraping_targets_rename
    RENAME TO scraping_targets;