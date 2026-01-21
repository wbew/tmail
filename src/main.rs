use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

const FASTMAIL_SESSION_URL: &str = "https://api.fastmail.com/jmap/session";
const FASTMAIL_API_URL: &str = "https://api.fastmail.com/jmap/api/";
const MASKED_EMAIL_CAPABILITY: &str = "https://www.fastmail.com/dev/maskedemail";

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

#[derive(Deserialize)]
struct SessionResponse {
    #[serde(rename = "primaryAccounts")]
    primary_accounts: HashMap<String, String>,
}

#[derive(Serialize)]
struct JmapRequest {
    using: Vec<String>,
    #[serde(rename = "methodCalls")]
    method_calls: Vec<(String, serde_json::Value, String)>,
}

#[derive(Deserialize)]
struct JmapResponse {
    #[serde(rename = "methodResponses")]
    method_responses: Vec<(String, serde_json::Value, String)>,
}

#[derive(Deserialize)]
struct MaskedEmailResult {
    email: String,
}

fn config_path() -> PathBuf {
    let config_dir = dirs::config_dir()
        .expect("Could not find config directory")
        .join("tmail");
    fs::create_dir_all(&config_dir).expect("Could not create config directory");
    config_dir.join("config.toml")
}

fn load_config() -> Option<Config> {
    let path = config_path();
    let content = fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

fn save_config(config: &Config) {
    let path = config_path();
    let content = toml::to_string_pretty(config).expect("Could not serialize config");
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

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(FASTMAIL_SESSION_URL)
        .bearer_auth(&token)
        .send();

    match response {
        Ok(resp) if resp.status().is_success() => {
            let session: SessionResponse = resp.json().expect("Failed to parse session response");
            let account_id = session
                .primary_accounts
                .get(MASKED_EMAIL_CAPABILITY)
                .expect("Masked email capability not found. Ensure your token has the correct scope.");

            let config = Config {
                api_token: token,
                account_id: account_id.clone(),
            };
            save_config(&config);
            println!("Logged in successfully. Config saved to {:?}", config_path());
        }
        Ok(resp) => {
            eprintln!("Authentication failed: {}", resp.status());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Request failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn create(description: Option<String>) {
    let config = load_config().expect("Not logged in. Run 'tmail login' first.");

    let create_obj = serde_json::json!({
        "new": {
            "state": "enabled",
            "description": description.unwrap_or_default(),
            "forDomain": ""
        }
    });

    let request = JmapRequest {
        using: vec![MASKED_EMAIL_CAPABILITY.to_string()],
        method_calls: vec![(
            "MaskedEmail/set".to_string(),
            serde_json::json!({
                "accountId": config.account_id,
                "create": create_obj
            }),
            "0".to_string(),
        )],
    };

    let client = reqwest::blocking::Client::new();
    let response = client
        .post(FASTMAIL_API_URL)
        .bearer_auth(&config.api_token)
        .json(&request)
        .send();

    match response {
        Ok(resp) if resp.status().is_success() => {
            let jmap_response: JmapResponse = resp.json().expect("Failed to parse JMAP response");

            if let Some((method, result, _)) = jmap_response.method_responses.first() {
                if method == "MaskedEmail/set" {
                    if let Some(created) = result.get("created") {
                        if let Some(new_email) = created.get("new") {
                            let masked: MaskedEmailResult =
                                serde_json::from_value(new_email.clone())
                                    .expect("Failed to parse masked email");
                            println!("{}", masked.email);
                            return;
                        }
                    }
                    if let Some(not_created) = result.get("notCreated") {
                        eprintln!("Failed to create masked email: {:?}", not_created);
                        std::process::exit(1);
                    }
                }
            }
            eprintln!("Unexpected response format");
            std::process::exit(1);
        }
        Ok(resp) => {
            eprintln!("Request failed: {}", resp.status());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Request failed: {}", e);
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
