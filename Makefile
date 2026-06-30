# Asistente Comercial — dev tasks
.DEFAULT_GOAL := help
.PHONY: help build release test fmt fmt-check clippy run up down logs smoke clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN{FS=":.*?## "}{printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Debug build
	cargo build

release: ## Optimized release build
	cargo build --release

test: ## Run tests (scoring rubric)
	cargo test

fmt: ## Format the codebase
	cargo fmt

fmt-check: ## Check formatting (CI)
	cargo fmt --check

clippy: ## Lint; warnings are errors
	cargo clippy --all-targets -- -D warnings

run: ## Run locally (needs .env + Postgres)
	cargo run

up: ## docker compose up (app + postgres)
	docker compose up --build -d

down: ## docker compose down
	docker compose down

logs: ## Tail the app logs
	docker compose logs -f app

smoke: ## Smoke-test the webhook (verify handshake; --inbound for full pipeline)
	./scripts/smoke.sh

clean: ## Remove build artifacts
	cargo clean
