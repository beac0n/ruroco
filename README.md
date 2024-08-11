![build](https://github.com/beac0n/ruroco/actions/workflows/rust.yml/badge.svg)
[![release](https://img.shields.io/github/v/release/beac0n/ruroco?style=flat&labelColor=1C2C2E&color=C96329&logo=GitHub&logoColor=white)](https://github.com/beac0n/ruroco/releases)
[![codecov](https://codecov.io/gh/beac0n/ruroco/graph/badge.svg?token=H7ABBHYYWT)](https://codecov.io/gh/beac0n/ruroco)

# ruroco - run remote command

Ruroco is a tool to run pre-defined commands on a remote server, using the UDP protocol to hide the existence of the
service from adversaries, making the service on the server "invisible".

## use case

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
address = "0.0.0.0:8080"  # address the ruroco serer listens on, if systemd/ruroco.socket is not used
config_dir = "/etc/ruroco/"  # path where the configuration files are saved

[commands]
# open ssh, but only for the IP address where the request came from
open_ssh = "ufw allow from $RUROCO_IP proto tcp to any port 80"
# close ssh, but only for the IP address where the request came from
close_ssh = "ufw delete allow from $RUROCO_IP proto tcp to any port 80"
```

If you have configured ruroco on server like that and execute the following client side command

```shell
ruroco-client send --address host.domain:8080 --private-pem-path /path/to/ruroco_private.pem --command open_ssh --deadline 5
```

the server will validate that the client is authorized to execute that command by using public-key cryptography (RSA)
and will then execute the command defined in the config above under "open_ssh". The `--deadline` argument means that the
command has to be started on the server within 5 seconds after executing the command.

This gives you the ability to effectively only allow access to the SSH port, for only the IP that the UDP packet was
sent from, if you want to connect to your server. Of course, you should also do all the other security hardening tasks
you would do if the SSH port would be exposed to the internet.

You can define any number of commands you wish, by adding more commands to configuration file.

## security

A lot of thought has gone into making this tool as secure as possible:

- The client sends a UDP packet to the server, to which the server never responds. So port-scanning does not help an
  adversary.
- The server only holds the public key. The client uses the private key to send an encrypted packet.
- Each request that is sent holds the current timestamp and the command that the server should execute.
  This encrypted packet is only valid for a configurable amount of time.
- On the server, the service that received the UDP package has as little OS rights as possible (restricted by systemd).
  After validating the data, the service that received the UDP packet (server) instructs another service (commander) to
  execute the command. So even if the server service is compromised, it can't do anything, because it's rights are
  extremely limited.
- Each packet can only be sent once and will be blacklisted on the server.
- (WIP) To make the service less vulnerable against DoS attacks ...

## architecture

### overview

The service consists of three parts:

- `client`
    - binary that is executed on your local host
- `server`
    - service that runs on a remote host where you wish to execute the commands on
    - exposed to the internet
    - has minimal rights to receive and decrypt data and to communicate with the commander
- `commander`
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

### execution

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
