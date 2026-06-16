use crate::client::send::core::Sender;
use crate::common::info;
use anyhow::{bail, Context};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket};

impl Sender {
    pub(super) fn get_destination_ips(&self) -> anyhow::Result<Vec<IpAddr>> {
        let address = &self.cmd.address;

        let destination_ips: Vec<SocketAddr> = self
            .cmd
            .address
            .to_socket_addrs()
            .with_context(|| format!("Could not resolve hostname for {}", self.cmd.address))?
            .collect();

        let destination_ipv4s: Vec<&SocketAddr> =
            destination_ips.iter().filter(|a| a.is_ipv4()).collect();

        let destination_ipv6s: Vec<&SocketAddr> =
            destination_ips.iter().filter(|a| a.is_ipv6()).collect();

        // Neither or both of --ipv4/--ipv6 given means "no preference": send to whatever resolves.
        let use_ip_undef = self.cmd.ipv4 == self.cmd.ipv6;
        Ok(match (destination_ipv4s.first(), destination_ipv6s.first()) {
            // ipv4 or ipv6 where not defined or where both defined
            (Some(ipv4), Some(ipv6)) if use_ip_undef => vec![ipv4.ip(), ipv6.ip()],
            (Some(ipv4), None) if use_ip_undef => vec![ipv4.ip()],
            (None, Some(ipv6)) if use_ip_undef => vec![ipv6.ip()],
            // ipv4 xor ipv6 where defined
            (_, Some(ipv6)) if self.cmd.ipv6 => vec![ipv6.ip()],
            (Some(ipv4), _) if self.cmd.ipv4 => vec![ipv4.ip()],
            (_, None) if self.cmd.ipv6 => {
                bail!("Could not find any IPv6 address for {address}")
            }
            (None, _) if self.cmd.ipv4 => {
                bail!("Could not find any IPv4 address for {address}")
            }
            // could not find any address
            _ => bail!("Could not find any IPv4 or IPv6 address for {address}"),
        })
    }

    pub(super) fn send_data(&mut self, ip: IpAddr) -> anyhow::Result<()> {
        self.counter.inc()?;
        let bind_address = if ip.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" };

        info(format!("Connecting to {ip}..."));
        let data_to_encrypt = self.get_data_to_encrypt(ip)?;
        let data_to_send = self.data_parser.encode(&data_to_encrypt)?;

        let address = &self.cmd.address;
        let socket = UdpSocket::bind(bind_address).with_context(|| Self::socket_ctx(address))?;
        socket.connect(address).with_context(|| Self::socket_ctx(address))?;
        socket.send(&data_to_send).with_context(|| Self::socket_ctx(address))?;

        info(format!("Sent command {} from {bind_address} to udp://{address}", &self.cmd.command));
        Ok(())
    }

    pub(super) fn socket_ctx<E: std::fmt::Debug>(val: E) -> String {
        format!("Could not connect/send data to {val:?}")
    }
}
