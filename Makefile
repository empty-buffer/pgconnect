.PHONY: build install uninstall clean test run compose_up compose_down compose_teardown

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

run:
	cargo run

# Install to user's cargo bin directory (no sudo required)
install-user:
	cargo install --path . --force

compose_up:
	docker compose -f deploy/docker-compose.yml up -d

compose_down:
	docker compose -f deploy/docker-compose.yml down

compose_teardown:
	docker compose -f deploy/docker-compose.yml down -v
