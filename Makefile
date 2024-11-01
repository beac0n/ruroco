print_commits:
	git --no-pager log $$(git tag --sort=-version:refname | head -n 2 | tail -1)..$$(git tag --sort=-version:refname | head -n 1) --oneline

hooks:
	echo "#!/usr/bin/env bash" > .git/hooks/pre-push
	echo "cargo fmt && cargo clippy --fix" >> .git/hooks/pre-push
	chmod +x .git/hooks/pre-push

build:
	cargo build --color=always --package ruroco --target x86_64-unknown-linux-gnu

release_android:
	x build --features release-build --features android-build --release --platform android --arch arm64 --format apk --verbose

release: release_android
	# see https://github.com/johnthagen/min-sized-rust
	cargo build --color=always --release --package ruroco --features release-build --target x86_64-unknown-linux-gnu
	upx --best --lzma target/x86_64-unknown-linux-gnu/release/client_ui
	upx --best --lzma target/x86_64-unknown-linux-gnu/release/client
	upx --best --lzma target/x86_64-unknown-linux-gnu/release/server
	upx --best --lzma target/x86_64-unknown-linux-gnu/release/commander

test:
	cargo test -- --test-threads=1

format:
	cargo fmt && cargo clippy --fix

install_client: release
	sudo cp ./target/x86_64-unknown-linux-gnu/release/client /usr/local/bin/ruroco-client
	sudo cp ./target/x86_64-unknown-linux-gnu/release/client_ui /usr/local/bin/ruroco-client-ui

install_server: release
	sudo cp ./target/x86_64-unknown-linux-gnu/release/server /usr/local/bin/ruroco-server
	sudo cp ./target/x86_64-unknown-linux-gnu/release/commander /usr/local/bin/ruroco-commander
	sudo cp ./target/x86_64-unknown-linux-gnu/release/client /usr/local/bin/ruroco-client

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
