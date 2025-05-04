# Set project-wide variables

command_name := "toggle"
args := " "


# Text colors
BLACK := '\033[30m'
RED := '\033[31m'
GREEN := '\033[32m'
YELLOW := '\033[33m'
BLUE := '\033[34m'
MAGENTA := '\033[35m'
CYAN := '\033[36m'
WHITE := '\033[37m'
GRAY := '\033[90m'

# Background colors
BG_BLACK := '\033[40m'
BG_RED := '\033[41m'
BG_GREEN := '\033[42m'
BG_YELLOW := '\033[43m'
BG_BLUE := '\033[44m'
BG_MAGENTA := '\033[45m'
BG_CYAN := '\033[46m'
BG_WHITE := '\033[47m'

# Text styles
BOLD := '\033[1m'
DIM := '\033[2m'
ITALIC := '\033[3m'
UNDERLINE := '\033[4m'

# Reset all styles
NC := '\033[0m'

# Display a symbol
CHECK := "$(GREEN)✓$(NC)"
CROSS := "$(RED)✗$(NC)"
DASH := "$(GRAY)-$(NC)"


# List all available recipes
@default:
    just --list --unsorted

# Check if required tools are installed
[group('check')]
@check-deps:
    @#!/usr/bin/env sh
    if ! command -v just >/dev/null 2>&1; then echo "just is not installed"; exit 1; fi
    if ! command -v cargo >/dev/null 2>&1; then echo "cargo is not installed"; exit 1; fi
    if ! command -v rustc >/dev/null 2>&1; then echo "rust is not installed"; exit 1; fi
    echo "All required tools are installed"

alias c := check-deps

# Runs a complete dev cycle: formats code, runs linter, executes tests, builds the app
[group('dev'), group('quick start')]
@dev:
    just format
    just lint
    just test
    just build
    # just run

# Alias for dev (full developer cycle: format → lint → test → build)
alias cycle := dev


# Format code
[group('check'), group('quick start')]
@format:
    echo "Running formatter..."
    echo "  rustfmt"
    cargo fmt --all

alias f := format

# Run linter (code style and quality checks)
[group('check')]
@lint:
    echo "Running linter..."
    echo "  clippy"
    cargo clippy -- -D warnings

alias l := lint


# Run tests
[group('check')]
@test *options:
    cargo test {{options}}

alias t := test

# Run benchmarks
[group('check')]
@bench:
    cargo bench

alias be := bench

# Run all checks
[group('check'), group('quick start')]
@check: test lint
    echo "All checks passed!"

alias ca := check

# Run debug package command.
[group('run'), group('quick start')]
@run-debug *args=args:
    cargo run -- {{args}}

alias rd := run-debug

# Run release package command.
[group('run')]
@run-release *args=args:
    cargo run --release -- {{args}}

alias rr := run-release

# Build package
[group('build')]
@build: check
    cargo build

alias b := build

# Set up pre-commit hooks
[group('pre-commit')]
@pre-commit-setup:
    cargo install cargo-husky
    cargo husky install


# Run all pre-commit Hooks
[group('pre-commit')]
@pre-commit-run:
    cargo husky run --all

alias pc := pre-commit-run

# Check installed package version
[group('check')]
@version cmd=command_name:
    {{cmd}} --version

# Clean up temporary files and caches
[group('clean'), group('quick start')]
@clean:
    cargo clean
    rm -rf target/
    rm -rf Cargo.lock
    rm -rf **/*.rs.bk
    rm -rf .cargo-cache/
    rm -rf .rustc_info.json


# Install Sphinx and any necessary extensions
[group('docs')]
@install-docs:
    @#!/usr/bin/env sh
    if ! command -v cargo >/dev/null 2>&1; then echo "cargo is not installed"; exit 1; fi
    echo "Installing mdBook..."
    cargo install mdbook
    echo "Installing required mdBook components..."
    cargo install mdbook-linkcheck
    cargo install mdbook-mermaid
    echo "{{GREEN}} Documentation dependencies installed"

# Not usually needed, Initialize docs only if you are starting a new project
[group('docs')]
@init-docs:
    cargo doc --document-private-items --open


# Show help for documentation
[group('docs')]
@docs-help:
    cargo doc --help

# Build documentation
[group('docs')]
@docs target:
    @#!/usr/bin/env sh
    if ! command -v mdbook >/dev/null 2>&1; then \
        echo "mdBook is not installed. Run 'just install-docs' first"; \
        exit 1; \
    fi
    echo "Building documentation..."
    mdbook build docs
    echo "{{GREEN}}Documentation built successfully{{NC}}"

# Run documentation server with hot reloading
[group('docs'), group('quick start')]
@docs-dev:
    cargo doc --document-private-items --open --watch

# Clean documentation build files
[group('docs')]
@docs-clean:
    cargo clean
    rm -rf docs/build
    rm -rf docs/source

# Build release version and install locally
[group('build')]
@build-release:
    cargo build --release
    @echo "{{GREEN}}Built release binary at target/release/{{command_name}}{{NC}}"
    @echo "To make it available system-wide, copy it to a directory in your PATH:"
    @echo "cp target/release/{{command_name}} ~/.local/bin/ # or another directory in your PATH"

alias br := build-release

# Run the simple Python test case
[group('test')]
@test-python:
    @echo "Running Python test case..."
    ./tests/test_simple_python.sh

alias tp := test-python


# Help for Task Master
[group('help')]
taskmaster-help:
    @echo "Help Task Master"
    task-master --help

# Docs for Task Master
[group('docs')]
taskmaster-docs:
    @echo "Task Master Docs"
    open "https://github.com/eyaltoledano/claude-task-master"

# Version of Task Master
[group('version')]
taskmaster-version:
    @echo "Task Master Version"
    task-master --version

# Print Task Master quickstart commands
[group('help')]
taskmaster-quickstart:
    @echo "Task Master Quickstart Commands:"
    @echo "# Initialize a new project"
    @echo "task-master init"
    @echo ""
    @echo "# Parse a PRD and generate tasks"
    @echo "task-master parse-prd your-prd.txt"
    @echo ""
    @echo "# List all tasks"
    @echo "task-master list"
    @echo ""
    @echo "# Show the next task to work on"
    @echo "task-master next"
    @echo ""
    @echo "# Generate task files"
    @echo "task-master generate"