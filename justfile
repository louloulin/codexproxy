# OpenAI Proxy - Justfile
# Just command runner for rcodex
# Run `just --list` to see all available commands

# ============================================================
# Build & Run
# ============================================================

# Build debug binary
build:
    cargo build

# Build admin SPA (required before embedding)
build-admin:
    cd rcodex-admin && npm run build

# Build release binary
build-release:
    cargo build --release

# Build admin and release binary
build-all: build-admin build-release

# Run in debug mode
run:
    cargo run

# Run in release mode
run-release:
    cargo run --release

# Run frontend dev server
dev-frontend:
    cd rcodex-admin && npm run dev

# Run both backend and frontend (requires two terminals or background processes)
dev: build
    @echo "Starting backend on port 8788..."
    @echo "Frontend: run 'just dev-frontend' in another terminal"
    cargo run

# Watch mode (requires cargo-watch)
watch:
    cargo watch -x check -x test -x run

# Build and run release
start: build-release
    @echo "Starting rcodex on port 9080..."
    ./target/release/rcodex

# Stop running server
stop:
    pkill -f "rcodex" || true
    @echo "Server stopped"

# ============================================================
# Testing
# ============================================================

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run integration tests only
test-integration:
    cargo test --test integration_tests

# Run library unit tests
test-lib:
    cargo test --lib

# Run specific test by name
test-name NAME="test_health_check":
    cargo test {{NAME}}

# Run tests with logging
test-log:
    RUST_LOG=debug cargo test -- --nocapture

# ============================================================
# Code Quality
# ============================================================

# Check for errors without building
check:
    cargo check

# Clippy linter with warnings as errors
clippy:
    cargo clippy -- -D warnings

# Clippy all targets
clippy-all:
    cargo clippy --all-targets --all-features -- -W clippy::all

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# All pre-commit checks
pre-commit: fmt-check clippy test

# ============================================================
# Documentation
# ============================================================

# Generate documentation
doc:
    cargo doc --no-deps

# Open docs in browser
doc-open:
    cargo doc --no-deps --open

# Watch docs
doc-watch:
    cargo doc --no-deps --watch --open

# ============================================================
# Development Helpers
# ============================================================

# Run with custom config file
run-config FILE="config.yaml":
    CONFIG={{FILE}} cargo run

# Run on specific port
run-port PORT="9080":
    RUST_LOG=info cargo run

# Run with debug logging
debug:
    RUST_LOG=debug cargo run

# Run with trace logging (most verbose)
trace:
    RUST_LOG=trace cargo run

# Run with json logging (production)
prod:
    RUST_LOG=json cargo run --release

# Test specific endpoint
curl-health:
    curl -s http://localhost:9080/health

# Test chat completions (mock)
curl-chat:
    curl -X POST http://localhost:9080/v1/chat/completions \
      -H "Content-Type: application/json" \
      -d '{"model":"glm-4","messages":[{"role":"user","content":"Hello"}]}'

# Test responses API (mock)
curl-responses:
    curl -X POST http://localhost:9080/v1/responses \
      -H "Content-Type: application/json" \
      -d '{"model":"glm-4","input":[]}'

# ============================================================
# Maintenance
# ============================================================

# Clean build artifacts
clean:
    cargo clean

# Remove target completely
clean-all: clean
    rm -rf target

# Update dependencies
update:
    cargo update

# Update specific dependency
update-dep DEP="axum":
    cargo update {{DEP}}

# Audit dependencies for vulnerabilities
audit:
    cargo audit || true

# List dependencies
deps:
    cargo tree

# List top-level deps only
deps-top:
    cargo tree --depth 1

# Inverted dependency tree (what depends on X)
deps-of PKG="axum":
    cargo tree --invert {{PKG}}

# ============================================================
# Profiling & Benchmarks
# ============================================================

# Build with profiling
build-profile:
    cargo build --release --features profiling

# Check compile times
time:
    cargo build --timings

# ============================================================
# Release
# ============================================================

# Bump version (patch|minor|major)
bump KIND="patch":
    cargo release {{KIND}}

# Full release workflow
release: build-release test clippy
    @echo "Release binary: target/release/rcodex"
    @echo "Run `./target/release/rcodex` to start"

# ============================================================
# Docker (if applicable)
# ============================================================

# Build Docker image
docker-build:
    docker build -t rcodex:latest .

# Run in Docker
docker-run:
    docker run -p 9080:9080 -v $(pwd)/config.yaml:/app/config.yaml rcodex:latest

# ============================================================
# Utilities
# ============================================================

# Show all commands
help:
    @just --list --unsorted

# Show recipe details
help-full:
    @just --list

# Install just if not present
install-just:
    @echo "Installing just..."
    cargo install just

# Check if just is installed
check-just:
    @just --version

# ============================================================
# Default
# ============================================================

# Default recipe
default: build

