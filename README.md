# tmail

CLI to create Fastmail masked emails.

## Install

```bash
cargo install --path .
```

## Setup

1. Go to Fastmail → Settings → Privacy & Security → API tokens
2. Create token with "Masked Email" scope
3. Run `tmail login` and paste token

## Usage

```bash
# Create masked email
tmail masked create

# Create with description
tmail masked create -d "newsletter signup"
```

## Config

Stored at `~/.config/tmail/config.toml`
