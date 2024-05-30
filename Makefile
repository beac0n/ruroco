build:
	RUROCO_SOCKET_DIR="/tmp/ruroco/" RUROCO_SOCKET_FILE_PATH="/tmp/ruroco/ruroco.socket" cargo build --color=always --package ruroco

release:
	RUROCO_SOCKET_DIR="/etc/ruroco/" RUROCO_SOCKET_FILE_PATH="/etc/ruroco/ruroco.socket" cargo build --color=always --release --package ruroco
	upx --best --lzma target/release/client
	upx --best --lzma target/release/server
	upx --best --lzma target/release/commander

test:
	RUROCO_SOCKET_DIR="/tmp/ruroco/" RUROCO_SOCKET_FILE_PATH="/tmp/ruroco/ruroco.socket" cargo test -- --test-threads=1

install: release
	sudo cp ./target/release/server /usr/local/bin/ruroco-server
	sudo cp ./target/release/commander /usr/local/bin/ruroco-commander

	sudo useradd --system ruroco --shell /bin/false
	sudo cp ./systemd/* /etc/systemd/system

	sudo systemctl daemon-reload

	echo "Please check ruroco.service, ruroco.socket and ruroco-commander.service and configure them accordingly"

test_end_to_end: release
	./target/release/client gen -k 1024

	mkdir /tmp/ruroco_test
	cp ./target/release/server /tmp/ruroco_test/server
	cp ./target/release/commander /tmp/ruroco_test/commander

	mv ./ruroco_private.pem /tmp/ruroco_test

	sudo mkdir /etc/ruroco
	sudo mv ./ruroco_public.pem /etc/ruroco
	sudo chmod 400 /etc/ruroco/ruroco_public.pem

	sudo chown -R ruroco:ruroco /tmp/ruroco_test
	sudo chown -R ruroco:ruroco /etc/ruroco

	sudo cp ./systemd/* /run/systemd/system

	sudo sed -i "s@/usr/local/bin/ruroco-server@/tmp/ruroco_test/server@g" /run/systemd/system/ruroco.service

	sudo sed -i "s@/usr/local/bin/ruroco-commander@/tmp/ruroco_test/commander@g" /run/systemd/system/ruroco-commander.service
	sudo sed -i 's@echo "start"@touch /tmp/ruroco_test/start.test@g' /run/systemd/system/ruroco-commander.service
	sudo sed -i 's@echo "stop"@touch /tmp/ruroco_test/stop.test@g' /run/systemd/system/ruroco-commander.service
	sudo sed -i "s@--sleep 5@--sleep 1@g" /run/systemd/system/ruroco-commander.service

	sudo systemctl daemon-reload
	sudo systemctl start ruroco-commander.service
	sudo systemctl start ruroco.service

	./target/release/client send -a 127.0.0.1:80 -p /tmp/ruroco_test/ruroco_private.pem

	sleep 2

	test -f "/tmp/ruroco_test/start.test"
	test -f "/tmp/ruroco_test/stop.test"

	sudo systemctl stop ruroco-commander.service
	sudo systemctl stop ruroco.service
	sudo systemctl daemon-reload

	sudo rm -rf /tmp/ruroco_test
	sudo rm -rf /etc/ruroco
	sudo rm /run/systemd/system/ruroco-commander.service /run/systemd/system/ruroco.service /run/systemd/system/ruroco.socket