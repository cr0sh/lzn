CREATE TABLE comics (
    provider TEXT NOT NULL,
    comic_id TEXT NOT NULL,
    episode_seq INTEGER NOT NULL,
    episode_name TEXT,
    image_seq INTEGER NOT NULL,
    image BLOB NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(provider, comic_id, episode_seq, image_seq)
);

INSERT INTO comics
    SELECT 'lezhin', * FROM lezhin
        WHERE lezhin.picture IS NOT NULL;

DROP TABLE lezhin;