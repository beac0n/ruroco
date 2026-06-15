#!/usr/bin/env bash
set -euxo pipefail

sudo useradd --system ruroco --shell /bin/false || true
RUROCO_KEY="$(./target/x86_64-unknown-linux-gnu/debug/client gen)"
echo "$RUROCO_KEY" > ruroco.key

# Binaries and the command output dir live under /opt, not /tmp: both units set PrivateTmp=true, so
# systemd replaces /tmp with a private tmpfs before exec — a binary under /tmp would not be found,
# and files written to /tmp would not be visible to the host assertions below.
sudo mkdir -p /opt/ruroco_test
sudo cp ./target/x86_64-unknown-linux-gnu/debug/server /opt/ruroco_test/server
sudo cp ./target/x86_64-unknown-linux-gnu/debug/commander /opt/ruroco_test/commander

sudo mkdir /etc/ruroco
sudo mv ./ruroco.key /etc/ruroco
sudo cp ./tests/files/config_end_to_end.toml /etc/ruroco/config.toml
sudo cp ./tests/files/commands_end_to_end.toml /etc/ruroco/commands.toml
sudo chmod 400 /etc/ruroco/ruroco.key

sudo chown -R ruroco:ruroco /opt/ruroco_test
sudo chown -R ruroco:ruroco /etc/ruroco

# commands.toml is read only by commander (root); the unprivileged server user must not be able to read the command set.
# Re-restrict it after the recursive chown.
sudo chown root:root /etc/ruroco/commands.toml
sudo chmod 600 /etc/ruroco/commands.toml

# blocklist (StateDirectory -> /var/lib/ruroco) and the socket (RuntimeDirectory -> /run/ruroco)
# are created by systemd from the unit files; /etc/ruroco stays read-only.
sudo cp ./systemd/* /run/systemd/system

sudo sed -i "s@/usr/local/bin/ruroco-server@/opt/ruroco_test/server@g" /run/systemd/system/ruroco.service
sudo sed -i "s@/usr/local/bin/ruroco-commander@/opt/ruroco_test/commander@g" /run/systemd/system/ruroco-commander.service

sudo systemctl daemon-reload
sudo systemctl start ruroco-commander.service
sudo systemctl start ruroco.service

# Wait for the server to finish Server::create() and seed the blocklist floor.
# Type=simple returns as soon as the process is forked; without this sleep the
# client can call now_nanos() before the server sets its floor, causing the
# counter to be rejected as "on blocklist".
sleep 1

./target/x86_64-unknown-linux-gnu/debug/client send -a 127.0.0.1:80 -k "$RUROCO_KEY"

sleep 2

test -f "/opt/ruroco_test/start.test"
test -f "/opt/ruroco_test/stop.test"
