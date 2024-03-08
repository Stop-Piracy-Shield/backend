use diesel::prelude::*;
use rocket::http::Status;
use stop_piracy_shield::establish_connection;
use stop_piracy_shield::models::*;

#[macro_use]
extern crate rocket;
use rocket::serde::json::Json;

#[get("/signatures")]
fn get_signatures() -> Json<Vec<PublicSignature>> {
    use stop_piracy_shield::schema::signatures::dsl::*;
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
    use stop_piracy_shield::schema::signatures;
    let connection = &mut establish_connection();

    diesel::insert_into(signatures::table)
        .values(&*signature)
        .returning(PublicSignature::as_returning())
        .get_result(connection)
        .expect("Error saving signature");
    ();
    Status::Ok
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![get_signatures, new_signature])
}
