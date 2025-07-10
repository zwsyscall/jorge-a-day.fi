use std::sync::Arc;

use crate::config::AppConfig;
use actix_web::{HttpResponse, Responder, get, http::header::ContentType, web};
use log::error;
use tokio::sync::Mutex;

use crate::cache::ImageCache;

#[get("/daily")]
async fn get_daily_image(
    _: web::Data<AppConfig>,
    cache: web::Data<Arc<Mutex<ImageCache>>>,
) -> impl Responder {
    let image = cache.lock().await.get_newest_image().await.unwrap();

    HttpResponse::Ok()
        .content_type(image.content_type())
        .body(image.data)
}

#[get("/images")]
async fn list_images(
    _: web::Data<AppConfig>,
    cache: web::Data<Arc<Mutex<ImageCache>>>,
) -> impl Responder {
    let images = cache.lock().await.get_images().await;
    HttpResponse::Ok()
        .content_type(ContentType::json())
        .json(images)
}

#[get("/images/{id}")]
async fn get_image(
    _: web::Data<AppConfig>,
    cache: web::Data<Arc<Mutex<ImageCache>>>,
    path: web::Path<String>,
) -> impl Responder {
    let image_path = path.into_inner();

    match cache.lock().await.fetch_image(&image_path).await {
        Ok(image) => HttpResponse::Ok()
            .content_type(image.content_type())
            .body(image.data),
        Err(e) => {
            error!("Error with requested file {:?}", e);
            HttpResponse::NotFound().finish()
        }
    }
}
