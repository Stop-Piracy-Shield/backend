use chrono::prelude::*;
use diesel::prelude::*;
use rocket::{
    serde::{Deserialize, Serialize},
    FromForm,
};
use uuid;

#[derive(Debug, Queryable, Selectable, Insertable, Identifiable)]
#[diesel(primary_key(id))]
#[diesel(table_name = crate::schema::signatures)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Signature {
    id: uuid::Uuid,
    first_name: String,
    last_name: String,
    org: Option<String>,
    email: String,
    created_at: NaiveDateTime,
    verified: bool,
    verified_at: Option<NaiveDateTime>,
}

#[derive(Serialize, Queryable, Selectable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::signatures)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublicSignature {
    first_name: String,
    last_name: String,
    org: Option<String>,
    created_at: NaiveDateTime,
}

#[derive(FromForm, Deserialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::signatures)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SignatureForm {
    first_name: String,
    last_name: String,
    org: Option<String>,
    email: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::signatures)]
pub struct SignatureFormVerify {
    pub verified: bool,
    pub verified_at: Option<NaiveDateTime>,
}
