use diesel::prelude::*;
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::{Build, Rocket};

#[macro_use]
extern crate rocket;
use rocket::serde::json::Json;
use rocket_sync_db_pools::database;
use uuid::Uuid;

use stop_piracy_shield::schema::signatures;
use stop_piracy_shield::{models::*, send_confirmation_email};

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
    use crate::signatures::dsl::*;

    let email_to = signature.0.email.clone();
    if conn
        .run(move |c: &mut diesel::PgConnection| {
            signatures
                .filter(email.eq(&email_to))
                .select(PublicSignature::as_select())
                .load(c)
                .expect("Error loading signatures")
                .len()
        })
        .await
        > 0
    {
        // You can't sign twice with the same email
        return Status::Forbidden;
    }

    let signature = conn
        .run(move |c: &mut diesel::PgConnection| {
            diesel::insert_into(signatures)
                .values(&*signature)
                .returning(Signature::as_returning())
                .get_result(c)
                .expect("Error saving signature")
        })
        .await;

    match send_confirmation_email(signature) {
        Ok(_) => Status::Ok,
        Err(signature_id) => {
            let _ = conn
                .run(move |c: &mut diesel::PgConnection| {
                    diesel::delete(signatures.find(signature_id)).execute(c)
                })
                .await;
            Status::BadRequest
        }
    }
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

#[get("/signatures/<signature_id>")]
async fn get_signature_by_id(
    conn: DbConnection,
    signature_id: &str,
) -> Result<Json<PublicSignature>, Status> {
    use crate::signatures::dsl::*;

    let signature_uuid = match Uuid::parse_str(signature_id) {
        Ok(u) => u,
        Err(_) => return Err(Status::BadRequest),
    };

    conn.run(move |c: &mut diesel::PgConnection| {
        signatures
            .find(signature_uuid)
            .select(PublicSignature::as_select())
            .first::<PublicSignature>(c)
    })
    .await
    .map(Json)
    .map_err(|_| Status::NotFound)
}

async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    DbConnection::get_one(&rocket)
        .await
        .expect("database connection")
        .run(|conn| {
            conn.run_pending_migrations(MIGRATIONS)
                .expect("diesel migrations");
        })
        .await;

    rocket
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![get_signatures, new_signature, verify_signature, get_signature_by_id,],
        )
        .attach(DbConnection::fairing())
        .attach(AdHoc::on_ignite("Run Migrations", run_migrations))
}
