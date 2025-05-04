SHELL := /bin/zsh

# Text colors
BLACK := \033[30m
RED := \033[31m
GREEN := \033[32m
YELLOW := \033[33m
BLUE := \033[34m
MAGENTA := \033[35m
CYAN := \033[36m
WHITE := \033[37m
GRAY := \033[90m

# Background colors
BG_BLACK := \033[40m
BG_RED := \033[41m
BG_GREEN := \033[42m
BG_YELLOW := \033[43m
BG_BLUE := \033[44m
BG_MAGENTA := \033[45m
BG_CYAN := \033[46m
BG_WHITE := \033[47m

# Text styles
BOLD := \033[1m
DIM := \033[2m
ITALIC := \033[3m
UNDERLINE := \033[4m

# Reset
NC := \033[0m

CHECK := $(GREEN)✓$(NC)
CROSS := $(RED)✗$(NC)
DASH := $(GRAY)-$(NC)

.PHONY: all build test bench clean format lint

# Default target
all: test build

# Build the project
build:
	cargo build

# Run tests
test:
	cargo test

# Run benchmarks
bench:
	cargo bench

# Clean build artifacts
clean:
	cargo clean

# Format code
format:
	cargo fmt

# Run linter
lint:
	cargo clippy -- -D warnings

# Release build
release:
	cargo build --release

## `make check` Example Output

### Success case 
# Checking dependencies...
# === System Requirements Status ===
# [✓] Just
# All dependencies are installed!

### Failure case
# Checking dependencies...
# === System Requirements Status ===
# [✓] just

# Found 1 missing deps: uv 
# make: *** [check] Error 1

check: ## Check system requirements
	@echo "Checking dependencies..."
	@echo "=== System Requirements Status ==="
	@ERROR_COUNT=0; \
	CHECK_CMD_NAME="just"; \
	CHECK_CMD_INSTALL="install-just"; \
	if [ $(shell command -v just >/dev/null 2>&1 && echo "0" || echo "1" ) -eq 0 ] ; then \
		printf "[$(CHECK)] $${CHECK_CMD_NAME}\n"; \
	else \
		printf "[$(CROSS)] $${CHECK_CMD_NAME} ($(GREEN)make $${CHECK_CMD_INSTALL}$(NC))\n"; \
		ERROR_COUNT=$$((ERROR_COUNT + 1)); \
		MISSING_DEPS="$${CHECK_CMD_NAME}$${MISSING_DEPS:+,} $${MISSING_DEPS}"; \
	fi; \
	if [ "$${ERROR_COUNT}" = "0" ]; then \
		echo -e "$(GREEN)All dependencies are installed!$(NC)"; \
	else \
		echo ""; \
		echo -e "$(RED)Found $$ERROR_COUNT missing deps: $${MISSING_DEPS}$(NC)"; \
		exit 1; \
	fi

install-just: ## Print install just command and where to find install options
	@echo "just installation command:"
	@echo -e "${CYAN}curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to ~/bin${NC}"
	@echo "NOTE:change ~/bin to the desired installation directory"
	@echo "Find other install options here: https://github.com/casey/just"
	@echo -e "To setup just PATH, run: ${YELLOW}SET_PATH=$(HOME)/bin make set-path${NC}"

set-path: ## Add SET_PATH to PATH in .zshenv if not already present
	@if [ -z "$(SET_PATH)" ]; then \
		echo -e "$(RED)Error: SET_PATH must be set$(NC)"; \
		echo -e "Usage: $(BLUE)make test2 SET_PATH=/your/path$(NC)"; \
		exit 1; \
	fi; \
	if ! awk -v path="$(SET_PATH)" ' \
		BEGIN {found=0} \
		/^export PATH=/ { \
			if (index($$0, path) > 0) { \
				found=1; \
				exit; \
			} \
		} \
		END {exit !found}' "$(HOME)/.zshenv"; then \
		echo "export PATH=\"\$$PATH:$(SET_PATH)\"" >> "$(HOME)/.zshenv"; \
		echo -e "$(GREEN)Added PATH entry:$(NC) \$$PATH:$(SET_PATH)"; \
		echo -e "Run $(BLUE)source $(HOME)/.zshenv$(NC) to apply changes"; \
	else \
		echo -e "$(CHECK) PATH already contains $(SET_PATH)"; \
	fi

help: ## The help command - this command
	@echo ""
	@echo "Purpose of this Makefile:"
	@echo -e "  To make it easy to check for and install"
	@echo -e "  the main dependencies because almost everyone has $(GREEN)make$(NC)"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "$(CYAN)%-30s$(NC) %s\n", $$1, $$2}' 
	@echo ""

