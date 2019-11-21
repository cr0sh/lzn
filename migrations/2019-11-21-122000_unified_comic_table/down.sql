CREATE TABLE lezhin (
    comic TEXT NOT NULL,
    episode_seq INTEGER NOT NULL,
    episode TEXT,
    picture_seq INTEGER NOT NULL,
    picture BLOB,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(comic, episode_seq, picture_seq)
);

INSERT INTO lezhin
    SELECT comic_id, episode_seq, episode_name, image_seq, image, updated_at
        FROM comics;

DROP TABLE comics;