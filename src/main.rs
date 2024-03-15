use diesel::prelude::*;
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::{Build, Rocket};

#[macro_use]
extern crate rocket;
use rocket::serde::json::{json, Json, Value};
use rocket_sync_db_pools::database;
use uuid::Uuid;

use stop_piracy_shield::schema::signatures;
use stop_piracy_shield::{models::*, send_confirmation_email};

#[database("postgres")]
struct DbConnection(diesel::PgConnection);

#[macro_export]
macro_rules! error_response {
    ($msg:expr, $status:expr) => {
        (
            $status,
            rocket::serde::json::Json(rocket::serde::json::json!({ "ok": false, "error": $msg }))
        )
    };
}

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
async fn new_signature(
    conn: DbConnection,
    signature: Json<SignatureForm>,
) -> Result<Json<Value>, (Status, Json<Value>)> {
    use crate::signatures::dsl::*;

    let email_to = signature.0.email.clone();
    let email_exists = conn
        .run(move |c: &mut diesel::PgConnection| {
            signatures
                .filter(email.eq(&email_to))
                .select(id)
                .first::<Uuid>(c)
                .optional()
        })
        .await
        .map_err(|_| {
            error_response!(
                "Database error during email check",
                Status::InternalServerError
            )
        })?;
    if email_exists.is_some() {
        // You can't sign twice with the same email
        return Err(error_response!(
            "Email already used for signing",
            Status::Forbidden
        ));
    }

    let signature_result = conn
        .run(move |c: &mut diesel::PgConnection| {
            diesel::insert_into(signatures)
                .values(&*signature)
                .returning(Signature::as_returning())
                .get_result(c)
                .optional()
        })
        .await;

    match signature_result {
        Ok(Some(signature)) => match send_confirmation_email(signature) {
            Ok(_) => Ok(Json(json!({"ok": true}))),
            Err((maybe_err, signature_id)) => {
                let _ = conn
                    .run(move |c: &mut diesel::PgConnection| {
                        diesel::delete(signatures.find(signature_id)).execute(c)
                    })
                    .await;
                if let Some(err) = maybe_err {
                    Err((
                        Status::BadRequest,
                        Json(json!({"ok": false, "error": err.to_string()})),
                    ))
                } else {
                    Err(error_response!(
                        "Failed to send confirmation email",
                        Status::InternalServerError
                    ))
                }
            }
        },
        Ok(None) => Err(error_response!(
            "Failed to save signature",
            Status::BadRequest
        )),
        Err(_) => Err(error_response!(
            "Database error during signature saving",
            Status::InternalServerError
        )),
    }
}

#[put("/signatures/<signature_id>/verify")]
async fn verify_signature(
    conn: DbConnection,
    signature_id: &str,
) -> Result<Json<Value>, (Status, Json<Value>)> {
    use crate::signatures::dsl::*;

    let signature_uuid = Uuid::parse_str(signature_id)
        .map_err(|_| error_response!("Invalid signature id format", Status::BadRequest))?;

    conn.run(move |c: &mut diesel::PgConnection| {
        diesel::update(signatures.find(signature_uuid))
            .set(&SignatureFormVerify {
                verified: true,
                verified_at: Some(chrono::Utc::now().naive_utc()),
            })
            .execute(c)
            .map(|_| Json(json!({"ok": true})))
            .map_err(|_| error_response!("Signature not found", Status::NotFound))
    })
    .await
}

#[get("/signatures/<signature_id>")]
async fn get_signature_by_id(
    conn: DbConnection,
    signature_id: &str,
) -> Result<Json<PublicSignature>, (Status, Json<Value>)> {
    use crate::signatures::dsl::*;

    let signature_uuid = Uuid::parse_str(signature_id)
        .map_err(|_| error_response!("Invalid signature id format", Status::BadRequest))?;
    conn.run(move |c: &mut diesel::PgConnection| {
        signatures
            .find(signature_uuid)
            .select(PublicSignature::as_select())
            .first::<PublicSignature>(c)
    })
    .await
    .map(Json)
    .map_err(|_| error_response!("Signature not found", Status::NotFound))
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
            routes![
                get_signatures,
                new_signature,
                verify_signature,
                get_signature_by_id,
            ],
        )
        .attach(DbConnection::fairing())
        .attach(AdHoc::on_ignite("Run Migrations", run_migrations))
}
