#!/usr/bin/env bash
# Scientist-in-loop (sil) single-command installer script.
# Usage: curl -fsSL https://raw.githubusercontent.com/scientist-in-loop/scientist-in-loop/main/install.sh | bash

set -euo pipefail

echo "🔬 Installing scientist-in-loop (sil)..."

if command -v cargo >/dev/null 2>&1; then
    echo "⚙  Building from source using Cargo..."
    cargo install --path crates/sil --force
    echo "✓ Installed sil to $(which sil || echo '~/.cargo/bin/sil')"
else
    echo "⚠ Cargo not found. Please install Rust via https://rustup.rs or download prebuilt binaries from GitHub Releases."
    exit 1
fi

echo "🚀 Run 'sil project doctor' to verify setup."
