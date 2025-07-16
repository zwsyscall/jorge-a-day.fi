mod cache;
mod config;
mod endpoints;
mod image_cache;

use actix_web::{App, HttpServer, middleware, web};
use confique::Config;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::cache::{cache_cleanup, directory_watcher};
#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app_config = config::AppConfig::from_file(config::CONFIG_PATH)?;
    let bind_address = app_config.address.clone();

    let ssl_enabled = app_config.ssl;
    let certificate_bundle = app_config.check();

    let mut cache = image_cache::cache::Cache::from(app_config.cache_age);
    cache.init(&app_config).await;

    let shared_cache = Arc::new(Mutex::new(cache));

    {
        let cache = Arc::clone(&shared_cache);
        tokio::spawn(async move {
            cache_cleanup(cache).await;
        });
    }

    {
        let cache = Arc::clone(&shared_cache);
        tokio::spawn(async move {
            directory_watcher(cache).await;
        });
    }

    info!("Starting server at {}", bind_address);

    let app_config = app_config.clone();
    let shared_cache = Arc::clone(&shared_cache);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(web::Data::new(app_config.clone()))
            .app_data(web::Data::new(shared_cache.clone()))
            .service(endpoints::api::routes::daily)
            .service(endpoints::api::routes::get_image)
            .service(endpoints::api::routes::list_images)
            .service(endpoints::ui::routes::gallery)
            .service(endpoints::ui::routes::favicon)
    });

    if ssl_enabled {
        match certificate_bundle {
            Ok((cert, key)) => {
                info!("Starting server with SSL enabled.");
                let builder = config::create_ssl_builder(&cert, &key).map_err(|e| {
                    error!("Error creating TLS instance: {}", e);
                    anyhow::anyhow!("Cannot access certificates!")
                })?;
                server.bind_openssl(bind_address, builder)?.run().await?;
            }
            Err(e) => {
                error!("Invalid certificate bundle: {}", e);
                return Err(e.into());
            }
        }
    } else {
        info!("Starting server without SSL.");
        server.bind(bind_address)?.run().await?;
    }

    Ok(())
}
