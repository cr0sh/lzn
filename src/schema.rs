table! {
    lezhin (comic, episode_seq, picture_seq) {
        comic -> Text,
        episode_seq -> Integer,
        episode -> Nullable<Text>,
        picture_seq -> Integer,
        picture -> Nullable<Binary>,
        updated_at -> Timestamp,
    }
}

table! {
    titles (id) {
        id -> Text,
        title -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(lezhin, titles,);
