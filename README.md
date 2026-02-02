# pgconnect

PostgreSQL connection manager CLI with encrypted storage. Store and manage PostgreSQL connection configurations securely, then connect with an interactive menu using your preferred client (psql or pgcli).

## Features

- 🔐 Encrypted password storage using AES-256-GCM
- 🔑 Master password protection with Argon2id
- 📦 SQLite-based storage
- 🎯 Interactive connection selector
- 🔄 Support for both `psql` and `pgcli` clients
- ✨ Full CRUD operations for connections

## Installation

### Option 1: Using Cargo Install (Recommended)

Install directly from the repository:

```bash
cargo install --path .
```

Or install from git:

```bash
cargo install --git https://github.com/empty-buffer/pgconnect.git
```

This installs to `~/.cargo/bin/` (make sure it's in your PATH).

### Option 2: Using Install Script

```bash
# Make script executable
chmod +x install.sh

# Install (may require sudo for system-wide installation)
./install.sh

# Or specify custom directory
INSTALL_DIR=/usr/bin ./install.sh
```

### Option 3: Using Make

```bash
# Build
make build

# Install system-wide (requires sudo)
sudo make install

# Install to user directory (no sudo)
make install-user

# Uninstall
sudo make uninstall
```

### Option 4: Manual Installation

```bash
# Build release binary
cargo build --release

# Copy to desired location
sudo cp target/release/pgconnect /usr/local/bin/
sudo chmod +x /usr/local/bin/pgconnect
```

## Usage

### First Time Setup

Run any command to start the setup process:

```bash
pgconnect add
```

You'll be prompted to:
1. Set a master password (encrypts your database credentials)
2. Choose your preferred PostgreSQL client (psql or pgcli)

### Commands

```bash
# Interactive connection selector (default)
pgconnect

# Add a new connection
pgconnect add

# List all saved connections
pgconnect list

# Edit an existing connection
pgconnect edit <connection-name>

# Remove a connection
pgconnect remove <connection-name>

# Change preferred client (psql/pgcli)
pgconnect set-client
```

### Example Workflow

```bash
# Add a connection
$ pgconnect add
Connection name: production-db
Host: db.example.com
Port: 5432
Database name: myapp
Username: admin
Password: ****

# Connect interactively
$ pgconnect
? Select a connection
❯ production-db (admin@db.example.com:5432/myapp)
```

## Requirements

- Rust 1.70+ (for building from source)
- PostgreSQL client: `psql` or `pgcli` (at least one must be installed)

## Data Storage

Connection data is stored in:
- **Linux/Mac**: `~/.local/share/pgconnect/pgconnect.db`
- **Windows**: `%APPDATA%\pgconnect\pgconnect.db`

All passwords are encrypted using AES-256-GCM with a key derived from your master password.

## License

[Add your license here] 
