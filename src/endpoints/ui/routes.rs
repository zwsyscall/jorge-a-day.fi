use crate::{cache::CacheTrait, endpoints::ui::pages::GalleryPage, image_cache::cache::Cache};
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
    let image_size = cache.lock().await.len();

    /*let page = AboutPage { images: data };
    match page.render() {
        Ok(page) => HttpResponse::Ok().body(page),
        Err(_) => HttpResponse::InternalServerError().body("Error templating gallery page"),
    }*/
    "ok"
}

#[get("/favicon.ico")]
async fn favicon() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/vnd.microsoft.icon")
        .body(&include_bytes!("../../../static/favicon.ico")[..])
}
