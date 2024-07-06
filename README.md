# ruroco - run remote command

ruroco is a tool to run pre-defined commands on a remote server.

## use case

If you host a server on the web, you know that you'll get lots of brute-force attacks on (at least) the SSH port of your
server. While using good practices in securing your server will keep you safe from such attacks, these attacks are quite
annoying (filling up logs) and even if you secured your server correctly, you will still not be 100% safe, 
see https://www.schneier.com/blog/archives/2024/04/xz-utils-backdoor.html.

Completely blocking all traffic to all ports that do not have to be open at all times can reduce the attack surface.
But blocking the SSH port completely will make SSH unusable for that server.

This is where ruroco comes in. Ruroco can execute a command that opens up the SSH port for just a short amount of time, 
so that you can ssh into your server. Afterward ruruco closes the SSH port again. To implement this use case with
ruroco, you have to use a configuration similar to the one shown below:

```toml
address = "127.0.0.1:8080"
pem_path = "/etc/ruroco/ruroco_public.pem"
max_delay_sec = 5

[commands]
[commands.default]
start = "ufw allow 22/tcp"
stop = "ufw deny 22/tcp"
sleep = 5
```

If you have configured ruroco on server like that and execute the client side command 
`ruroco-client send --address host.domain:port --private-pem-path /path/to/ruroco_private.pem --command default`, the
server will validate that the client is authorized to execute that command by using public-private-key cryptography (RSA)
and will then execute the following sequence:
- run command `ufw allow 22/tcp`
- sleep for 5 seconds
- run command `ufw deny 22/tcp`

this will give you 5 seconds to ssh into your SSH server, before the port is blocked again by ufw.

This gives you the ability to effectively only allow access to the SSH port, if you want to connect to your server.
Of course, you should also do all the other security hardening tasks you would do if the SSH port would be exposed to
the open web.

You can define any number of commands you wish, by adding more commands to configuration file.

## security

A lot of thought has gone into making this tool as secure as possible:
- The client sends a UDP packet to the server, to which the server never responds. So port-scanning does not help an attacker.
- The server only holds the public key. The client uses the private key to send an encrypted packet.
- Each request that is sent holds the current timestamp and the command that the server should execute. 
This encrypted packet is only valid for a configurable amount of time.
- On the server, the service that received the UDP package has as little OS rights as possible (restricted by systemd). 
After validating the data, the service that received the UDP packet (server) instructs another service (commander) to 
execute the command. So even if the server service is compromised, it can't do anything, because it's rights are extremely
limited from OS point of view.
- (WIP) Each packet can only be sent once and will be blacklisted on the server.
- (WIP) To make the service less vulnerable against DoS attacks ... 

## architecture

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