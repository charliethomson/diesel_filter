// @generated automatically by Diesel CLI.

diesel::table! {
    thingies (id) {
        id -> Int4,
        name -> Nullable<Varchar>,
        category -> Nullable<Varchar>,
        other -> Nullable<Varchar>,
    }
}
