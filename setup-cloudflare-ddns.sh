#!/bin/bash

# -----------------------------
# 1. Detect OS architecture
# -----------------------------
echo "➡️  Detecting system architecture..."
ARCH=$(uname -m)
if [ "$ARCH" == "x86_64" ]; then
    RUST_TARGET="x86_64-unknown-linux-gnu"
elif [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
    RUST_TARGET="aarch64-unknown-linux-gnu"
else
    echo "❌ Unsupported architecture: $ARCH"
    exit 1
fi
echo "✅ Architecture detected: $ARCH, target: $RUST_TARGET"

# -----------------------------
# 2. Check if cargo is installed, if not install Rust
# -----------------------------
echo "➡️  Checking for Rust and Cargo..."

if ! command -v cargo &> /dev/null; then
    echo "⚠️  Cargo not found. Installing Rust using official installer..."

    # Detect if running as root or normal user for proper rustup installation
    if [ "$EUID" -eq 0 ]; then
        echo "⚠️  Rustup cannot be installed as root. Please run this script as a normal user (without sudo)."
        exit 1
    fi

    curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh -s -- -y

    # Add cargo to PATH in current shell session
    source "$HOME/.cargo/env"

    # Check again to confirm
    if ! command -v cargo &> /dev/null; then
        echo "❌ Failed to install Rust/Cargo."
        exit 1
    fi
    echo "✅ Rust and Cargo installed successfully."
else
    echo "✅ Cargo is already installed."
fi

# -----------------------------
# 3. Build Rust program for target architecture
# -----------------------------
echo "➡️  Building Rust program for $RUST_TARGET..."
if ! cargo build --release --target "$RUST_TARGET"; then
    echo "❌ Failed to build Rust program"
    exit 1
fi
echo "✅ Rust program built successfully."

# -----------------------------
# 4. Check and set up .env configuration file interactively
# -----------------------------
echo "➡️  Checking and setting up /etc/cloudflare-ddns/.env..."
sudo mkdir -p /etc/cloudflare-ddns && echo "✅ Directory ensured: /etc/cloudflare-ddns."

if [ ! -f /etc/cloudflare-ddns/.env ]; then
  echo "⚠️  WARNING: /etc/cloudflare-ddns/.env file not found!"
  echo "👉 Please paste the content of your .env file now (press Ctrl+D when done):"
  
  # Collect user input and write to the .env file
  sudo tee /etc/cloudflare-ddns/.env > /dev/null

  echo "✅ .env file created successfully."
fi

# -----------------------------
# 5. Validate required keys in .env
# -----------------------------
REQUIRED_KEYS=("CF_API_TOKEN" "CF_ZONE_ID" "CF_RECORD_IDS" "CF_RECORD_NAMES" "CF_RECORD_TYPES" "CF_RECORD_PROXIED")

echo "➡️  Validating required keys in /etc/cloudflare-ddns/.env..."
MISSING_KEYS=()
for key in "${REQUIRED_KEYS[@]}"; do
  VALUE=$(grep -E "^${key}=" /etc/cloudflare-ddns/.env | cut -d '=' -f2- | xargs)
  if [ -z "$VALUE" ]; then
    MISSING_KEYS+=("$key")
  fi
done

# If any required key is missing, stop and report
if [ ${#MISSING_KEYS[@]} -ne 0 ]; then
  echo "❌ ERROR: The following required keys are missing or empty in /etc/cloudflare-ddns/.env:"
  for key in "${MISSING_KEYS[@]}"; do
    echo "  - $key"
  done
  echo "Please edit the file manually: sudo nano /etc/cloudflare-ddns/.env"
  exit 1
else
  echo "✅ .env file is valid and contains all required keys."
fi

# -----------------------------
# 6. Copy binary and .env to /etc/cloudflare-ddns/
# -----------------------------
echo "➡️  Copying binary and .env file..."
sudo cp "target/$RUST_TARGET/release/cloudflare-ddns" /etc/cloudflare-ddns/cloudflare-ddns
sudo chmod +x /etc/cloudflare-ddns/cloudflare-ddns
echo "✅ Files copied successfully."

# -----------------------------
# 7. Setup systemd service for weekly run
# -----------------------------
echo "➡️  Setting up systemd service and timer..."
SERVICE_PATH="/etc/systemd/system/cloudflare-ddns.service"
TIMER_PATH="/etc/systemd/system/cloudflare-ddns.timer"

# Create service file
sudo tee $SERVICE_PATH > /dev/null <<EOL
[Unit]
Description=Cloudflare Dynamic DNS Updater

[Service]
Type=oneshot
WorkingDirectory=/etc/cloudflare-ddns
ExecStart=/etc/cloudflare-ddns/cloudflare-ddns
EnvironmentFile=/etc/cloudflare-ddns/.env
EOL

# Create timer file for weekly execution
sudo tee $TIMER_PATH > /dev/null <<EOL
[Unit]
Description=Run Cloudflare DDNS updater weekly

[Timer]
OnCalendar=weekly
Persistent=true

[Install]
WantedBy=timers.target
EOL

echo "✅ systemd service and timer files created."

# -----------------------------
# 8. Reload systemd, enable, and start timer
# -----------------------------
echo "➡️  Enabling and starting systemd timer..."
sudo systemctl daemon-reload
sudo systemctl enable --now cloudflare-ddns.timer

# -----------------------------
# 9. Verify setup
# -----------------------------
echo "✅ Setup complete! Here is the status of your timer:"
sudo systemctl status cloudflare-ddns.timer
