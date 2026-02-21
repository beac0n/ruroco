[![build](https://github.com/beac0n/ruroco/actions/workflows/rust.yml/badge.svg)](https://github.com/beac0n/ruroco/actions)
[![release](https://img.shields.io/github/v/release/beac0n/ruroco?style=flat&labelColor=1C2C2E&color=C96329&logo=GitHub&logoColor=white)](https://github.com/beac0n/ruroco/releases)
[![codecov](https://codecov.io/gh/beac0n/ruroco/graph/badge.svg?token=H7ABBHYYWT)](https://codecov.io/gh/beac0n/ruroco)

# ruroco - Run Remote Command

ruroco is a tool that lets you execute commands on a server by sending UDP packets.

the tool consist of 4 binaries:

- `ruroco-client`: runs on your notebook/computer and sends the UDP packets
- `ruroco-client-ui`: presents most of the functionality of `ruroco-client` in an easier to use user interface.
- `ruroco-server`: receives the UDP packets and makes sure that they are valid
- `ruroco-commander`: runs the command encoded by the data of the UDP packet if it's valid

The commands are configured on the server side, so the client does not define what is going to be executed, it only
picks from existing commands.

## security

- client sends UDP packet to server, server never responds to it -> **port-scanning** does not help an adversary
- data sent from client to server is encrypted symmetrically with AES-256-GCM using a shared key
- client only defines command to execute, **commands are saved on server** -> client can pick command but not define it
- run server software in such a way so that it uses **as little operating system rights** as possible
- **replay protection** by adding every packet that the server received to a blocklist

## client ui usage

```shell
ruroco-client-ui
```

Use the Generate Key action (or `ruroco-client gen`) to produce a base64-encoded shared key. Copy that key into the
server’s `.key` files (see server config) and reuse the same string with `ruroco-client send`.

## client usage

### commands

```shell
ruroco-client
```

```text
Usage: ruroco-client <COMMAND>

Commands:
  gen     Generate a shared AES key (base64 with embedded key id)
  send    Send a command to a specific address
  update  Update the client binary
  wizard  Run the wizard to set up the server side
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

```

### gen

```shell
ruroco-client gen
```

```text
Generate a shared AES key (base64 with embedded key id)

Usage: ruroco-client gen

Options:
  -h, --help  Print help
```

### send

```shell
ruroco-client send --help
```

```text
Send a command to a specific address

Usage: ruroco-client send [OPTIONS] --address <ADDRESS> --key <KEY>

Options:
  -a, --address <ADDRESS>  Address to send the command to
  -k, --key <KEY>          Base64 key with id (output of `ruroco-client gen` or the UI)
  -c, --command <COMMAND>  Command to send [default: default]
  -e, --permissive         Allow permissive IP validation - source IP does not have to match provided IP
  -i, --ip <IP>            Optional IP address from which the command was sent. Use -6ei "dead:beef:dead:beef::/64" to allow you whole current IPv6 network. To do this automatically, use -6ei $(curl -s6 https://api64.ipify.org | awk -F: '{print $1":"$2":"$3":"$4"::/64"}')
  -4, --ipv4               Connect via IPv4
  -6, --ipv6               Connect via IPv6
  -h, --help               Print help
```

Pass the same base64 key string that you placed on the server. Example:

```shell
ruroco-client send -a 127.0.0.1:34020 -k "$(secret-tool lookup token ruroco)" -c default
```

## server usage

```shell
ruroco-server --help
```

```text
Usage: ruroco-server [OPTIONS]

Options:
  -c, --config <CONFIG>  [default: /etc/ruroco/config.toml]
  -h, --help             Print help
  -V, --version          Print version
```

## commander usage

```shell
ruroco-commander --help
```

```text
Usage: ruroco-commander [OPTIONS]

Options:
  -c, --config <CONFIG>  [default: /etc/ruroco/config.toml]
  -h, --help             Print help
  -V, --version          Print version
```

## server config

1. run `ruroco-client gen > ~/.config/ruroco/user.key` to create a shared base64 key (includes key id)
2. copy the same `.key` file to the server config dir (default `/etc/ruroco/user.key`); the server loads every `*.key`
   file there
3. add server config to `/etc/ruroco/config.toml` -> see [config.toml](config/config.toml)
4. call `ruroco-client send` with `-k "$(secret-tool lookup token ruroco)"` so client and server share the identical key

# setup

download binaries from the [releases page](https://github.com/beac0n/ruroco/releases) or build them yourself by running

```shell
make release
```

you can find the binaries in `target/release/client`, `target/release/client_ui`, `target/release/server` and
`target/release/commander`

## client

### self-build

See make goal `install_client`. This builds the project and copies the client binary to `/usr/local/bin/ruroco-client`

### pre-build

Run the following script

```shell
curl -Ls "$(curl -s https://api.github.com/repos/beac0n/ruroco/releases/latest | grep -oE 'https://[^"]*/client-v[0-9]+\.[0-9]+\.[0-9]+-x86_64-linux')" -o ~/.local/bin/ruroco-client 
chmod +x ~/.local/bin/ruroco-client
~/.local/bin/ruroco-client update --force
```

## server

### self-build

See make goal `install_server`, which

- Builds the project
- Copies the client binary to `~/.local/bin/`
- Copies the server binaries to `/usr/local/bin/`
- Runs `ruroco-client wizard`
- After running the make goal, you have to
    - generate a shared `.key` file and copy it to the right place
    - setup the `config.toml`

### pre-build

Run the following script

```shell
curl -Ls "$(curl -s https://api.github.com/repos/beac0n/ruroco/releases/latest | grep -oE 'https://[^"]*/client-v[0-9]+\.[0-9]+\.[0-9]+-x86_64-linux')" -o ~/.local/bin/ruroco-client 
chmod +x ~/.local/bin/ruroco-client
~/.local/bin/ruroco-client update --force
sudo ~/.local/bin/ruroco-client wizard
```

## android

See `nix/android.nix`, `scripts/dev_ui_android.sh` and `scripts/release_android.sh`

# use cases

## single packet authorization (SPA)

If you host a server on the web, you know that you'll get lots of brute-force attacks on (at least) the SSH port of your
server. While using good practices in securing your server will keep you safe from such attacks, these attacks are quite
annoying (filling up logs) and even if you secured your server correctly, you will still not be 100% safe, see
https://www.schneier.com/blog/archives/2024/04/xz-utils-backdoor.html or
https://www.qualys.com/2024/07/01/cve-2024-6387/regresshion.txt

Completely blocking all traffic to all ports that do not have to be open at all times can reduce the attack surface.
But blocking the SSH port completely will make SSH unusable for that server.

This is where ruroco comes in. Ruroco can execute a command that opens up the SSH port for just a short amount of time,
so that you can ssh into your server. Afterward ruruco closes the SSH port again. To implement this use case with
ruroco, you have to use a configuration similar to the one shown below:

```toml
# see chapter "server config"
[commands]
open_ssh = "ufw allow from $RUROCO_IP proto tcp to any port 22"         #  open ssh for IP where request came from
close_ssh = "ufw delete allow from $RUROCO_IP proto tcp to any port 22" # close ssh for IP where request came from
```

If you have configured ruroco on server like that and execute the following client side command

```shell
ruroco-client send --address host.domain:8080 --command open_ssh --key "$(secret-tool lookup token ruroco)"
```

If you want to use a different IP address than the one you are sending the packet from, you can use the `--ip` argument
together with `--permissive`:

```shell
ruroco-client send --address host.domain:8080 --command open_ssh --ip 94.111.111.111 --permissive --key "$(secret-tool lookup token ruroco)"
```

If you want to make sure that an adversary does not spoof your source IP address, you can get your external IP address
from a service - the ruroco server will make sure that the IP addresses match:

```shell
ruroco-client send --address host.domain:8080 --command open_ssh --ip $(curl -s https://api64.ipify.org) --key "$(secret-tool lookup token ruroco)"
```

the server will validate that the client is authorized to execute that command by using the shared AES key (id is sent
with the packet) and will then execute the command defined in the config above under "open_ssh". The `--deadline`
argument means that the command has to be started on the server within 5 seconds after executing the command.

This gives you the ability to effectively only allow access to the SSH port, for only the IP that the UDP packet was
sent from, if you want to connect to your server. Of course, you should also do all the other security hardening tasks
you would do if the SSH port would be exposed to the internet.

You can define any number of commands you wish, by adding more commands to configuration file.

## Enabling webservice

You may run a webservice like https://github.com/filebrowser/filebrowser on your server, which you do not want to
publicly expose. If you use nginx as a reverse proxy, you can use ruroco to enable or disable services:

```toml
# see chapter "server config"
[commands]
disable_file_browser = "mv /etc/nginx/conf.d/https_file_browser.conf /etc/nginx/conf.d/https_file_browser.conf_disabled && nginx -s reload"
enable_file_browser = "mv /etc/nginx/conf.d/https_file_browser.conf_disabled /etc/nginx/conf.d/https_file_browser.conf && nginx -s reload"
```

If you have configured ruroco on server like that and execute the following client side command

```shell
ruroco-client send --address host.domain:8080 --command enable_file_browser --key "$(secret-tool lookup token ruroco)"
```

the file browser nginx config will be enabled and nginx reloaded, effectively making the file browser accessible.

# architecture

## overview

The service consists of three parts:

- `ruroco-client`
    - binary that is executed on your local host
- `ruroco-server`
    - service that runs on a remote host where you wish to execute the commands on
    - exposed to the internet
    - has minimal rights to receive and decrypt data and to communicate with the commander
- `ruroco-commander`
    - daemon service that runs on the same host as the server
    - not exposed to the internet
    - has all the rights it needs to run the commands that are passed to it

<!-- created with https://asciiflow.com/#/ -->

```text
┌────────────────┐ ┌────────────────┐
│                │ │                │
│   ┌────────┐   │ │ ┌────────────┐ │
│   │ Client ├───┼─┤►│   Server   │ │
│   └────────┘   │ │ └─────┬──────┘ │
│                │ │       │        │
│                │ │ ┌─────▼──────┐ │
│                │ │ │  Commander │ │
│                │ │ └────────────┘ │
│   Local Host   │ │   Remote Host  │
└────────────────┘ └────────────────┘
```

## execution

Whenever a user sends a command via the client, the following steps are executed

1. client concatenates the current timestamp (in nanoseconds) with the command name (e.g. "default"), encrypts data with
   the private key and sends the encrypted data via UDP to the server
2. server receives the UDP package (does **not** answer), decrypts it with the public key and validates its content
3. if the content is valid, the server sends the command name to the commander. If the content is invalid an error
   message is logged
4. commander receives the command name and executes the command if the command is defined in the configuration

```text
     ┌─────────┐            ┌─────────┐              ┌───────────┐              
     │ Client  │            │ Server  │              │ Commander │              
     └────┬────┘            └────┬────┘              └─────┬─────┘              
          │                      │                         │                    
          │ Encrypt and send     │                         │                    
          ├─────────────────────►│                         │                    
          │ data via UDP         │                         │                    
          │                      │ Decrypt and validate    │                    
          │                      ├─────────────┐ data      │                    
          │                      │             │           │                    
          │                      │◄────────────┘           │                    
          │                      │                         │                    
          │                      │                         │                    
          │                      │                         │                    
┌────┬────┼──────────────────────┼─────────────────────────┼───────────────────┐
│alt │    │                      │                         │                   │
├────┘    │                      │ Log error               │                   │
│if is    │                      ├─────────────┐           │                   │
│invalid  │                      │             │           │                   │
│         │                      │◄────────────┘           │                   │
│         │                      │                         │                   │
├─  ──  ──┤ ──  ──  ──  ──  ──  ─┤  ──  ──  ──  ──  ──  ── ├──  ──  ──  ──  ── │
│else     │                      │                         │                   │
│         │                      │ Send command name       │                   │
│         │                      ├────────────────────────►│                   │
│         │                      │                         │                   │
│         │                      │                         │ Check if command  │
│         │                      │                         │ is valid          │
│         │                      │                         ├───────────────┐   │
│         │                      │                         │               │   │
│         │                      │                         │◄──────────────┘   │
│         │                      │                         │                   │
│         │            ┌────┬────┼─────────────────────────┼───────────────────┤
│         │            │alt │    │                         │ Execute command   │
│         │            ├────┘    │                         ├───────────────┐   │
│         │            │if is    │                         │               │   │
│         │            │valid    │                         │◄──────────────┘   │
│         │            │         │                         │                   │
│         │            ├─  ──  ──┤ ──  ──  ──  ──  ──  ──  ├─  ──  ──  ──  ──  │
│         │            │else     │                         │Log Error          │
│         │            │         │                         ├───────────────┐   │
│         │            │         │                         │               │   │
│         │            │         │                         │◄──────────────┘   │
│         │            │         │                         │                   │
└─────────┴────────────┴─────────┴─────────────────────────┴───────────────────┘
```
