// collect_network.rs — Network collection (M4)
// BEHAVIORs: collect-network-interfaces, collect-network-routing, collect-network-firewall
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::interfaces::{Filesystem, CommandRunner};
use crate::types::*;

// ---------------------------------------------------------------------------
// collect-network-interfaces
// ---------------------------------------------------------------------------

pub fn collect_network_interfaces(cr: &dyn CommandRunner) -> ScopeWrapper<NetworkInterfaceRecord> {
    let output = match cr.run("ip", &["-j", "addr", "show"]) {
        Ok((o, _)) => o,
        Err(e) => {
            eprintln!("sitar: collect-network-interfaces: ip -j addr show failed: {}", e);
            return ScopeWrapper::default();
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&output) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("sitar: collect-network-interfaces: JSON parse error: {}", e);
            return ScopeWrapper::default();
        }
    };

    let interfaces = match json.as_array() {
        Some(a) => a,
        None => return ScopeWrapper::default(),
    };

    let mut records = Vec::new();
    for iface in interfaces {
        let mut rec = NetworkInterfaceRecord::default();

        rec.ifname    = iface.get("ifname").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.link_type = iface.get("link_type").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.address   = iface.get("address").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.mtu       = iface.get("mtu").and_then(|v| v.as_i64()).unwrap_or(0);
        rec.operstate = iface.get("operstate").and_then(|v| v.as_str()).unwrap_or("").to_string();

        if let Some(flags) = iface.get("flags").and_then(|v| v.as_array()) {
            rec.flags = flags.iter()
                .filter_map(|f| f.as_str())
                .map(|s| s.to_string())
                .collect();
        }

        if let Some(addr_info) = iface.get("addr_info").and_then(|v| v.as_array()) {
            for addr in addr_info {
                let family = addr.get("family").and_then(|v| v.as_str()).unwrap_or("");
                if family == "inet" && rec.ip.is_empty() {
                    rec.ip        = addr.get("local").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    rec.prefixlen = addr.get("prefixlen").and_then(|v| v.as_u64())
                        .map(|n| n.to_string())
                        .unwrap_or_default();
                    rec.broadcast = addr.get("broadcast").and_then(|v| v.as_str()).unwrap_or("").to_string();
                }
                if family == "inet6" && rec.ip6.is_empty() {
                    let scope = addr.get("scope").and_then(|v| v.as_str()).unwrap_or("");
                    if scope != "link" {
                        rec.ip6        = addr.get("local").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        rec.ip6_prefixlen = addr.get("prefixlen").and_then(|v| v.as_u64())
                            .map(|n| n.to_string())
                            .unwrap_or_default();
                    }
                }
            }
        }

        records.push(rec);
    }

    let mut attributes = std::collections::HashMap::new();
    attributes.insert("command".to_string(), serde_json::Value::String("ip".to_string()));

    ScopeWrapper { attributes, elements: records }
}

// ---------------------------------------------------------------------------
// collect-network-routing
// ---------------------------------------------------------------------------

pub fn collect_network_routing(cr: &dyn CommandRunner) -> ScopeWrapper<RouteRecord> {
    let output = match cr.run("ip", &["-j", "route", "show"]) {
        Ok((o, _)) => o,
        Err(e) => {
            eprintln!("sitar: collect-network-routing: ip -j route show failed: {}", e);
            return ScopeWrapper::default();
        }
    };

    let json: serde_json::Value = match serde_json::from_str(&output) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("sitar: collect-network-routing: JSON parse error: {}", e);
            return ScopeWrapper::default();
        }
    };

    let routes = match json.as_array() {
        Some(a) => a,
        None => return ScopeWrapper::default(),
    };

    let mut records = Vec::new();
    for route in routes {
        let mut rec = RouteRecord::default();

        let dst = route.get("dst").and_then(|v| v.as_str()).unwrap_or("default");
        rec.dst = if dst == "0.0.0.0/0" { "default".to_string() } else { dst.to_string() };

        rec.gateway  = route.get("gateway").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.dev      = route.get("dev").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.protocol = route.get("protocol").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.scope    = route.get("scope").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.r#type   = route.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.metric   = route.get("metric").and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_default();

        if let Some(flags) = route.get("flags").and_then(|v| v.as_array()) {
            rec.flags = flags.iter()
                .filter_map(|f| f.as_str())
                .map(|s| s.to_string())
                .collect();
        }

        records.push(rec);
    }

    ScopeWrapper { attributes: Default::default(), elements: records }
}

// ---------------------------------------------------------------------------
// collect-network-firewall
// ---------------------------------------------------------------------------

pub fn collect_network_firewall(
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
) -> ScopeWrapper<PacketFilterRecord> {
    // Step 1: ipfwadm
    if fs.exists("/proc/net/ip_input") {
        return ScopeWrapper {
            attributes: Default::default(),
            elements: vec![PacketFilterRecord {
                engine:     "ipfwadm".to_string(),
                table:      String::new(),
                raw_output: "ipfwadm is not supported.".to_string(),
            }],
        };
    }

    // Step 2: ipchains
    if fs.exists("/proc/net/ip_fwnames") {
        let fwnames = fs.read_file("/proc/net/ip_fwnames").unwrap_or_default();
        let fwchains = fs.read_file("/proc/net/ip_fwchains").unwrap_or_default();
        let mut records = Vec::new();
        for chain in fwnames.lines() {
            records.push(PacketFilterRecord {
                engine:     "ipchains".to_string(),
                table:      chain.trim().to_string(),
                raw_output: fwchains.clone(),
            });
        }
        if records.is_empty() {
            records.push(PacketFilterRecord {
                engine:     "ipchains".to_string(),
                table:      String::new(),
                raw_output: fwchains,
            });
        }
        return ScopeWrapper { attributes: Default::default(), elements: records };
    }

    // Step 3: iptables
    if fs.exists("/proc/net/ip_tables_names") {
        let tables_content = fs.read_file("/proc/net/ip_tables_names").unwrap_or_default();
        let mut records = Vec::new();
        for table in tables_content.lines() {
            let table = table.trim();
            if table.is_empty() { continue; }
            let raw_output = cr.run("iptables", &["-v", "-L", "-n", "-t", table])
                .map(|(o, _)| o)
                .unwrap_or_else(|e| format!("iptables error: {}", e));
            records.push(PacketFilterRecord {
                engine:     "iptables".to_string(),
                table:      table.to_string(),
                raw_output,
            });
        }
        if !records.is_empty() {
            return ScopeWrapper { attributes: Default::default(), elements: records };
        }
    }

    // Step 4: no packet filter
    ScopeWrapper {
        attributes: Default::default(),
        elements: vec![PacketFilterRecord {
            engine:     "none".to_string(),
            table:      String::new(),
            raw_output: "No packet filter installed.".to_string(),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::{FakeFilesystem, FakeCommandRunner};

    #[test]
    fn test_collect_network_interfaces_loopback() {
        let mut cr = FakeCommandRunner::new();
        cr.responses.insert("ip".to_string(), (
            r#"[{"ifname":"lo","flags":["LOOPBACK","UP"],"mtu":65536,"operstate":"UNKNOWN","link_type":"loopback","addr_info":[{"family":"inet","local":"127.0.0.1","prefixlen":8}]}]"#.to_string(),
            String::new()
        ));
        let scope = collect_network_interfaces(&cr);
        assert_eq!(scope.elements.len(), 1);
        assert_eq!(scope.elements[0].ifname, "lo");
        assert_eq!(scope.elements[0].ip, "127.0.0.1");
    }

    #[test]
    fn test_collect_network_routing_default() {
        let mut cr = FakeCommandRunner::new();
        cr.responses.insert("ip".to_string(), (
            r#"[{"dst":"default","gateway":"192.168.1.1","dev":"eth0","protocol":"dhcp","scope":"","type":"unicast","flags":[]}]"#.to_string(),
            String::new()
        ));
        let scope = collect_network_routing(&cr);
        assert_eq!(scope.elements.len(), 1);
        assert_eq!(scope.elements[0].dst, "default");
        assert_eq!(scope.elements[0].gateway, "192.168.1.1");
    }

    #[test]
    fn test_collect_firewall_none() {
        let fs = FakeFilesystem::new();
        let cr = FakeCommandRunner::new();
        let scope = collect_network_firewall(&fs, &cr);
        assert_eq!(scope.elements.len(), 1);
        assert_eq!(scope.elements[0].engine, "none");
    }

    #[test]
    fn test_collect_firewall_iptables() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/proc/net/ip_tables_names".to_string(), "filter\nnat\n".to_string());
        let mut cr = FakeCommandRunner::new();
        cr.responses.insert("iptables".to_string(), ("Chain INPUT ...\n".to_string(), String::new()));
        let scope = collect_network_firewall(&fs, &cr);
        assert_eq!(scope.elements.len(), 2);
        assert_eq!(scope.elements[0].engine, "iptables");
        assert_eq!(scope.elements[0].table, "filter");
    }
}
