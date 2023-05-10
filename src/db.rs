use dotenvy::dotenv;
use std::env;
use polodb_core::Database;

pub fn establish_connection() -> Database {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    Database::open_file(&database_url).unwrap()
}
