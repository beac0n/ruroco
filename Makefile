print_commits:
	git --no-pager log $$(git tag --sort=-version:refname | head -n 2 | tail -1)..$$(git tag --sort=-version:refname | head -n 1) --oneline

hooks:
	echo "#!/usr/bin/env bash" > .git/hooks/pre-push
	echo "cargo fmt && cargo clippy --fix" >> .git/hooks/pre-push
	chmod +x .git/hooks/pre-push

dev_ui_local:
	cargo run --bin client_ui

dev_ui_android:
	nix-shell nix/android.nix --pure --run ./scripts/dev_ui_android.sh

build:
	cargo build --color=always --package ruroco --target x86_64-unknown-linux-gnu

clean:
	rm -rf target
	rm -rf nix/.nix-*

release: release_android release_linux

release_linux:
	./scripts/release_linux.sh

release_linux_nix:
	nix-shell nix/linux.nix --pure --run ./scripts/release_linux.sh

release_android:
	nix-shell nix/android.nix --pure --run ./scripts/release_android.sh

coverage:
	cargo tarpaulin --timeout 360 --out xml -- --test-threads 1

test:
	export TEST_UPDATER=1; cargo nextest run --retries 2

check:
	cargo check --locked --verbose && cargo check --locked --no-default-features --verbose

format:
	cargo fix && cargo fmt && cargo clippy --tests --verbose -- -D warnings

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
