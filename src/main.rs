mod cli;
mod models;
mod services;
mod storage;
mod ssh;

use clap::Parser;
use cli::{Cli, Commands};
use crate::cli::GroupCommands;

fn main() {
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
            services::server::remove(name);
        }
        Commands::Run { name, command } => {
            services::server::run(name, command);
        }
        Commands::Group { command } => {
            match command {
                GroupCommands::Add { name, servers } => {
                    services::group::add(name, servers);
                }
                GroupCommands::Remove { name } => {
                    services::group::remove(name);
                }
                GroupCommands::List => {
                    services::group::list();
                }
                GroupCommands::Run { name, command } => {
                    services::group::run(name, command);
                }
            }
        }
    }
}