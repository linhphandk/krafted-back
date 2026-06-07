use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub database_pool_size: u32,
    pub jwt_secret: String,
    pub jwt_expiry_minutes: u64,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_endpoint: Option<String>,
    pub s3_public_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, envy::Error> {
        envy::from_env()
    }
}
