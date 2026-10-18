use std::net::{IpAddr, Ipv4Addr};

/// Builds the upload URLs reachable from the LAN for the given candidate addresses.
///
/// Keeps only routable IPv4 addresses (no loopback, link-local, unspecified or IPv6),
/// preserves the order of first appearance and drops duplicates.
pub fn upload_urls(candidates: &[IpAddr], port: u16) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    for candidate in candidates {
        let IpAddr::V4(ip) = candidate else { continue };
        if !is_lan_ipv4(ip) {
            continue;
        }
        let url = format!("http://{ip}:{port}/upload");
        if !urls.contains(&url) {
            urls.push(url);
        }
    }
    urls
}

/// Detects the machine's LAN IPv4 addresses: the default-route address first,
/// then the remaining interface addresses.
pub fn detect_lan_ips() -> Vec<IpAddr> {
    let mut ips = Vec::new();
    if let Ok(ip) = local_ip_address::local_ip() {
        ips.push(ip);
    }
    if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
        ips.extend(interfaces.into_iter().map(|(_, ip)| ip));
    }
    ips
}

/// Upload URLs for this machine on the port the server actually bound.
pub fn lan_upload_urls(port: u16) -> Vec<String> {
    upload_urls(&detect_lan_ips(), port)
}

fn is_lan_ipv4(ip: &Ipv4Addr) -> bool {
    !(ip.is_loopback() || ip.is_link_local() || ip.is_unspecified())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn formats_lan_address_with_bound_port() {
        assert_eq!(
            upload_urls(&[ip("192.168.1.20")], 4848),
            vec!["http://192.168.1.20:4848/upload"]
        );
    }

    #[test]
    fn uses_fallback_port() {
        assert_eq!(
            upload_urls(&[ip("192.168.1.20")], 4849),
            vec!["http://192.168.1.20:4849/upload"]
        );
    }

    #[test]
    fn drops_loopback_link_local_unspecified_and_ipv6() {
        let all = [
            ip("127.0.0.1"),
            ip("169.254.10.5"),
            ip("0.0.0.0"),
            ip("fe80::1"),
            ip("2001:db8::1"),
            ip("10.0.0.7"),
        ];
        assert_eq!(upload_urls(&all, 4848), vec!["http://10.0.0.7:4848/upload"]);
    }

    #[test]
    fn deduplicates_keeping_first_order() {
        let all = [ip("192.168.1.20"), ip("10.0.0.7"), ip("192.168.1.20")];
        assert_eq!(
            upload_urls(&all, 4848),
            vec![
                "http://192.168.1.20:4848/upload",
                "http://10.0.0.7:4848/upload"
            ]
        );
    }

    #[test]
    fn no_candidates_yields_no_urls() {
        assert!(upload_urls(&[], 4848).is_empty());
    }
}
