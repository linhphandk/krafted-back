use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub database_pool_size: u32,
    pub authentik_base_url: String,
    pub authentik_client_id: String,
    pub authentik_client_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
