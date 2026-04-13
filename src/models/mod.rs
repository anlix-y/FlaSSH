use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
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

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub default_color: String,
    pub hotkeys: Hotkeys,
}

#[derive(Serialize, Deserialize)]
pub struct Hotkeys {
    pub switch_focus: String,
    pub sort_output: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_color: "green".to_string(),
            hotkeys: Hotkeys {
                switch_focus: "alt+tab".to_string(),
                sort_output: "alt+s".to_string(),
            },
        }
    }
}