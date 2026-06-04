print_commits:
	git --no-pager log $$(git tag --sort=-version:refname | head -n 2 | tail -1)..$$(git tag --sort=-version:refname | head -n 1) --oneline

dev_ui_local:
	cargo run --bin client_ui --features="with-gui"

dev_ui_android:
	nix-shell nix/android.nix --pure --run ./scripts/dev_ui_android.sh

build:
	cargo build --color=always --package ruroco --no-default-features --features with-client --bin client --target x86_64-unknown-linux-gnu
	cargo build --color=always --package ruroco --no-default-features --features with-gui --bin client_ui --target x86_64-unknown-linux-gnu
	cargo build --color=always --package ruroco --no-default-features --features with-server --bin server --target x86_64-unknown-linux-gnu
	cargo build --color=always --package ruroco --no-default-features --features with-server --bin commander --target x86_64-unknown-linux-gnu

.PHONY: docs docs_serve

docs:
	mdbook build docs

docs_serve:
	mdbook serve docs --open

clean:
	rm -rf target
	rm -rf nix/.nix-*

gen_signing_key:
	@if [ -f keys/ruroco-release-ed25519.key ]; then \
		echo "keys/ruroco-release-ed25519.key already exists - refusing to overwrite (remove it manually to regenerate)"; \
		exit 1; \
	fi
	mkdir -p keys
	openssl genpkey -algorithm ed25519 -out keys/ruroco-release-ed25519.key
	openssl pkey -in keys/ruroco-release-ed25519.key -pubout -out keys/ruroco-release-ed25519.pub.pem
	@echo ""
	@echo "Generated:"
	@echo "  keys/ruroco-release-ed25519.key      (private, gitignored - keep secret, back up offline)"
	@echo "  keys/ruroco-release-ed25519.pub.pem  (public, committed - embedded into the client)"
	@echo ""
	@echo "Next: add the private key as the RUROCO_SIGNING_KEY GitHub Actions secret:"
	@echo "  gh secret set RUROCO_SIGNING_KEY < keys/ruroco-release-ed25519.key"

release: release_android release_linux

release_linux:
	cargo build --color=always --release --package ruroco --no-default-features --features release-build,with-client --bin client --target x86_64-unknown-linux-gnu
	cargo build --color=always --release --package ruroco --no-default-features --features release-build,with-gui --bin client_ui --target x86_64-unknown-linux-gnu
	cargo build --color=always --release --package ruroco --no-default-features --features release-build,with-server --bin server --target x86_64-unknown-linux-gnu
	cargo build --color=always --release --package ruroco --no-default-features --features release-build,with-server --bin commander --target x86_64-unknown-linux-gnu

release_linux_nix:
	nix-shell nix/linux.nix --pure --run ./scripts/release_linux.sh

release_android:
	nix-shell nix/android.nix --pure --run ./scripts/release_android.sh

coverage:
	export TEST_UPDATER=1; cargo tarpaulin --features with-client,with-server,with-gui --timeout 360 --engine llvm --out xml --out html

test:
	export TEST_UPDATER=1; cargo nextest run --retries 2 --features with-client,with-server,with-gui

test_unit:
	cargo nextest run --retries 2 --features with-client,with-server,with-gui --filter-expr 'not binary(integration_test)'

test_integration:
	export TEST_UPDATER=1; cargo nextest run --retries 2 --features with-client,with-server,with-gui --filter-expr 'binary(integration_test)'

check:
	cargo check --locked --verbose && cargo check --locked --no-default-features --verbose

typos:
	command -v typos >/dev/null 2>&1 || cargo install typos-cli
	typos

format:
	cargo fmt && cargo clippy --tests --features with-client,with-server,with-gui --verbose -- -D warnings && cargo fix --allow-dirty --features with-client,with-server,with-gui

install_client: release
	mkdir -p ~/.local/bin/
	cp ./target/x86_64-unknown-linux-gnu/release/client ~/.local/bin/ruroco-client
	cp ./target/x86_64-unknown-linux-gnu/release/client_ui ~/.local/bin/ruroco-client-ui

install_server: install_client
	sudo cp ./target/x86_64-unknown-linux-gnu/release/server /usr/local/bin/ruroco-server
	sudo cp ./target/x86_64-unknown-linux-gnu/release/commander /usr/local/bin/ruroco-commander
	sudo ruroco-client wizard

test_end_to_end: clean_test_end_to_end build
	./scripts/test_end_to_end.sh
	$(MAKE) clean_test_end_to_end

clean_test_end_to_end:
	sudo systemctl stop ruroco-commander.service || true
	sudo systemctl stop ruroco.service || true
	sudo systemctl daemon-reload || true

	sudo rm -rf /tmp/ruroco_test
	sudo rm -rf /etc/ruroco
	sudo rm -f /run/systemd/system/ruroco-commander.service /run/systemd/system/ruroco.service /run/systemd/system/ruroco.socket
	rm -f ~/.config/ruroco/counter
