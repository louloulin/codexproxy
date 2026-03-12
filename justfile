# OpenAI Proxy - Justfile for task automation

# Build the release binary
build:
    cargo build --release

# Run the service in development mode
run:
    cargo run

# Run tests
test:
    cargo test

# Run linter
check:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Clean build artifacts
clean:
    cargo clean

# Watch mode for development
watch:
    cargo watch -x run

# Build and run in release mode
start: build
    @echo "Starting OpenAI Proxy on port 8080..."
    ./target/release/openai-proxy

# Install just (if not already installed)
install-just:
    @echo "Install just: https://github.com/casey/just"
    cargo install just

# Default recipe shows available commands
default:
    @just --list
