use crate::models::Server;
use crate::storage;
use crate::ssh;

pub fn add(name: String, host: String, user: String, port: Option<u16>, password: Option<String>, key_path: Option<String> ) -> Result<(), String> {
    let mut servers = storage::server::load();

    if name == "all" {
        return Err("Reserved name".to_string());
    }

    if servers.iter().any(|s| s.name == name) {
        return Err("Server already exists".to_string());
    }

    let port = port.unwrap_or(22);

    servers.push(Server { name, host, user, port, password, key_path});

    storage::server::save(&servers);

    Ok(())
}

pub fn list() {
    let servers = storage::server::load();

    for s in servers {
        println!("{} {}@{}:{}", s.name, s.host, s.user, s.port);
    }
}

pub fn remove(name: String) {
    let mut servers = storage::server::load();

    servers.retain(|s| s.name != name);

    storage::server::save(&servers);
}

pub fn run(name: String, command: Option<String>) {
    let servers = storage::server::load();

    if servers.is_empty() {
        println!("No servers found");
        return;
    }

    if name == "all" {
        for s in &servers {
            if let Some(cmd) = &command {
                run_one(s, cmd);
            } else {
                ssh::interactive(s);
            }
        }
        return;
    }

    let server = servers.iter().find(|s| s.name == name);

    match server {
        Some(s) => {
            if let Some(cmd) = &command {
                run_one(s, cmd);
            } else {
                ssh::interactive(s);
            }
        }
        None => println!("Server not found"),
    }
}

pub fn run_one(server: &Server, command: &str) {
    ssh::execute(server, command);
}