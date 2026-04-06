use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "srv")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Add {
        name: String,
        host: String,
        user: String,
        port: Option<u16>,
        password: Option<String>,
        key_path: Option<String>,
    },
    List,
    Remove {
        name: String,
    },
    Run {
        name: String,
        command: Option<String>,
    },
    Group {
        #[command(subcommand)]
        command: GroupCommands,
    },
}

#[derive(Subcommand)]
pub enum GroupCommands {
    Add {
        name: String,
        servers: Vec<String>,
    },
    Remove {
        name: String,
    },
    List,
    Run {
        name: String,
        command: Option<String>,
    }
}