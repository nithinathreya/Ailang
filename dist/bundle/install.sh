#!/usr/bin/env bash
# ==============================================================================
# Ailang: Google Gemma & Qwen -> Native Rust Interpreter Installer
# ==============================================================================
# Usage:
#   ./install.sh                Interactive installation
#   ./install.sh -y             Non-interactive installation (defaults)
#   ./install.sh --prefix /dir  Install binary to /dir (default: /usr/local/bin)
#   ./install.sh --uninstall    Uninstall Ailang from system
# ==============================================================================

set -e

# Color output helpers
BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
RESET='\033[0m'

info() {
    printf "${CYAN}${BOLD}[INFO]${RESET} %s\n" "$1"
}

success() {
    printf "${GREEN}${BOLD}[SUCCESS]${RESET} %s\n" "$1"
}

warn() {
    printf "${YELLOW}${BOLD}[WARN]${RESET} %s\n" "$1"
}

error() {
    printf "${RED}${BOLD}[ERROR]${RESET} %s\n" "$1" >&2
}

# Defaults
AUTO_YES=false
UNINSTALL=false
INSTALL_PREFIX=""
MODEL_NAME="qwen2.5-coder:0.5b"
SKIP_OLLAMA=false

# Parse command line flags
while [[ $# -gt 0 ]]; do
    case "$1" in
        -y|--yes)
            AUTO_YES=true
            shift
            ;;
        --prefix)
            INSTALL_PREFIX="$2"
            shift 2
            ;;
        --model)
            MODEL_NAME="$2"
            shift 2
            ;;
        --skip-ollama)
            SKIP_OLLAMA=true
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        -h|--help)
            echo "Ailang Installer"
            echo "Options:"
            echo "  -y, --yes          Skip prompts and accept defaults"
            echo "  --prefix <path>    Specify destination directory for ailang binary"
            echo "  --model <name>     Specify model to pull (default: qwen2.5-coder:0.5b)"
            echo "  --skip-ollama      Do not install or configure Ollama"
            echo "  --uninstall        Remove Ailang binary and configuration"
            echo "  -h, --help         Show this message"
            exit 0
            ;;
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ------------------------------------------------------------------------------
# Handle Uninstallation
# ------------------------------------------------------------------------------
if [ "$UNINSTALL" = true ]; then
    info "Uninstalling Ailang..."

    # Look for binary in common locations
    REMOVED=0
    for loc in "/usr/local/bin/ailang" "$HOME/.local/bin/ailang" "$HOME/.cargo/bin/ailang"; do
        if [ -f "$loc" ]; then
            if [ -w "$(dirname "$loc")" ]; then
                rm -f "$loc"
            else
                sudo rm -f "$loc"
            fi
            success "Removed binary from $loc"
            REMOVED=1
        fi
    done

    if [ -d "$HOME/.ailang" ]; then
        rm -rf "$HOME/.ailang"
        success "Removed configuration at $HOME/.ailang"
    fi

    if [ "$REMOVED" -eq 1 ]; then
        success "Ailang has been successfully uninstalled."
    else
        warn "No Ailang installation was found."
    fi
    exit 0
fi

# ------------------------------------------------------------------------------
# Welcome Banner
# ------------------------------------------------------------------------------
printf "${CYAN}${BOLD}"
cat << 'EOF'
========================================================================
    ___    ____ __                       
   /   |  /  _// /   ____ _____  ____ _  
  / /| |  / / / /   / __ `/ __ \/ __ `/  
 / ___ |_/ / / /___/ /_/ / / / / /_/ /   
/_/  |_/___//_____/\__,_/_/ /_/\__, /    
                              /____/     
  Google Gemma & Qwen ➜ Native Rust Dynamic Interpreter
========================================================================
EOF
printf "${RESET}\n"

# ------------------------------------------------------------------------------
# Step 1: Detect Operating System & Architecture
# ------------------------------------------------------------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"
info "Detected platform: $OS ($ARCH)"

# ------------------------------------------------------------------------------
# Step 2: Determine Destination Directory
# ------------------------------------------------------------------------------
if [ -z "$INSTALL_PREFIX" ]; then
    if [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
        INSTALL_PREFIX="/usr/local/bin"
    elif [ -d "$HOME/.local/bin" ]; then
        INSTALL_PREFIX="$HOME/.local/bin"
    elif [ -d "$HOME/.cargo/bin" ]; then
        INSTALL_PREFIX="$HOME/.cargo/bin"
    else
        INSTALL_PREFIX="/usr/local/bin"
    fi
fi

info "Installation destination: $INSTALL_PREFIX/ailang"

# ------------------------------------------------------------------------------
# Step 3: Check Rust Toolchain (rustc & cargo)
# ------------------------------------------------------------------------------
info "Checking for Rust toolchain..."
if ! command -v rustc &> /dev/null || ! command -v cargo &> /dev/null; then
    warn "Rust compiler ('rustc' / 'cargo') was not found in PATH."
    if [ "$AUTO_YES" = true ]; then
        info "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    else
        read -p "Would you like to install Rust via rustup now? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        else
            error "Rust is required to run Ailang. Please install Rust from https://rustup.rs/"
            exit 1
        fi
    fi
fi

RUST_VER="$(rustc --version)"
success "Found Rust: $RUST_VER"

# ------------------------------------------------------------------------------
# Step 4: Build or Locate `ailang` Binary
# ------------------------------------------------------------------------------
BIN_SRC=""

# Check for prebuilt release binary in current workspace
if [ -f "$SCRIPT_DIR/target/release/ailang" ]; then
    BIN_SRC="$SCRIPT_DIR/target/release/ailang"
elif [ -f "$SCRIPT_DIR/bin/ailang" ]; then
    BIN_SRC="$SCRIPT_DIR/bin/ailang"
elif [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
    info "Compiling Ailang release binary with cargo..."
    (cd "$SCRIPT_DIR" && cargo build --release)
    BIN_SRC="$SCRIPT_DIR/target/release/ailang"
else
    error "Could not find ailang binary or Cargo.toml in $SCRIPT_DIR"
    exit 1
fi

# ------------------------------------------------------------------------------
# Step 5: Install Binary to Destination
# ------------------------------------------------------------------------------
info "Installing ailang binary to $INSTALL_PREFIX/ailang..."

mkdir -p "$INSTALL_PREFIX" 2>/dev/null || true

if [ -w "$INSTALL_PREFIX" ]; then
    rm -f "$INSTALL_PREFIX/ailang"
    cp "$BIN_SRC" "$INSTALL_PREFIX/ailang"
    chmod +x "$INSTALL_PREFIX/ailang"
    if [ "$OS" = "Darwin" ]; then
        xattr -cr "$INSTALL_PREFIX/ailang" 2>/dev/null || true
        codesign --force --sign - "$INSTALL_PREFIX/ailang" 2>/dev/null || true
    fi
else
    warn "Elevated privileges required to write to $INSTALL_PREFIX. Running with sudo..."
    sudo mkdir -p "$INSTALL_PREFIX"
    sudo rm -f "$INSTALL_PREFIX/ailang"
    sudo cp "$BIN_SRC" "$INSTALL_PREFIX/ailang"
    sudo chmod +x "$INSTALL_PREFIX/ailang"
    if [ "$OS" = "Darwin" ]; then
        sudo xattr -cr "$INSTALL_PREFIX/ailang" 2>/dev/null || true
        sudo codesign --force --sign - "$INSTALL_PREFIX/ailang" 2>/dev/null || true
    fi
fi

success "Installed $INSTALL_PREFIX/ailang"

# Verify PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_PREFIX"; then
    warn "$INSTALL_PREFIX is not in your current PATH."
    warn "Add it to your shell config (~/.zshrc or ~/.bashrc):"
    warn "  export PATH=\"$INSTALL_PREFIX:\$PATH\""
fi

# ------------------------------------------------------------------------------
# Step 6: Setup Ollama & Model
# ------------------------------------------------------------------------------
if [ "$SKIP_OLLAMA" = false ]; then
    info "Checking Ollama installation..."

    if ! command -v ollama &> /dev/null; then
        warn "Ollama is not installed."
        INSTALL_OLLAMA=false
        if [ "$AUTO_YES" = true ]; then
            INSTALL_OLLAMA=true
        else
            read -p "Would you like to install Ollama now? [Y/n] " -n 1 -r
            echo
            if [[ ! $REPLY =~ ^[Nn]$ ]]; then
                INSTALL_OLLAMA=true
            fi
        fi

        if [ "$INSTALL_OLLAMA" = true ]; then
            if [ "$OS" = "Darwin" ]; then
                if command -v brew &> /dev/null; then
                    info "Installing Ollama via Homebrew..."
                    brew install ollama
                else
                    info "Installing Ollama via official installer..."
                    curl -fsSL https://ollama.com/install.sh | sh
                fi
            else
                info "Installing Ollama via official installer..."
                curl -fsSL https://ollama.com/install.sh | sh
            fi
        fi
    fi

    if command -v ollama &> /dev/null; then
        success "Found Ollama: $(ollama --version 2>&1 | head -n 1)"

        # Start Ollama service if not running
        if ! curl -s http://127.0.0.1:11434/api/tags &> /dev/null; then
            info "Starting Ollama background service..."
            if [ "$OS" = "Darwin" ] && command -v brew &> /dev/null; then
                brew services start ollama || true
            else
                nohup ollama serve > /dev/null 2>&1 &
            fi
            sleep 2
        fi

        # Pull model if not present
        if curl -s http://127.0.0.1:11434/api/tags | grep -q "$MODEL_NAME"; then
            success "Model '$MODEL_NAME' is already installed in Ollama."
        else
            info "Pulling model '$MODEL_NAME'..."
            ollama pull "$MODEL_NAME"
            success "Model '$MODEL_NAME' is ready."
        fi
    fi
fi

# ------------------------------------------------------------------------------
# Step 7: Configuration & Examples Setup
# ------------------------------------------------------------------------------
CONFIG_DIR="$HOME/.ailang"
mkdir -p "$CONFIG_DIR"

if [ ! -f "$CONFIG_DIR/config.json" ]; then
    cat << EOF > "$CONFIG_DIR/config.json"
{
  "backend": "auto",
  "model": "$MODEL_NAME",
  "api_key": null,
  "endpoint": "http://127.0.0.1:11434",
  "command": null,
  "max_retries": 3,
  "timeout_secs": 90,
  "verbose": false,
  "show_rust": false,
  "keep_temp": false
}
EOF
    success "Created configuration at $CONFIG_DIR/config.json"
fi

if [ -d "$SCRIPT_DIR/examples" ]; then
    cp -R "$SCRIPT_DIR/examples" "$CONFIG_DIR/"
    success "Copied sample scripts to $CONFIG_DIR/examples"
fi

# ------------------------------------------------------------------------------
# Step 8: Diagnostics & Self-Test
# ------------------------------------------------------------------------------
printf "\n"
info "Running self-test diagnostics..."
"$INSTALL_PREFIX/ailang" check || true

printf "\n${GREEN}${BOLD}========================================================================${RESET}\n"
printf "${GREEN}${BOLD}  ✓ Ailang Installation Complete!${RESET}\n"
printf "${GREEN}${BOLD}========================================================================${RESET}\n\n"

echo "Try running:"
echo "  ailang repl                    # Start the interactive interpreter REPL"
echo "  ailang eval \"calculate 12 * 12\" # Evaluate single instruction"
echo "  ailang check                   # View environment diagnostics"
echo ""
echo "Sample scripts available at: $CONFIG_DIR/examples"
echo "To uninstall anytime, run: ./install.sh --uninstall"
echo ""
