use diesel::prelude::*;
use rocket::fairing::AdHoc;
use rocket::http::Status;
use rocket::{Build, Rocket};

#[macro_use]
extern crate rocket;
use rocket::serde::json::{Json, Value};
use rocket_sync_db_pools::database;
use uuid::Uuid;

use stop_piracy_shield::schema::signatures;
use stop_piracy_shield::{
    generate_auth_token, models::*, send_confirmation_email, send_sign_email,
};

#[database("postgres")]
struct DbConnection(diesel::PgConnection);

#[macro_export]
macro_rules! ok_response {
    () => {
        rocket::serde::json::Json(rocket::serde::json::json!({ "result": true }))
    };
}
#[macro_export]
macro_rules! error_response {
    ($msg:expr, $status:expr) => {
        (
            $status,
            rocket::serde::json::Json(rocket::serde::json::json!({ "result": false, "error": $msg }))
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
            Ok(_) => Ok(ok_response!()),
            Err(signature_id) => {
                let _ = delete_signature(&conn, signature_id).await;
                Err(error_response!(
                    "Failed to send confirmation email",
                    Status::InternalServerError
                ))
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

#[put("/signatures/<auth_token>/verify")]
async fn verify_signature(
    conn: DbConnection,
    auth_token: &str,
) -> Result<Json<Value>, (Status, Json<Value>)> {
    use crate::signatures::dsl::*;

    let signature = validate_auth_token(&conn, auth_token, false).await?;

    conn.run(move |c: &mut diesel::PgConnection| {
        diesel::update(signatures.find(signature.id))
            .set(&SignatureFormVerify {
                verified: true,
                verified_at: Some(chrono::Utc::now().naive_utc()),
            })
            .execute(c)
            .map(|_| ok_response!())
            .map_err(|_| error_response!("Signature not found", Status::NotFound))
    })
    .await?;

    match send_sign_email(find_signature(&conn, signature.id).await?) {
        Ok(_) => Ok(ok_response!()),
        Err(signature_id) => {
            let _ = delete_signature(&conn, signature_id).await;
            Err(error_response!(
                "Failed to send sign email",
                Status::InternalServerError
            ))
        }
    }
}

#[put("/signatures/<auth_token>/revoke")]
async fn revoke_signature(
    conn: DbConnection,
    auth_token: &str,
) -> Result<Json<Value>, (Status, Json<Value>)> {
    let signature_uuid = validate_auth_token(&conn, auth_token, true).await?.id;
    return delete_signature(&conn, signature_uuid).await;
}

#[get("/signatures/<signature_id>")]
async fn get_signature_by_id(
    conn: DbConnection,
    signature_id: &str,
) -> Result<Json<PublicSignature>, (Status, Json<Value>)> {
    use crate::signatures::dsl::*;

    if signature_id.chars().count() < 36 {
        return Err(error_response!(
            "Invalid signature id format",
            Status::BadRequest
        ));
    }

    let signature_uuid = Uuid::parse_str(&signature_id[..36])
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

async fn find_signature(
    conn: &DbConnection,
    signature_uuid: Uuid,
) -> Result<Signature, (Status, Json<Value>)> {
    use crate::signatures::dsl::*;

    match conn
        .run(move |c: &mut diesel::PgConnection| {
            signatures
                .find(signature_uuid)
                .select(Signature::as_select())
                .first::<Signature>(c)
        })
        .await
    {
        Ok(signature) => Ok(signature),
        Err(_err) => return Err(error_response!("Signature not found", Status::Forbidden)),
    }
}

async fn validate_auth_token(
    conn: &DbConnection,
    auth_token: &str,
    expected_verified_state: bool,
) -> Result<Signature, (Status, Json<Value>)> {
    if auth_token.chars().count() != 100 {
        return Err(error_response!(
            "Invalid auth token format",
            Status::Forbidden
        ));
    }

    let signature_uuid = Uuid::parse_str(&auth_token[..36])
        .map_err(|_| error_response!("Invalid signature id format", Status::BadRequest))?;

    let signature = find_signature(conn, signature_uuid).await?;
    let generated_auth_token = generate_auth_token(&signature);

    if auth_token.eq(&generated_auth_token) && expected_verified_state == signature.verified {
        Ok(signature)
    } else {
        Err(error_response!("Auth token is invalid", Status::Forbidden))
    }
}

async fn delete_signature(
    conn: &DbConnection,
    signature_uuid: uuid::Uuid,
) -> Result<Json<Value>, (Status, Json<Value>)> {
    use crate::signatures::dsl::*;

    conn.run(move |c: &mut diesel::PgConnection| {
        diesel::delete(signatures.find(signature_uuid))
            .execute(c)
            .map(|_| ok_response!())
            .map_err(|_| error_response!("Signature not found", Status::NotFound))
    })
    .await
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
                revoke_signature,
                get_signature_by_id,
            ],
        )
        .attach(DbConnection::fairing())
        .attach(AdHoc::on_ignite("Run Migrations", run_migrations))
}
