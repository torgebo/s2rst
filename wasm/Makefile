.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help (the default target)
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z][a-zA-Z0-9_-]*:.*## / {printf "  \033[36m%-13s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: test
test: ## Run the wasm-bindgen test suite in Node (wasm-pack)
	wasm-pack test --node .
