mod api;
mod api_schema;
mod cache;
mod config;
mod image_cache;
mod web_gui;

use actix_web::{App, HttpServer, middleware, web};
use confique::Config;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::cache::{cache_cleanup, directory_watcher};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Fix this jibberish
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));
    let app_config = config::AppConfig::from_file(config::CONFIG_PATH).unwrap();

    let mut cache = image_cache::cache::Cache::from(app_config.cache_age);
    cache.init(&app_config.clone()).await;

    let shared_cache = Arc::new(Mutex::new(cache));
    let shared_cache_clone = Arc::clone(&shared_cache);
    let shared_cache_clone_2 = Arc::clone(&shared_cache);

    tokio::spawn(async move { cache_cleanup(shared_cache_clone).await });
    tokio::spawn(async move { directory_watcher(shared_cache_clone_2).await });

    log::info!("Starting server at http://127.0.0.1:8080");
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(app_config.to_owned()))
            .app_data(web::Data::new(shared_cache.to_owned()))
            .service(api::get_daily_image)
            .service(api::get_image)
            .service(api::list_images)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
