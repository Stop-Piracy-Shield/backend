use diesel::prelude::*;
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

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![get_signatures])
}
