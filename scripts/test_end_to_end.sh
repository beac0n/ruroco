sudo useradd --system ruroco --shell /bin/false || true
RUROCO_KEY="$(./target/x86_64-unknown-linux-gnu/debug/client gen)"
echo "$RUROCO_KEY" > ruroco.key

mkdir /tmp/ruroco_test
cp ./target/x86_64-unknown-linux-gnu/debug/server /tmp/ruroco_test/server
cp ./target/x86_64-unknown-linux-gnu/debug/commander /tmp/ruroco_test/commander

sudo mkdir /etc/ruroco
sudo mv ./ruroco.key /etc/ruroco
sudo cp ./tests/files/config_end_to_end.toml /etc/ruroco/config.toml
sudo chmod 400 /etc/ruroco/ruroco.key

sudo chown -R ruroco:ruroco /tmp/ruroco_test
sudo chown -R ruroco:ruroco /etc/ruroco

sudo cp ./systemd/* /run/systemd/system

sudo sed -i "s@/usr/local/bin/ruroco-server@/tmp/ruroco_test/server@g" /run/systemd/system/ruroco.service
sudo sed -i "s@/usr/local/bin/ruroco-commander@/tmp/ruroco_test/commander@g" /run/systemd/system/ruroco-commander.service

sudo systemctl daemon-reload
sudo systemctl start ruroco-commander.service
sudo systemctl start ruroco.service

./target/x86_64-unknown-linux-gnu/debug/client send -a 127.0.0.1:80 -k "$RUROCO_KEY"

sleep 2

test -f "/tmp/ruroco_test/start.test"
test -f "/tmp/ruroco_test/stop.test"