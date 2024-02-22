// @generated automatically by Diesel CLI.

diesel::table! {
    guild_settings (id) {
        id -> BigInt,
        speed -> Double,
        volume -> Double,
        loop_type -> SmallInt,
        bass_enabled -> Bool,
        bass_gain -> Double,
        equalizer_32 -> Double,
        equalizer_64 -> Double,
        equalizer_125 -> Double,
        equalizer_250 -> Double,
        equalizer_500 -> Double,
        equalizer_1k -> Double,
        equalizer_2k -> Double,
        equalizer_4k -> Double,
        equalizer_8k -> Double,
        equalizer_16k -> Double,
    }
}

diesel::table! {
    users (id) {
        id -> BigInt,
        password -> Text,
        is_admin -> Bool,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    guild_settings,
    users,
);
