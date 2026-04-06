use std::fs;
use crate::models::Server;

const FILE: &str = "servers.json";

pub fn load() -> Vec<Server> {
    let data = fs::read_to_string(FILE).unwrap_or("[]".to_string());
    serde_json::from_str(&data).unwrap_or(vec![])
}

pub fn save(servers: &Vec<Server>) {
    let json = serde_json::to_string_pretty(servers).unwrap();
    fs::write(FILE, json).unwrap();
}
