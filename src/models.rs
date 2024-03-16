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
    pub id: uuid::Uuid,
    pub first_name: String,
    pub last_name: String,
    org: Option<String>,
    pub email: String,
    pub created_at: NaiveDateTime,
    pub verified: bool,
    pub verified_at: Option<NaiveDateTime>,
}

#[derive(Serialize, Queryable, Selectable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::signatures)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct PublicSignature {
    first_name: String,
    last_name: String,
    org: Option<String>,
    #[serde(with = "public_date_format")]
    created_at: NaiveDateTime,
    message: Option<String>,
}

mod public_date_format {
    use chrono::NaiveDateTime;
    use rocket::serde::Serializer;

    const FORMAT: &'static str = "%d-%m-%Y %H:%M";

    pub fn serialize<S>(
        date: &NaiveDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
    S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }
}

#[derive(FromForm, Deserialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::signatures)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SignatureForm {
    first_name: String,
    last_name: String,
    org: Option<String>,
    pub email: String,
    message: Option<String>,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::signatures)]
pub struct SignatureFormVerify {
    pub verified: bool,
    pub verified_at: Option<NaiveDateTime>,
}
