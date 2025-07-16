use std::sync::Arc;

use crate::endpoints::schema::CompressQuery;
use crate::image_cache::cache::Cache;
use crate::{cache::CacheTrait, config::AppConfig};
use actix_web::{HttpRequest, HttpResponse, Responder, get, http::header::ContentType, web};
use log::error;
use tokio::sync::Mutex;

#[get("/daily")]
async fn daily(_: web::Data<AppConfig>, cache: web::Data<Arc<Mutex<Cache>>>) -> impl Responder {
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
    cache: web::Data<Arc<Mutex<Cache>>>,
    path: web::Path<String>,
    query: web::Query<CompressQuery>,
) -> impl Responder {
    let image_path = path.into_inner();
    let compressed = query.compress.clone().map(|_| true).unwrap_or(false);

    match cache
        .lock()
        .await
        .get_data_bytes(&image_path, compressed)
        .await
    {
        Ok((content_type, image)) => {
            return HttpResponse::Ok().content_type(content_type).body(image);
        }
        Err(e) => {
            error!("Error with requested file {:?}", e);
            return HttpResponse::NotFound().finish();
        }
    }
}
