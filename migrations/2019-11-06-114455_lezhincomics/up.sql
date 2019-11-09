CREATE TABLE lezhin (
    comic TEXT NOT NULL,
    episode_seq INTEGER NOT NULL,
    episode TEXT,
    picture_seq INTEGER NOT NULL,
    picture BLOB,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(comic, episode_seq, picture_seq)
);
