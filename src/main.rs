mod cli;
mod models;
mod services;
mod storage;
mod ssh;

use clap::Parser;
use cli::{Cli, Commands, ConfigCommands};
use crate::cli::GroupCommands;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { name, host, user, port, password, key_path} => {
            match services::server::add(name, host, user, port, password, key_path) {
                Ok(_) => println!("Server added"),
                Err(e) => println!("Error: {}", e),
            }
        }
        Commands::List => {
            services::server::list();
        }
        Commands::Remove { name } => {
            if let Err(e) = services::server::remove(name) {
                println!("Error: {}", e);
            } else {
                println!("Server removed");
            }
        }
        Commands::Run { name, command } => {
            services::server::run(name, command);
        }
        Commands::Group { command } => {
            match command {
                GroupCommands::Add { name, servers } => {
                    match services::group::add(name, servers) {
                        Ok(_) => println!("Group added"),
                        Err(e) => println!("Error: {}", e),
                    }
                }
                GroupCommands::Remove { name } => {
                    if let Err(e) = services::group::remove(name) {
                        println!("Error: {}", e);
                    } else {
                        println!("Group removed");
                    }
                }
                GroupCommands::List => {
                    services::group::list();
                }
                GroupCommands::Run { name, command } => {
                    match command {
                        Some(cmd) => {
                            services::group::run_stream(name, cmd).await;
                        }
                        None => {
                            services::group::interactive(name);
                        }
                    }
                }
            }
        }
        Commands::Config { command } => {
            match command {
                ConfigCommands::Set { key, value } => {
                    if let Err(e) = services::config::set(key, value) {
                        println!("Error: {}", e);
                    } else {
                        println!("Config updated");
                    }
                }
                ConfigCommands::Show => {
                    services::config::show();
                }
            }
        }
    }
}