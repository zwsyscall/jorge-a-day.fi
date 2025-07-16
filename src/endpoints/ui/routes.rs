use crate::{
    cache::CacheTrait,
    endpoints::ui::pages::{AboutPage, GalleryPage},
    image_cache::cache::Cache,
};
use actix_web::{HttpResponse, Responder, get, web};
use askama::Template;
use std::sync::Arc;

use tokio::sync::Mutex;

#[get("/")]
async fn gallery(cache: web::Data<Arc<Mutex<Cache>>>) -> impl Responder {
    let data: Vec<String> = cache
        .lock()
        .await
        .get_images("images")
        .await
        .iter()
        .map(|i| i.url.to_owned())
        .collect();

    let page = GalleryPage { images: data };
    match page.render() {
        Ok(page) => HttpResponse::Ok().body(page),
        Err(_) => HttpResponse::InternalServerError().body("Error templating gallery page"),
    }
}

#[get("/about")]
async fn about(cache: web::Data<Arc<Mutex<Cache>>>) -> impl Responder {
    use rand::prelude::*;
    let mut rng = rand::rng();

    let cache_lock = cache.lock().await;
    let len = cache_lock.len();
    let images = cache_lock.get_images("images").await;
    if let Some(random_image) = images.choose(&mut rng) {
        let page = AboutPage {
            image_count: len,
            random_image: random_image.url.clone(),
        };

        return match page.render() {
            Ok(page) => HttpResponse::Ok().body(page),
            Err(_) => HttpResponse::InternalServerError().body("Error templating about page"),
        };
    }
    HttpResponse::InternalServerError().body("Error generating about page")
}

#[get("/favicon.ico")]
async fn favicon() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/vnd.microsoft.icon")
        .body(&include_bytes!("../../../static/favicon.ico")[..])
}
