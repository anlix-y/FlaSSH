use crate::models::Server;
use crate::storage;
use crate::ssh;

pub fn add(name: String, host: String, user: String, port: Option<u16>, password: Option<String>, key_path: Option<String>) -> Result<(), String> {
    if name == "all" {
        return Err("Name 'all' is reserved".to_string());
    }

    let mut servers = storage::server::load();
    if servers.iter().any(|s| s.name == name) {
        return Err(format!("Server '{}' already exists", name));
    }

    servers.push(Server {
        name,
        host,
        user,
        port: port.unwrap_or(22),
        password,
        key_path,
    });

    storage::server::save(&servers);
    Ok(())
}

pub fn list() {
    let servers = storage::server::load();
    if servers.is_empty() {
        println!("No servers configured.");
        return;
    }

    for s in servers {
        println!("{:<15} {}@{}:{}", s.name, s.user, s.host, s.port);
    }
}

pub fn remove(name: String) -> Result<(), String> {
    let mut servers = storage::server::load();
    let initial_len = servers.len();
    servers.retain(|s| s.name != name);

    if servers.len() == initial_len {
        return Err(format!("Server '{}' not found", name));
    }

    storage::server::save(&servers);
    Ok(())
}

pub fn run(name: String, command: Option<String>) {
    let servers = storage::server::load();

    if name == "all" {
        if servers.is_empty() {
            println!("No servers found");
            return;
        }
        for s in &servers {
            execute(s, &command);
        }
    } else {
        match servers.iter().find(|s| s.name == name) {
            Some(s) => execute(s, &command),
            None => println!("Server '{}' not found", name),
        }
    }
}

fn execute(server: &Server, command: &Option<String>) {
    let config = crate::storage::config::load();
    match command {
        Some(cmd) => ssh::execute(server, cmd, &config.default_color),
        None => ssh::interactive(server, &config.default_color),
    }
}