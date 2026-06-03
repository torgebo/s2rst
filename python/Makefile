.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help (the default target)
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z][a-zA-Z0-9_-]*:.*## / {printf "  \033[36m%-13s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: test
test: ## Sync deps, lint/format-check with ruff, and run the pytest suite
	uv sync --extra test --group dev
	uv run ruff check .
	uv run ruff format --check .
	uv run pytest
