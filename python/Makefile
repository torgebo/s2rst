.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help (the default target)
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z][a-zA-Z0-9_-]*:.*## / {printf "  \033[36m%-13s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: typecheck
typecheck: ## Type-check the package (mypy) and verify the stub matches the runtime (mypy.stubtest)
	uv run mypy -p s2rst
	uv run python -m mypy.stubtest s2rst._core --ignore-positional-only --allowlist stubtest-allowlist.txt

.PHONY: test
test: ## Sync deps, lint/format-check (ruff), type-check (mypy + stubtest), and run pytest
	uv sync --extra test --group dev
	uv run ruff check .
	uv run ruff format --check .
	$(MAKE) typecheck
	uv run pytest
