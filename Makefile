PREFIX ?= $(HOME)/.local/bin

.PHONY: build install uninstall setup

build:
	cargo build --release

install: build
	@mkdir -p $(PREFIX)
	@cp target/release/vision-squeezer $(PREFIX)/vision-squeezer
	@cp target/release/vision-squeezer-mcp $(PREFIX)/vision-squeezer-mcp
	@echo "✅ Installed to $(PREFIX)/"
	@echo ""
	@echo "Run 'vision-squeezer-mcp --setup' to get MCP config for your editor."

uninstall:
	@rm -f $(PREFIX)/vision-squeezer $(PREFIX)/vision-squeezer-mcp
	@echo "🗑  Uninstalled from $(PREFIX)/"

setup:
	@$(PREFIX)/vision-squeezer-mcp --setup
