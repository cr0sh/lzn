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
