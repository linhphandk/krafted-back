use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub sub: String,
    pub email: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct OidcConfig {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub revocation_endpoint: String,
    pub jwks_uri: String,
}
