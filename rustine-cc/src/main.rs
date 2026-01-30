mod network;

use network::IPAdapter;

fn main() {
    let adapters: Vec<IPAdapter> = IPAdapter::get_adapters();
    if adapters.is_empty() {
        println!("No ethernet IPv4 adapters found or `ip` command missing.");
    } else {
        for a in adapters.iter() {
            if let Some(hw) = &a.hwaddr {
                println!(
                    "{} -> {} mask {} mtu {} hw {}",
                    a.name, a.address, a.address_mask, a.mtu, hw
                );
            } else {
                println!(
                    "{} -> {} mask {} mtu {}",
                    a.name, a.address, a.address_mask, a.mtu
                );
            }
        }
    }
}
