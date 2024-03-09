use diesel::prelude::*;
use rocket::http::Status;

#[macro_use]
extern crate rocket;
use rocket::serde::json::Json;
use rocket_sync_db_pools::database;
use uuid::Uuid;

use stop_piracy_shield::models::*;
use stop_piracy_shield::schema::signatures;

#[database("postgres")]
struct DbConnection(diesel::PgConnection);

#[get("/signatures")]
async fn get_signatures(conn: DbConnection) -> Json<Vec<PublicSignature>> {
    use crate::signatures::dsl::*;

    conn.run(|c: &mut diesel::PgConnection| {
        signatures
            .filter(verified.eq(true))
            .order(created_at.desc())
            .select(PublicSignature::as_select())
            .load(c)
            .expect("Error loading signatures")
            .into()
    })
    .await
}

#[post("/signatures", data = "<signature>")]
async fn new_signature(conn: DbConnection, signature: Json<SignatureForm>) -> Status {
    conn.run(move |c: &mut diesel::PgConnection| {
        diesel::insert_into(signatures::table)
            .values(&*signature)
            .returning(PublicSignature::as_returning())
            .get_result(c)
            .expect("Error saving signature")
    })
    .await;
    Status::Ok
}

#[put("/signatures/<signature_id>/verify")]
async fn verify_signature(conn: DbConnection, signature_id: &str) -> Status {
    use crate::signatures::dsl::*;

    let signature_uuid = match Uuid::parse_str(signature_id) {
        Ok(u) => u,
        Err(_) => return Status::BadRequest,
    };

    conn.run(move |c: &mut diesel::PgConnection| {
        match diesel::update(signatures.find(signature_uuid))
            .set(&SignatureFormVerify {
                verified: true,
                verified_at: Some(chrono::Utc::now().naive_utc()),
            })
            .execute(c)
        {
            Ok(_) => Status::Ok,
            Err(_) => Status::NotFound,
        }
    })
    .await
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![get_signatures, new_signature, verify_signature,],
        )
        .attach(DbConnection::fairing())
}
