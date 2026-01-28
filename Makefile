.PHONY: build install uninstall clean test

# Installation directory (can be overridden: make install INSTALL_DIR=/usr/bin)
INSTALL_DIR ?= /usr/local/bin

build:
	cargo build --release

install: build
	@echo "Installing pgconnect to $(INSTALL_DIR)..."
	@mkdir -p $(INSTALL_DIR)
	@cp target/release/pgconnect $(INSTALL_DIR)/pgconnect
	@chmod +x $(INSTALL_DIR)/pgconnect
	@echo "✓ pgconnect installed successfully!"

uninstall:
	@echo "Removing pgconnect from $(INSTALL_DIR)..."
	@rm -f $(INSTALL_DIR)/pgconnect
	@echo "✓ pgconnect uninstalled"

clean:
	cargo clean

test:
	cargo test

# Install to user's cargo bin directory (no sudo required)
install-user:
	cargo install --path . --force
