print_commits:
	git --no-pager log $$(git tag --sort=-version:refname | head -n 2 | tail -1)..$$(git tag --sort=-version:refname | head -n 1) --oneline

hooks:
	echo "#!/usr/bin/env bash" > .git/hooks/pre-push
	echo "cargo fmt && cargo clippy --fix" >> .git/hooks/pre-push
	chmod +x .git/hooks/pre-push

build:
	cargo build --color=always --package ruroco

release:
	# see https://github.com/johnthagen/min-sized-rust
	cargo build --color=always --release --package ruroco
	upx --best --lzma target/release/client
	upx --best --lzma target/release/server
	upx --best --lzma target/release/commander

test:
	cargo test

format:
	cargo fmt && cargo clippy --fix

install_client: release
	sudo cp ./target/release/client /usr/local/bin/ruroco-client

install_server: release
	sudo cp ./target/release/server /usr/local/bin/ruroco-server
	sudo cp ./target/release/commander /usr/local/bin/ruroco-commander
	sudo cp ./target/release/client /usr/local/bin/ruroco-client

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

test_end_to_end: clean_test_end_to_end release
	sudo useradd --system ruroco --shell /bin/false || true
	./target/release/client gen -k 4096

	mkdir /tmp/ruroco_test
	cp ./target/release/server /tmp/ruroco_test/server
	cp ./target/release/commander /tmp/ruroco_test/commander

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

	./target/release/client send -a 127.0.0.1:80 -p /tmp/ruroco_test/ruroco_private.pem

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