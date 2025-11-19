mod commands;
mod config;
mod models;

use clap::{Parser, Subcommand};
use commands::*;
use std::process;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Create {
        #[arg(short, long, default_value_t = false)]
        save: bool,
    },

    #[command(visible_alias = "bal")]
    Balance {},

    Send {
        #[arg(short, long)]
        to: String,

        #[arg(short, long)]
        amount: u64,
    },
}

impl Cli {
    pub fn print_default_help() {
        println!("ASI Chain Wallet CLI\n");
        println!("Use 'help' to see available commands");
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    let result = match &cli.command {
        Some(Commands::Create { save }) => handle_create_wallet(*save),
        Some(Commands::Balance {}) => handle_get_balance().await,
        Some(Commands::Send { to, amount }) => handle_send(to, *amount).await,
        None => {
            Cli::print_default_help();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
