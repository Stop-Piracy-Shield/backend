use diesel::prelude::*;
use stop_piracy_shield::models::*;
use stop_piracy_shield::establish_connection;

fn get_signatures() -> Vec<Signature> {
    use stop_piracy_shield::schema::signatures::dsl::*;

    let connection = &mut establish_connection();
    let results = signatures
        .select(Signature::as_select())
        .load(connection)
        .expect("Error loading signatures");

    dbg!(results)
}

fn main() {
    get_signatures();
}
