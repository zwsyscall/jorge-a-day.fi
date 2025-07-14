use std::sync::Arc;

use crate::{cache::CacheTrait, config::AppConfig};
use actix_web::{HttpRequest, HttpResponse, Responder, get, http::header::ContentType, web};
use log::error;
use tokio::sync::Mutex;

use crate::image_cache::cache::Cache;

#[get("/daily")]
async fn get_daily_image(
    _: web::Data<AppConfig>,
    cache: web::Data<Arc<Mutex<Cache>>>,
) -> impl Responder {
    match cache.lock().await.get_newest_image().await {
        Some(image) => HttpResponse::Ok()
            .content_type(image.content_type())
            .body(image.data),
        None => {
            error!("Daily image is missing?");
            HttpResponse::NotFound().finish()
        }
    }
}

#[get("/images")]
async fn list_images(req: HttpRequest, cache: web::Data<Arc<Mutex<Cache>>>) -> impl Responder {
    let domain = req.full_url();

    let images = cache.lock().await.get_images(&domain.to_string()).await;
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(images)
}

#[get("/images/{id}")]
async fn get_image(
    _: web::Data<AppConfig>,
    cache: web::Data<Arc<Mutex<Cache>>>,
    path: web::Path<String>,
) -> impl Responder {
    let image_path = path.into_inner();

    match cache.lock().await.get_data(&image_path).await {
        Ok(image) => HttpResponse::Ok()
            .content_type(image.content_type())
            .body(image.data),
        Err(e) => {
            error!("Error with requested file {:?}", e);
            HttpResponse::NotFound().finish()
        }
    }
}
