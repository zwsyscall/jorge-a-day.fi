mod api;
mod api_schema;
mod cache;
mod config;
mod image_cache;
mod web_gui;

use actix_web::{App, HttpServer, middleware, web};
use confique::Config;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::cache::{cache_cleanup, directory_watcher};

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Fix this jibberish
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let app_config = config::AppConfig::from_file(config::CONFIG_PATH)?;
    let ssl_enabled = app_config.ssl;
    let certificate_bundle = app_config.check();

    let bind_address = app_config.address.to_owned();

    let mut cache = image_cache::cache::Cache::from(app_config.cache_age);
    cache.init(&app_config.clone()).await;

    let shared_cache = Arc::new(Mutex::new(cache));
    let shared_cache_clone = Arc::clone(&shared_cache);
    let shared_cache_clone_2 = Arc::clone(&shared_cache);

    tokio::spawn(async move { cache_cleanup(shared_cache_clone).await });
    tokio::spawn(async move { directory_watcher(shared_cache_clone_2).await });

    info!("Starting server at {}", bind_address);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(app_config.to_owned()))
            .app_data(web::Data::new(shared_cache.to_owned()))
            .service(api::daily)
            .service(api::get_image)
            .service(api::list_images)
            .service(web_gui::gallery)
            .service(web_gui::favicon)
    });

    // Start the server with (or without) SSL
    if ssl_enabled {
        if let Ok((cert, key)) = certificate_bundle {
            info!("Starting server with SSL enabled.");
            let builder = match config::create_ssl_builder(&cert, &key) {
                Ok(data) => data,
                Err(e) => {
                    error!("Error creating TLS instance: {}", e);
                    panic!("Cannot access certificates!")
                }
            };
            server.bind_openssl(bind_address, builder)?.run().await?;
        }
    } else {
        info!("Starting server without SSL enabled.");
        server.bind(bind_address)?.run().await?;
    }

    Ok(())
}
