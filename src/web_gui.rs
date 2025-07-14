use std::sync::Arc;

use crate::image_cache::cache::Cache;
use actix_web::web::Html;
use actix_web::{HttpResponse, Responder, get, web};
use askama::Template;

use tokio::sync::Mutex;
#[derive(Template)]
#[template(path = "gallery.html", ext = "html")]
struct GalleryPage {
    images: Vec<String>,
}

#[get("/gallery")]
async fn gallery(cache: web::Data<Arc<Mutex<Cache>>>) -> impl Responder {
    let data: Vec<String> = cache
        .lock()
        .await
        .get_images("images")
        .await
        .iter()
        .map(|i| i.url.clone())
        .collect();

    let page = GalleryPage { images: data };
    Html::new(page.render().unwrap())
}

#[get("/favicon.ico")]

async fn favicon() -> impl Responder {
    HttpResponse::Ok()
        .content_type("image/vnd.microsoft.icon")
        .body(&include_bytes!("../static/favicon.ico")[..])
}
