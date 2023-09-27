mod authenticate_token;
mod config;
mod google_oauth;
mod handler;
mod model;

use actix_web::web::Data;
use awmp::PartsConfig;
use std::fs::{create_dir_all, read_to_string};

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use model::AppState;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RuntimeConfig {
    admin_emails: Vec<String>,
}

lazy_static::lazy_static! {
    pub static ref CONFIG: RuntimeConfig = serde_json::from_str(&read_to_string("config.json").unwrap()).unwrap();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "actix_web=info");
    }
    dotenv().ok();
    env_logger::init();
    let _ = create_dir_all("forms");
    let _ = create_dir_all("storage");

    let db = AppState::init().await;
    let app_data = web::Data::new(db);

    println!("🚀 Server started successfully");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        // let cors = Cors::default()
        //     .allowed_origin("http://localhost:5173")
        //     .allowed_methods(vec!["GET", "POST"])
        //     .allowed_headers(vec![
        //         header::CONTENT_TYPE,
        //         header::AUTHORIZATION,
        //         header::ACCEPT,
        //     ])
        //     .supports_credentials();
        App::new()
            .app_data(Data::new(
                PartsConfig::default()
                    .with_file_limit(500_000_000)
                    .with_text_limit(500_000_000)
                    .with_temp_dir("./storage"),
            ))
            .app_data(app_data.clone())
            .configure(handler::config)
            .wrap(cors)
            .wrap(Logger::default())
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
