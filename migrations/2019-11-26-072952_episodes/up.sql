CREATE TABLE episodes (
    provider TEXT NOT NULL,
    id TEXT NOT NULL,
    seq INTEGER NOT NULL,
    title TEXT,
    images_count INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_update TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(provider, id, seq)
);

INSERT INTO episodes
    SELECT provider,comic_id,episode_seq,episode_name,count(image),min(updated_at),max(updated_at) from comics
        GROUP BY provider,comic_id,episode_seq