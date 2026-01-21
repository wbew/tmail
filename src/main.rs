use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tmail::FastmailClient;
use serde_json;

#[derive(Parser)]
#[command(name = "tmail")]
#[command(about = "CLI for interacting with email APIs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with Fastmail API
    Login,
    /// Manage masked emails
    Masked {
        #[command(subcommand)]
        command: MaskedCommands,
    },
}

#[derive(Subcommand)]
enum MaskedCommands {
    /// Create a new masked email
    Create {
        /// Description for the masked email
        #[arg(short, long)]
        description: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
struct Config {
    api_token: String,
    account_id: String,
}

fn config_path() -> PathBuf {
    let home = dirs::home_dir().expect("Could not find home directory");
    let config_dir = home.join(".config").join("tmail");
    fs::create_dir_all(&config_dir).expect("Could not create config directory");
    config_dir.join("config.json")
}

fn load_config() -> Option<Config> {
    let path = config_path();
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_config(config: &Config) {
    let path = config_path();
    let content = serde_json::to_string_pretty(config).expect("Could not serialize config");
    fs::write(path, content).expect("Could not write config file");
}

fn prompt(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn login() {
    println!("Get your API token from: Fastmail → Settings → Privacy & Security → API tokens");
    println!("Create a new token with 'Masked Email' scope.\n");

    let token = prompt("Enter API token: ");
    if token.is_empty() {
        eprintln!("Error: Token cannot be empty");
        std::process::exit(1);
    }

    let client = FastmailClient::new(&token);

    match client.get_account_id() {
        Ok(account_id) => {
            let config = Config {
                api_token: token,
                account_id,
            };
            save_config(&config);
            println!("Logged in successfully. Config saved to {:?}", config_path());
        }
        Err(e) => {
            eprintln!("Login failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn create(description: Option<String>) {
    let config = load_config().expect("Not logged in. Run 'tmail login' first.");
    let client = FastmailClient::new(&config.api_token);

    match client.create_masked_email(&config.account_id, description.as_deref()) {
        Ok(masked) => {
            println!("{}", masked.email);
        }
        Err(e) => {
            eprintln!("Failed to create masked email: {}", e);
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => login(),
        Commands::Masked { command } => match command {
            MaskedCommands::Create { description } => create(description),
        },
    }
}
