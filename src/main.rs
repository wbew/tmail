mod prompt;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tmail::{FastmailClient, MaskedEmail};
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
    /// List all masked emails
    List {
        /// Show all emails including disabled/deleted
        #[arg(short, long)]
        all: bool,
    },
    /// Create a new masked email
    Create {
        /// Description for the masked email
        #[arg(short, long)]
        description: Option<String>,
        /// Website/domain this email is for
        #[arg(short, long)]
        website: Option<String>,
    },
    /// Delete (archive) a masked email
    Delete {
        /// The email address to archive (e.g., abc123@fastmail.com)
        email: Option<String>,
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

fn list(all: bool) {
    let config = load_config().expect("Not logged in. Run 'tmail login' first.");
    let client = FastmailClient::new(&config.api_token);

    match client.list_masked_emails(&config.account_id) {
        Ok(emails) => {
            let filtered: Vec<&MaskedEmail> = if all {
                emails.iter().collect()
            } else {
                emails
                    .iter()
                    .filter(|e| e.state.as_deref() == Some("enabled"))
                    .collect()
            };

            if filtered.is_empty() {
                println!("No masked emails found.");
                return;
            }

            for email in filtered {
                let desc = email.description.as_deref().unwrap_or("");
                let domain = email.for_domain.as_deref().unwrap_or("");
                let state = email.state.as_deref().unwrap_or("unknown");
                // Extract date portion from ISO 8601 timestamp (first 10 chars: "2024-01-15")
                let created = email.created_at.as_deref().map(|s| &s[..10]).unwrap_or("");

                if all {
                    println!("{}\t{}\t{}\t{}\t{}", email.email, created, state, domain, desc);
                } else {
                    println!("{}\t{}\t{}\t{}", email.email, created, domain, desc);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to list masked emails: {}", e);
            std::process::exit(1);
        }
    }
}

fn create(description: Option<String>, website: Option<String>) {
    let config = load_config().expect("Not logged in. Run 'tmail login' first.");
    let client = FastmailClient::new(&config.api_token);

    // Interactive mode if no description provided and stdin is a TTY
    let (desc, site) = if description.is_none() && prompt::is_interactive() {
        let desc = prompt::prompt_text(
            "Description:",
            Some("What is this masked email for?"),
            None,
        );
        let site = prompt::prompt_text(
            "Website:",
            Some("Optional: domain this email is for"),
            Some("example.com"),
        );
        (desc, site)
    } else {
        (description, website)
    };

    match client.create_masked_email(&config.account_id, desc.as_deref(), site.as_deref()) {
        Ok(masked) => {
            println!("{}", masked.email);
        }
        Err(e) => {
            eprintln!("Failed to create masked email: {}", e);
            std::process::exit(1);
        }
    }
}

fn delete(email: Option<String>) {
    let Some(email) = email else {
        eprintln!("Error: No email address specified.");
        eprintln!();
        eprintln!("Usage: tmail masked delete <EMAIL>");
        eprintln!();
        eprintln!("To see your masked emails, run:");
        eprintln!("  tmail masked list");
        eprintln!();
        eprintln!("To include disabled/deleted emails:");
        eprintln!("  tmail masked list --all");
        std::process::exit(1);
    };

    let config = load_config().expect("Not logged in. Run 'tmail login' first.");
    let client = FastmailClient::new(&config.api_token);

    // Find the email in the list to get its ID
    let emails = match client.list_masked_emails(&config.account_id) {
        Ok(emails) => emails,
        Err(e) => {
            eprintln!("Failed to list masked emails: {}", e);
            std::process::exit(1);
        }
    };

    let masked = emails.iter().find(|e| e.email == email);
    let Some(masked) = masked else {
        eprintln!("Error: Masked email '{}' not found.", email);
        eprintln!();
        eprintln!("To see your masked emails, run:");
        eprintln!("  tmail masked list --all");
        std::process::exit(1);
    };

    let Some(id) = &masked.id else {
        eprintln!("Error: Masked email has no ID.");
        std::process::exit(1);
    };

    match client.delete_masked_email(&config.account_id, id) {
        Ok(()) => {
            println!("Archived: {}", email);
        }
        Err(e) => {
            eprintln!("Failed to archive masked email: {}", e);
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Login => login(),
        Commands::Masked { command } => match command {
            MaskedCommands::List { all } => list(all),
            MaskedCommands::Create { description, website } => create(description, website),
            MaskedCommands::Delete { email } => delete(email),
        },
    }
}
