.DEFAULT_GOAL := help

.PHONY: help dev build test lint fmt fmt-check check

help: ## List available targets
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

dev: build ## Start the development environment (builds the crate)

build: ## Compile the crate
	cargo build

test: ## Run the test suite
	cargo test

lint: ## Lint with clippy, treating warnings as errors
	cargo clippy --all-targets -- -D warnings

fmt: ## Format the code
	cargo fmt

fmt-check: ## Check formatting without modifying files
	cargo fmt --check

check: lint fmt-check test ## Run all checks: lint, format check, tests
