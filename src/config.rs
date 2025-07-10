use confique::Config;
use serde::Deserialize;

pub static CONFIG_PATH: &'static str = "./config.toml";

#[derive(Config, Deserialize, Clone)]
pub struct AppConfig {
    #[config(default = ["."])]
    pub directories: Vec<String>,
    #[config(default = 300)]
    pub cache_age: i64,
}
