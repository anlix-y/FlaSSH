use crate::models::Group;
use crate::{services, ssh, storage};

pub fn add(name: String, servers: Vec<String>) -> Result<(), String> {
    let mut groups = storage::group::load();

    if groups.iter().any(|s| s.name == name) {
        return Err("Group already exists".to_string());
    }

    groups.push(Group{ name, servers });

    storage::group::save(&groups);

    Ok(())
}

pub fn list() {
    let groups = storage::group::load();

    for g in groups {
        println!("{} - {:?}", g.name, g.servers);
    }
}

pub fn remove(name: String) {
    let mut groups = storage::group::load();

    groups.retain(|s| s.name != name);

    storage::group::save(&groups);
}

pub fn run(name: String, command: Option<String>) {
    let groups = storage::group::load();

    if groups.is_empty() {
        println!("No groups found");
        return;
    }

    let group = groups.iter().find(|g| g.name == name);

    match group {
        Some(group) => {
            let servers = storage::server::load();

            for server_name in &group.servers {
                match servers.iter().find(|s| s.name == *server_name) {
                    Some(server) => {
                        if let Some(cmd) = &command {
                            services::server::run_one(server, cmd);
                        } else {
                            ssh::interactive(server);
                        }
                    }
                    None => {
                        println!("Server '{}' not found", server_name);
                    }
                }
            }
        }
        None => {
            println!("Group not found");
        }
    }
}
