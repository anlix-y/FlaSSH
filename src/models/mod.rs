use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Server {
    pub name: String,
    pub host: String,
    pub user: String,
    pub port: u16,

    pub password: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub servers: Vec<String>,
}