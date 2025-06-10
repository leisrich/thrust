#!/bin/bash
# Build script for Thrustmaster to G29 Protocol Translator

set -e

echo "🔧 Building Thrustmaster to G29 Protocol Translator..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo not found. Please install Rust first:"
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Print Rust version
echo "📦 Using Rust version: $(rustc --version)"

# Build the project
echo "🔨 Building in release mode..."
cargo build --release

# Run tests
echo "🧪 Running tests..."
cargo test

# Check formatting
echo "🎨 Checking code formatting..."
cargo fmt --all -- --check || {
    echo "⚠️  Code formatting issues found. Run 'cargo fmt' to fix."
}

# Run lints
echo "📋 Running clippy lints..."
cargo clippy --all-targets --all-features -- -D warnings || {
    echo "⚠️  Clippy found issues. Please fix them before release."
}

echo "✅ Build completed successfully!"
echo ""
echo "📁 Binary location: ./target/release/tm-g29"
echo "🚀 To run: ./target/release/tm-g29 --help"
echo ""
echo "Quick start:"
echo "  1. ./target/release/tm-g29 config"
echo "  2. ./target/release/tm-g29 discover"  
echo "  3. ./target/release/tm-g29 run" 