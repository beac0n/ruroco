print_commits:
	git --no-pager log $$(git tag --sort=-version:refname | head -n 2 | tail -1)..$$(git tag --sort=-version:refname | head -n 1) --oneline

hooks:
	echo "#!/usr/bin/env bash" > .git/hooks/pre-push
	echo "cargo fmt && cargo clippy --fix" >> .git/hooks/pre-push
	chmod +x .git/hooks/pre-push

dev_ui_local:
	cargo run --bin client_ui

dev_ui_android:
	nix-shell android.nix --pure --run ./scripts/dev_ui_android.sh

build:
	nix-shell linux.nix --pure --run 'cargo build --color=always --package ruroco --target x86_64-unknown-linux-gnu'

release: release_android release_linux

release_linux:
	nix-shell linux.nix --pure --run ./scripts/release_linux.sh

release_android:
	nix-shell android.nix --pure --run ./scripts/release_android.sh

coverage:
	nix-shell linux.nix --pure --run 'cargo tarpaulin --timeout 360 --out xml -- --test-threads 1'

test:
	nix-shell linux.nix --pure --run 'export TEST_UPDATER=1; cargo nextest run --retries 2'
	rm -rf *.pem

format:
	nix-shell linux.nix --pure --run 'cargo fmt && cargo clippy --tests --verbose -- -D warnings'

install_client: release
	mkdir -p ~/.local/bin/
	cp ./target/x86_64-unknown-linux-gnu/release/client ~/.local/bin/ruroco-client
	cp ./target/x86_64-unknown-linux-gnu/release/client_ui ~/.local/bin/ruroco-client-ui

install_server: install_client
	sudo cp ./target/x86_64-unknown-linux-gnu/release/server /usr/local/bin/ruroco-server
	sudo cp ./target/x86_64-unknown-linux-gnu/release/commander /usr/local/bin/ruroco-commander

	sudo useradd --system ruroco --shell /bin/false || true
	sudo cp ./systemd/* /etc/systemd/system
	sudo cp ./config/config.toml /etc/ruroco/config.toml

	sudo chmod 400 /etc/ruroco/config.toml
	sudo chmod 500 /usr/local/bin/ruroco-server
	sudo chmod 100 /usr/local/bin/ruroco-commander
	sudo chown ruroco:ruroco /usr/local/bin/ruroco-server

	sudo systemctl daemon-reload

	sudo systemctl enable ruroco.service
	sudo systemctl enable ruroco-commander.service
	sudo systemctl enable ruroco.socket

	sudo systemctl start ruroco-commander.service
	sudo systemctl start ruroco.socket
	sudo systemctl start ruroco.service

	echo "##### installation complete #####"
	echo "# gen pub and priv pem files with 'ruroco-client gen' on client"
	echo "# move pub pem to /etc/ruroco/ruroco_public.pem on server"
	echo "# save priv pem to ~/.config/ruroco/ruroco_private.pem on client"
	echo "# update config /etc/ruroco/config.toml on server"
	echo "# start service with sudo systemctl start ruroco.service on server"

test_end_to_end: clean_test_end_to_end build
	sudo useradd --system ruroco --shell /bin/false || true
	./target/x86_64-unknown-linux-gnu/debug/client gen -k 4096 -r ruroco_private.pem -u ruroco_public.pem

	mkdir /tmp/ruroco_test
	cp ./target/x86_64-unknown-linux-gnu/debug/server /tmp/ruroco_test/server
	cp ./target/x86_64-unknown-linux-gnu/debug/commander /tmp/ruroco_test/commander

	mv ./ruroco_private.pem /tmp/ruroco_test

	sudo mkdir /etc/ruroco
	sudo mv ./ruroco_public.pem /etc/ruroco
	sudo cp ./tests/files/config_end_to_end.toml /etc/ruroco/config.toml
	sudo chmod 400 /etc/ruroco/ruroco_public.pem

	sudo chown -R ruroco:ruroco /tmp/ruroco_test
	sudo chown -R ruroco:ruroco /etc/ruroco

	sudo cp ./systemd/* /run/systemd/system

	sudo sed -i "s@/usr/local/bin/ruroco-server@/tmp/ruroco_test/server@g" /run/systemd/system/ruroco.service
	sudo sed -i "s@/usr/local/bin/ruroco-commander@/tmp/ruroco_test/commander@g" /run/systemd/system/ruroco-commander.service

	sudo systemctl daemon-reload
	sudo systemctl start ruroco-commander.service
	sudo systemctl start ruroco.service

	./target/x86_64-unknown-linux-gnu/debug/client send -a 127.0.0.1:80 -p /tmp/ruroco_test/ruroco_private.pem

	sleep 2

	test -f "/tmp/ruroco_test/start.test"
	test -f "/tmp/ruroco_test/stop.test"

	$(MAKE) clean_test_end_to_end

clean_test_end_to_end:
	sudo systemctl stop ruroco-commander.service || true
	sudo systemctl stop ruroco.service || true
	sudo systemctl daemon-reload || true

	sudo rm -rf /tmp/ruroco_test
	sudo rm -rf /etc/ruroco
	sudo rm -f /run/systemd/system/ruroco-commander.service /run/systemd/system/ruroco.service /run/systemd/system/ruroco.socket
