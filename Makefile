build:
	cargo build --color=always --package ruroco

release:
	cargo build --color=always --release --package ruroco

test:
	cargo test -- --test-threads=1

install: release
	sudo cp ./target/release/server /usr/local/bin/ruroco-server
	sudo cp ./target/release/commander /usr/local/bin/ruroco-commander

	sudo useradd --system ruroco --shell /bin/false
	sudo cp ./systemd/* /etc/systemd/system

	sudo systemctl daemon-reload

	echo "Please check ruroco.service, ruroco.socket and ruroco-commander.service and configure them accordingly"

test_end_to_end: build
	./target/debug/client gen -k 1024

	mkdir /tmp/ruroco
	cp ./target/debug/server /tmp/ruroco/server
	cp ./target/debug/commander /tmp/ruroco/commander

	mv ./ruroco_private.pem /tmp/ruroco

	sudo mkdir /etc/ruroco
	sudo mv ./ruroco_public.pem /etc/ruroco
	sudo chmod 400 /etc/ruroco/ruroco_public.pem

	sudo chown -R ruroco:ruroco /tmp/ruroco
	sudo chown -R ruroco:ruroco /etc/ruroco

	sudo cp ./systemd/* /run/systemd/system

	sudo sed -i "s@/usr/local/bin/ruroco-server@/tmp/ruroco/server@g" /run/systemd/system/ruroco.service

	sudo sed -i "s@/usr/local/bin/ruroco-commander@/tmp/ruroco/commander@g" /run/systemd/system/ruroco-commander.service
	sudo sed -i 's@echo "start"@touch /tmp/ruroco/start.test@g' /run/systemd/system/ruroco-commander.service
	sudo sed -i 's@echo "stop"@touch /tmp/ruroco/stop.test@g' /run/systemd/system/ruroco-commander.service
	sudo sed -i "s@--sleep 5@--sleep 1@g" /run/systemd/system/ruroco-commander.service

	sudo systemctl daemon-reload
	sudo systemctl start ruroco-commander.service
	sudo systemctl start ruroco.service

	./target/debug/client send -a 127.0.0.1:80 -p /tmp/ruroco/ruroco_private.pem

	sleep 2

	test -f "/tmp/ruroco/start.test"
	test -f "/tmp/ruroco/stop.test"

	sudo systemctl stop ruroco-commander.service
	sudo systemctl stop ruroco.service
	sudo systemctl daemon-reload

	sudo rm -rf /tmp/ruroco
	sudo rm -rf /etc/ruroco
	sudo rm /run/systemd/system/ruroco-commander.service /run/systemd/system/ruroco.service /run/systemd/system/ruroco.socket