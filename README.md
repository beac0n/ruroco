[![build](https://github.com/beac0n/ruroco/actions/workflows/rust.yml/badge.svg)](https://github.com/beac0n/ruroco/actions)
[![release](https://img.shields.io/github/v/release/beac0n/ruroco?style=flat&labelColor=1C2C2E&color=C96329&logo=GitHub&logoColor=white)](https://github.com/beac0n/ruroco/releases)
[![codecov](https://codecov.io/gh/beac0n/ruroco/graph/badge.svg?token=H7ABBHYYWT)](https://codecov.io/gh/beac0n/ruroco)

# ruroco - Run Remote Command

ruroco is a tool that lets you execute commands on a server by sending UDP packets.

It triggers a pre-configured **action** on the server (open a firewall rule, restart a service, run a
script) — it is **not** a tunnel or a VPN. There is no session, no connection, and no traffic carried: the
client fires a single stateless packet and the server runs a whitelisted command. In other words, ruroco
grants a *capability to act* without granting *network access*. See
[ruroco vs WireGuard / VPN](#ruroco-vs-wireguard--vpn) for when that matters and when a plain VPN is the
better choice.

the tool consist of 4 binaries:

- `ruroco-client`: runs on your notebook/computer and sends the UDP packets
- `ruroco-client-ui`: presents most of the functionality of `ruroco-client` in an easier to use user interface.
- `ruroco-server`: receives the UDP packets and makes sure that they are valid
- `ruroco-commander`: runs the command encoded by the data of the UDP packet if it's valid

The commands are configured on the server side, so the client does not define what is going to be executed, it only
picks from existing commands.

## Table of Contents

- [Installation](#installation)
  - [Client](#client)
  - [Client UI](#client-ui)
  - [Server](#server)
  - [Android](#android)
- [Security](#security)
- [Client UI usage](#client-ui-usage)
- [Client usage](#client-usage)
  - [gen](#gen)
  - [send](#send)
  - [update](#update)
  - [reseed](#reseed)
  - [wizard](#wizard)
- [Server usage](#server-usage)
- [Commander usage](#commander-usage)
- [Server config](#server-config)
- [Use cases](#use-cases)
  - [Triggering an action](#triggering-an-action)
  - [Single packet authorization (SPA)](#single-packet-authorization-spa)
  - [Enabling webservice](#enabling-webservice)
- [ruroco vs WireGuard / VPN](#ruroco-vs-wireguard--vpn)
- [Troubleshooting](#troubleshooting)
- [Architecture](#architecture)

## Installation

Download binaries from the [releases page](https://github.com/beac0n/ruroco/releases) or build them yourself by running

```shell
make release
```

you can find the binaries in `target/release/client`, `target/release/client_ui`, `target/release/server` and
`target/release/commander`

### Client

#### Arch Linux (AUR)

```shell
yay -S ruroco-client
```

#### Self-build

See make goal `install_client`. This builds the project and copies the client binary to `/usr/local/bin/ruroco-client`

#### Pre-build

Run the following script

```shell
curl -Ls "$(curl -s https://api.github.com/repos/beac0n/ruroco/releases/latest | grep -oE 'https://[^"]*/client-v[0-9]+\.[0-9]+\.[0-9]+-x86_64-linux')" -o ~/.local/bin/ruroco-client 
chmod +x ~/.local/bin/ruroco-client
~/.local/bin/ruroco-client update --force
```

### Client UI

#### Arch Linux (AUR)

```shell
yay -S ruroco-client-ui
```

### Server

#### Arch Linux (AUR)

```shell
yay -S ruroco-server
```

After installing, generate a shared key, place it in `/etc/ruroco/`, and edit `/etc/ruroco/config.toml`.

#### Self-build

See make goal `install_server`, which

- Builds the project
- Copies the client binary to `~/.local/bin/`
- Copies the server binaries to `/usr/local/bin/`
- Runs `ruroco-client wizard`
- After running the make goal, you have to
    - generate a shared `.key` file and copy it to the right place
    - setup the `config.toml`

#### Pre-build

Run the following script

```shell
curl -Ls "$(curl -s https://api.github.com/repos/beac0n/ruroco/releases/latest | grep -oE 'https://[^"]*/client-v[0-9]+\.[0-9]+\.[0-9]+-x86_64-linux')" -o ~/.local/bin/ruroco-client 
chmod +x ~/.local/bin/ruroco-client
~/.local/bin/ruroco-client update --force
sudo ~/.local/bin/ruroco-client wizard
```

### Android

See `nix/android.nix`, `scripts/dev_ui_android.sh` and `scripts/release_android.sh`

## security

- client sends UDP packet to server, server never responds to it -> **port-scanning** does not help an adversary
- data sent from client to server is encrypted symmetrically with AES-256-GCM-SIV using a shared key (symmetric is a
  deliberate trade-off, see [SECURITY.md](SECURITY.md))
- client only defines command to execute, **commands are saved on server** in a root-only file the network-facing
  server process cannot read -> client can pick command but not define it
- run server software in such a way so that it uses **as little operating system rights** as possible
- **replay protection** by adding every packet that the server received to a blocklist

See [SECURITY.md](SECURITY.md) for the full threat model: in-scope attacks, accepted risks, key lifecycle
(generation, rotation, revocation), and the supported-versions table.

## client ui usage

```shell
ruroco-client-ui
```

Use the Generate Key action (or `ruroco-client gen`) to produce a base64-encoded shared key. Copy that key into the
server's `.key` files (see server config) and reuse the same string with `ruroco-client send`. You can save your key
safely in a password manager or use `secret-tool` to store it in the local keyring, e.g. with
`secret-tool store --label="ruroco" token ruroco`

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
  reseed  Reseed the replay-protection counter to the current timestamp
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
  -a, --address <ADDRESS>          Address to send the command to
  -k, --key <KEY>                  Base64 key with id (output of `ruroco-client gen` or the UI)
  -c, --command <COMMAND>          Command to send [default: default]
  -e, --permissive                 Allow permissive IP validation - source IP does not have to match provided IP
  -i, --ip <IP>                    Optional IP address from which the command was sent. Use -6ei "dead:beef:dead:beef::/64" to allow you whole current IPv6 network. To do this automatically, use -6ei $(curl -s6 https://api64.ipify.org | awk -F: '{print $1":"$2":"$3":"$4"::/64"}')
  -4, --ipv4                       Connect via IPv4
  -6, --ipv6                       Connect via IPv6
  -d, --send-delay-ms <DELAY_MS>   Delay in milliseconds between sending to multiple destinations (IPv4 + IPv6) [default: 50]
  -h, --help                       Print help
```

Pass the same base64 key string that you placed on the server. Example:

```shell
ruroco-client send -a 127.0.0.1:80 -k "$(secret-tool lookup token ruroco)" -c default
```

### update

```shell
ruroco-client update --help
```

```text
Update the client binary

Usage: ruroco-client update [OPTIONS]

Options:
  -f, --force                  Force update even if already on the latest version
  -v, --version <VERSION>      Target version to install (default: latest)
  -b, --bin-path <BIN_PATH>    Directory where the binary is saved (default: same as current binary)
  -s, --server                 Update server-side binaries instead of the client
  -h, --help                   Print help
```

Every downloaded binary is verified against an Ed25519 signature before it is written to
disk. The public key is embedded in the client at build time; releases are signed in CI with
the matching private key. If the `.sig` asset is missing or the signature does not match, the
update aborts and the existing binary is left untouched. As a result, the client can only
update to releases that ship signatures (`v0.14.0` and later).

### reseed

```shell
ruroco-client reseed
```

Resets the local replay-protection counter to the current nanosecond timestamp. Use this if the counter
file is lost or corrupted, or after a system clock jump that would cause the server to reject packets.

The UI equivalent is the **Reseed Counter** button on the dashboard.

### wizard

```shell
ruroco-client wizard --help
```

```text
Run the wizard to set up the server side

Usage: ruroco-client wizard [OPTIONS]

Options:
  -f, --force   Overwrite existing configuration files
  -h, --help    Print help
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
4. add the commands to `/etc/ruroco/commands.toml` -> see [commands.toml](config/commands.toml). This file is read
   only by the commander (root) and must be installed `root`-owned with mode `0600` so the unprivileged server process
   cannot read the command set.
5. call `ruroco-client send` with `-k "$(secret-tool lookup token ruroco)"` so client and server share the identical key

# use cases

ruroco's core job is to **trigger a pre-configured action** on the server. The strongest cases are the ones
where there is no service to connect to at all — only something to *make happen*. The SPA example below
(opening a port on demand) is the one shape of use case that overlaps with what a VPN can do; for why ruroco
is still a distinct tool, see [ruroco vs WireGuard / VPN](#ruroco-vs-wireguard--vpn).

## triggering an action

The most VPN-orthogonal use of ruroco is firing a server-side action that has no "service" to reach: deploy,
restart something, rotate a secret, run a backup. Configure the commands on the server:

```toml
# /etc/ruroco/commands.toml (root-only, see chapter "server config")
[commands]
deploy = "/usr/local/bin/deploy.sh"
restart_app = "systemctl restart myapp"
run_backup = "/usr/local/bin/run-backup.sh"
```

then fire one from anywhere — including a CI runner, a cron job on another host, or a tap in the UI:

```shell
ruroco-client send --address host.domain:80 --command deploy --key "$(secret-tool lookup token ruroco)"
```

There is no tunnel to bring up and no session to maintain: the client sends a single stateless packet and the
command runs. A VPN cannot do this — it would only connect you so that *you* could then run the command
yourself. And because the client can only pick from whitelisted command names (it never sees the command
strings), a compromised client can fire those actions but gains no foothold on your network.

## single packet authorization (SPA)

Any port you expose to the internet attracts brute-force and exploit traffic — and even a well-secured
service is not 100% safe, see
https://www.schneier.com/blog/archives/2024/04/xz-utils-backdoor.html or
https://www.qualys.com/2024/07/01/cve-2024-6387/regresshion.txt

Blocking every port that does not need to be open at all times reduces that attack surface. But blocking a
port completely makes the service behind it unreachable when you actually do need it.

This is where ruroco comes in: it can open a firewalled port for just a short while — only for the IP that
asked — and close it again afterwards. A command can reference that IP via the `$RUROCO_IP` environment
variable, which the commander substitutes before running the command:

```toml
# /etc/ruroco/commands.toml (root-only, see chapter "server config")
[commands]
open_port = "ufw allow from $RUROCO_IP proto tcp to any port 8443"         # open the service for the requesting IP
close_port = "ufw delete allow from $RUROCO_IP proto tcp to any port 8443" # close it again
```

With that configured, run the client like this:

```shell
ruroco-client send --address host.domain:80 --command open_port --key "$(secret-tool lookup token ruroco)"
```

If you want to authorize a different address than the one you are sending from (for example your external IP
when sending from behind NAT, or another host entirely), pass it with `--ip` together with `--permissive`. In
permissive mode the server does **not** check the supplied IP against the packet's real source: it trusts
whatever `--ip` you send and uses it for `$RUROCO_IP`. Anyone holding the shared key can therefore authorize
any routable IP, so guard the key accordingly. (Without `--permissive` the default is strict: if you pass
`--ip` the server rejects the packet unless it matches the real source, and `$RUROCO_IP` is always the
verified sender.) Fetch your external address from a service when you need it:

```shell
ruroco-client send --address host.domain:80 --command open_port --ip $(curl -s https://api64.ipify.org) --key "$(secret-tool lookup token ruroco)"
```

The server validates that the client is authorized to run the command using the shared AES key (its id is
sent with the packet) and then runs the command defined under `open_port`. This gives you on-demand access to
a port for only the IP that sent the packet. Of course you should still apply the usual hardening to the
service behind it.

You can define any number of commands you wish, by adding more commands to the configuration file.

## Enabling webservice

You may run a webservice like https://github.com/filebrowser/filebrowser on your server, which you do not want to
publicly expose. If you use nginx as a reverse proxy, you can use ruroco to enable or disable services:

```toml
# /etc/ruroco/commands.toml (root-only, see chapter "server config")
[commands]
disable_file_browser = "mv /etc/nginx/conf.d/https_file_browser.conf /etc/nginx/conf.d/https_file_browser.conf_disabled && nginx -s reload"
enable_file_browser = "mv /etc/nginx/conf.d/https_file_browser.conf_disabled /etc/nginx/conf.d/https_file_browser.conf && nginx -s reload"
```

If you have configured ruroco on server like that and execute the following client side command

```shell
ruroco-client send --address host.domain:80 --command enable_file_browser --key "$(secret-tool lookup token ruroco)"
```

the file browser nginx config will be enabled and nginx reloaded, effectively making the file browser accessible.

# ruroco vs WireGuard / VPN

A reasonable question is: if you can hide every service behind a VPN like
[WireGuard](https://www.wireguard.com/) and only connect when you need them, why use ruroco at all? For that
specific use case — *you* hiding *your own* services and reaching them *yourself* — the honest answer is that
WireGuard alone is the better tool, and ruroco adds little:

- WireGuard is **also silent** to unauthenticated packets: only a valid handshake from a known peer gets a
  response, so port-scanning it reveals nothing either. The "nothing to scan" property is not unique to
  ruroco.
- WireGuard is small, in-kernel, and one of the most heavily audited pieces of crypto networking in
  existence. A custom UDP daemon has not had that scrutiny.
- Once you are on the tunnel you reach all your services bidirectionally with no per-service knocking —
  strictly more convenient.

So if your goal is "private services for myself", reach for WireGuard. ruroco is **not** a VPN and does not
try to replace one.

## where ruroco is a different tool

ruroco and a VPN answer different questions. **A VPN grants access; ruroco grants a capability.** A VPN peer
gets a position on your network and bidirectional reach to everything behind the tunnel — and if that client
is compromised, the attacker inherits that foothold. A ruroco client can only fire whitelisted, pre-defined
commands and never gets a network position at all; compromise it and the worst case is triggering a
configured action (e.g. enabling one configured service), not roaming your network.

That distinction is what justifies ruroco in cases a VPN does not cover:

- **Triggering server-side actions, not reaching services.** "Deploy", "restart nginx", "rotate the cert",
  "run a backup" — there is no service to connect to, only an action to perform. A VPN can only connect you
  so that you can then do it by hand.
- **Triggers from clients that cannot or should not hold a tunnel.** A CI runner, a cron job on another host,
  an IoT device, a phone tap. One stateless UDP packet, no handshake, no session, no client config to
  provision.
- **Granting a third party an action without granting them your network.** You might let a partner fire
  "rebuild the search index" while never giving them VPN access. SPA gives capability without connectivity —
  least privilege a VPN structurally cannot express.

## a note on "gate the VPN with ruroco"

It is tempting to put ruroco *in front of* WireGuard — keep the WireGuard port firewalled and open it with a
ruroco knock. We do **not** recommend this as a security win: it places a less-audited UDP packet parser
(ruroco) in front of a more-audited one (WireGuard) without removing "internet-facing daemon parsing hostile
packets" — it just swaps which daemon does it. Use ruroco for what only ruroco does (triggering actions), not
to wrap a VPN that is already silent on its own.

# troubleshooting

## server rejects packets from one client

Each client must have its own unique key. If two clients share the same key, they each maintain an
independent local counter, but the server tracks only one counter per key. Whichever client sends
a packet last advances the server's counter - the other client's counter now lags behind, and the
server will reject all its future packets as replays.

To fix this:

1. Generate a new key for each client with `ruroco-client gen` (or the **Generate** button in the UI)
2. Copy the new key to the server config dir alongside the existing key
3. Use the new key when calling `ruroco-client send`

If you intentionally share a key (not recommended) and the counter falls out of sync, you can
recover without generating a new key by running:

```shell
ruroco-client reseed
```

This resets the local counter to the current nanosecond timestamp, which will be higher than any
value the server has seen and allows packets to be accepted again. The UI equivalent is the
**Reseed Counter** button on the dashboard.

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
