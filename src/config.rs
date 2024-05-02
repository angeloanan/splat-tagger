use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    pub google_api_key: String,
    pub statink: StatInk,
}

#[derive(Serialize, Deserialize, Default)]
pub struct StatInk {
    pub username: String,
    pub identity_cookie: String,
}
