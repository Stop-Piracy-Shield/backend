use diesel::prelude::*;
use rocket::http::Status;
use stop_piracy_shield::establish_connection;
use stop_piracy_shield::models::*;

#[macro_use]
extern crate rocket;
use rocket::serde::json::Json;
use uuid::Uuid;

use stop_piracy_shield::schema::signatures;

#[get("/signatures")]
fn get_signatures() -> Json<Vec<PublicSignature>> {
    use crate::signatures::dsl::*;
    let connection = &mut establish_connection();

    signatures
        .filter(verified.eq(true))
        .order(created_at.desc())
        .select(PublicSignature::as_select())
        .load(connection)
        .expect("Error loading signatures")
        .into()
}

#[post("/signatures", data = "<signature>")]
fn new_signature(signature: Json<SignatureForm>) -> Status {
    let connection: &mut PgConnection = &mut establish_connection();

    diesel::insert_into(signatures::table)
        .values(&*signature)
        .returning(PublicSignature::as_returning())
        .get_result(connection)
        .expect("Error saving signature");
    Status::Ok
}

#[put("/signatures/<signature_id>/verify")]
fn verify_signature(signature_id: &str) -> Status {
    use crate::signatures::dsl::*;
    let connection: &mut PgConnection = &mut establish_connection();

    let signature_uuid = match Uuid::parse_str(signature_id) {
        Ok(u) => u,
        Err(_) => return Status::BadRequest,
    };

    match diesel::update(signatures.find(signature_uuid))
        .set(&SignatureFormVerify {
            verified: true,
            verified_at: Some(chrono::Utc::now().naive_utc()),
        })
        .execute(connection)
    {
        Ok(_) => Status::Ok,
        Err(_) => Status::NotFound,
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount(
        "/",
        routes![get_signatures, new_signature, verify_signature,],
    )
}
