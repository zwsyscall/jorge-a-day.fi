use askama::Template;

#[derive(Template)]
#[template(path = "gallery.html.j2", ext = "html")]
pub struct GalleryPage {
    pub images: Vec<String>,
}

#[derive(Template)]
#[template(path = "about.html.j2", ext = "html")]
pub struct AboutPage {
    pub image_count: usize,
    pub random_image: String,
}
