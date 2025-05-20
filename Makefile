.PHONY: install-service uninstall-service

main:
	cargo build --release

run:
	cargo run -- start

dev:
	cargo watch -x 'run -- start'

install:
	cargo install --path .

install-service: install
	@echo "Copying plist to LaunchAgents..."
	mkdir -p ~/Library/LaunchAgents
	cp scripts/com.sectorflabs.reservoir.plist ~/Library/LaunchAgents/com.sectorflabs.reservoir.plist
	launchctl unload -w ~/Library/LaunchAgents/com.sectorflabs.reservoir.plist || true
	launchctl load -w ~/Library/LaunchAgents/com.sectorflabs.reservoir.plist
	@echo "Service installed and started."

uninstall-service:
	@echo "Unloading and removing service..."
	launchctl unload -w ~/Library/LaunchAgents/com.sectorflabs.reservoir.plist || true
	rm -f ~/Library/LaunchAgents/com.sectorflabs.reservoir.plist
	@echo "Service removed."
