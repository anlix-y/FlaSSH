use std::fs;
use crate::models::Group;
const GROUP: &str = "groups.json";

pub fn load() -> Vec<Group> {
    let data = fs::read_to_string(GROUP).unwrap_or("[]".to_string());
    serde_json::from_str(&data).unwrap_or(vec![])
}

pub fn save(groups: &Vec<Group>) {
    let json = serde_json::to_string_pretty(groups).unwrap();
    fs::write(GROUP, json).unwrap();
}