// Copyright (c) 2019-2022 Alibaba Cloud
// Copyright (c) 2019-2022 Ant Group
//
// SPDX-License-Identifier: Apache-2.0
//

use std::{
    convert::TryFrom,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use anyhow::{anyhow, Result};
use netlink_packet_route::{nlas::address::Nla, AddressMessage, AF_INET, AF_INET6};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Address {
    pub addr: IpAddr,
    pub label: String,
    pub flags: u32,
    pub scope: u8,
    pub perfix_len: u8,
    pub peer: IpAddr,
    pub broadcast: IpAddr,
    pub prefered_lft: u32,
    pub valid_ltf: u32,
}

impl TryFrom<AddressMessage> for Address {
    type Error = anyhow::Error;
    fn try_from(msg: AddressMessage) -> Result<Self> {
        let AddressMessage { header, nlas } = msg;
        let mut addr = Address {
            addr: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            peer: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            broadcast: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            label: String::default(),
            flags: 0,
            scope: header.scope,
            perfix_len: header.prefix_len,
            prefered_lft: 0,
            valid_ltf: 0,
        };

        for nla in nlas.into_iter() {
            match nla {
                Nla::Address(a) => {
                    addr.addr = parse_ip(&a, header.family)?;
                }
                Nla::Broadcast(b) => {
                    addr.broadcast = parse_ip(&b, header.family)?;
                }
                Nla::Label(l) => {
                    addr.label = l;
                }
                Nla::Flags(f) => {
                    addr.flags = f;
                }
                Nla::CacheInfo(_c) => {}
                _ => {}
            }
        }

        Ok(addr)
    }
}

pub(crate) fn parse_ip(ip: &[u8], family: u8) -> Result<IpAddr> {
    let support_len = if family as u16 == AF_INET { 4 } else { 16 };
    if ip.len() != support_len {
        return Err(anyhow!(
            "invalid ip addresses {:?} support {}",
            &ip,
            support_len
        ));
    }
    match family as u16 {
        AF_INET => Ok(IpAddr::V4(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]))),
        AF_INET6 => {
            let mut octets = [0u8; 16];
            octets.copy_from_slice(&ip[..16]);
            Ok(IpAddr::V6(Ipv6Addr::from(octets)))
        }
        _ => Err(anyhow!("unknown IP network family {}", family)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ip() {
        let test_ipv4 = [10, 25, 64, 128];
        let ipv4 = parse_ip(test_ipv4.as_slice(), AF_INET as u8).unwrap();
        let expected_ipv4 = IpAddr::V4(Ipv4Addr::new(10, 25, 64, 128));
        assert_eq!(ipv4, expected_ipv4);

        let test_ipv6 = [0, 2, 4, 0, 0, 2, 4, 0, 0, 2, 4, 0, 0, 2, 4, 0];
        let ipv6 = parse_ip(test_ipv6.as_slice(), AF_INET6 as u8).unwrap();
        // two u8 => one u16, (0u8, 2u8 => 0x0002), (4u8, 0u8 => 0x0400)
        let expected_ipv6 = IpAddr::V6(Ipv6Addr::new(
            0x0002, 0x0400, 0x0002, 0x0400, 0x0002, 0x0400, 0x0002, 0x0400,
        ));
        assert_eq!(ipv6, expected_ipv6);

        let fail_ipv4 = [10, 22, 33, 44, 55];
        assert!(parse_ip(fail_ipv4.as_slice(), AF_INET as u8).is_err());

        let fail_ipv6 = [1, 2, 3, 4, 5, 6, 7, 8, 2, 3];
        assert!(parse_ip(fail_ipv6.as_slice(), AF_INET6 as u8).is_err());
    }
}
