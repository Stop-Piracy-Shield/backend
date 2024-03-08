use chrono::prelude::*;
use diesel::prelude::*;
use uuid;

#[derive(Debug, Queryable, Selectable, Insertable)]
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
