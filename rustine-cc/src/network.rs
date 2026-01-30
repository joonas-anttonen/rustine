#![allow(dead_code)]

use std::ffi::CStr;
use std::fs;
use std::net::Ipv4Addr;
use std::ptr;
use libc::{freeifaddrs, getifaddrs, ifaddrs, sockaddr_in, AF_INET};

#[derive(Debug, Clone)]
pub struct IPAdapter {
    pub address: Ipv4Addr,
    pub address_mask: Ipv4Addr,
    pub mtu: u32,
    pub name: String,
    pub hwaddr: Option<HardwareAddress>,
}

impl IPAdapter {
    pub fn get_adapters() -> Vec<IPAdapter> {
        let mut result = Vec::new();

        unsafe {
            let mut ifap: *mut ifaddrs = ptr::null_mut();
            if getifaddrs(&mut ifap) != 0 {
                return result;
            }

            let mut cur = ifap;
            while !cur.is_null() {
                let ifa = &*cur;
                if !ifa.ifa_addr.is_null() && (ifa.ifa_addr as *const libc::sockaddr).as_ref().map_or(false, |s| s.sa_family as i32 == AF_INET) {
                    // interface name
                    let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();

                    // extract IPv4 address
                    let sa: libc::sockaddr_in = *(ifa.ifa_addr as *const sockaddr_in);
                    let addr_u32 = u32::from_be(sa.sin_addr.s_addr);
                    let address = Ipv4Addr::from(addr_u32);

                    // netmask from ifa_netmask if available
                    let address_mask = if !ifa.ifa_netmask.is_null() {
                        let nm = *(ifa.ifa_netmask as *const sockaddr_in);
                        let mask_u32 = u32::from_be(nm.sin_addr.s_addr);
                        Ipv4Addr::from(mask_u32)
                    } else {
                        Ipv4Addr::from(0u32)
                    };

                    // Check interface sysfs for type and operstate
                    let base = format!("/sys/class/net/{}", name);
                    let operstate = fs::read_to_string(format!("{}/operstate", base))
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if operstate != "up" {
                        cur = ifa.ifa_next;
                        continue;
                    }

                    let iftype = fs::read_to_string(format!("{}/type", base))
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    if iftype != "1" {
                        cur = ifa.ifa_next;
                        continue;
                    }

                    let mtu = fs::read_to_string(format!("{}/mtu", base))
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .unwrap_or(0);

                    // read hardware (MAC) address if present
                    let hwaddr = fs::read_to_string(format!("{}/address", base))
                        .ok()
                        .and_then(|s| HardwareAddress::from_str(&s));

                    result.push(IPAdapter {
                        address,
                        address_mask,
                        mtu,
                        name,
                        hwaddr,
                    });
                }

                cur = (*cur).ifa_next;
            }

            freeifaddrs(ifap);
        }

        // sort by name to match original behavior
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct HardwareAddress {
    addr: [u8; 6],
}

impl HardwareAddress {
    pub const LENGTH_IN_BYTES: usize = 6;

    pub fn new() -> HardwareAddress {
        HardwareAddress { addr: [0xff; 6] }
    }

    pub fn from_slice(slice: &[u8]) -> Option<HardwareAddress> {
        if slice.len() < HardwareAddress::LENGTH_IN_BYTES {
            return None;
        }
        let mut a = [0u8; 6];
        a.copy_from_slice(&slice[0..6]);
        Some(HardwareAddress { addr: a })
    }

    pub fn from_str(s: &str) -> Option<HardwareAddress> {
        let s = s.trim();
        let parts: Vec<&str> = s.split(|c| c == ':' || c == '-').collect();
        if parts.len() != HardwareAddress::LENGTH_IN_BYTES {
            return None;
        }
        let mut a = [0u8; 6];
        for i in 0..6 {
            a[i] = u8::from_str_radix(parts[i], 16).ok()?;
        }
        Some(HardwareAddress { addr: a })
    }
}

impl std::fmt::Display for HardwareAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.addr[0],
            self.addr[1],
            self.addr[2],
            self.addr[3],
            self.addr[4],
            self.addr[5]
        )
    }
}

impl std::fmt::Debug for HardwareAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HardwareAddress({})", self)
    }

}
