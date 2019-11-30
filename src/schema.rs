table! {
    comics (provider, comic_id, episode_seq, image_seq) {
        provider -> Text,
        comic_id -> Text,
        episode_seq -> Integer,
        episode_name -> Nullable<Text>,
        image_seq -> Integer,
        image -> Binary,
        updated_at -> Timestamp,
    }
}

table! {
    episodes (provider, id, seq) {
        provider -> Text,
        id -> Text,
        seq -> Integer,
        title -> Nullable<Text>,
        images_count -> Integer,
        created_at -> Timestamp,
        last_update -> Timestamp,
    }
}

table! {
    scraping_targets (provider, id) {
        provider -> Text,
        id -> Text,
        status -> Integer,
        last_scraping -> Timestamp,
    }
}

table! {
    titles (provider, id) {
        provider -> Text,
        id -> Text,
        title -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(
    comics,
    episodes,
    scraping_targets,
    titles,
);
