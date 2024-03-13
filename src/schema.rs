// @generated automatically by Diesel CLI.

diesel::table! {
    signatures (id) {
        id -> Uuid,
        first_name -> Varchar,
        last_name -> Varchar,
        org -> Nullable<Varchar>,
        email -> Varchar,
        created_at -> Timestamp,
        verified -> Bool,
        verified_at -> Nullable<Timestamp>,
        message -> Nullable<Text>,
    }
}
