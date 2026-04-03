package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

// collectNetworkInterfaces collects network interface configuration.
func collectNetworkInterfaces(cr CommandRunner) *ScopeWrapper[NetworkInterfaceRecord] {
	scope := &ScopeWrapper[NetworkInterfaceRecord]{
		Attributes: map[string]interface{}{"command": "ip"},
		Elements:   []NetworkInterfaceRecord{},
	}

	stdout, _, err := cr.Run("ip", []string{"-j", "addr", "show"})
	if err != nil || stdout == "" {
		fmt.Fprintf(os.Stderr, "sitar: collect-network-interfaces: ip -j addr show failed: %v\n", err)
		return scope
	}

	// ip -j addr show JSON structure
	type addrInfo struct {
		Family    string `json:"family"`
		Local     string `json:"local"`
		PrefixLen int    `json:"prefixlen"`
		Broadcast string `json:"broadcast"`
		Scope     string `json:"scope"`
	}
	type ipInterface struct {
		IfIndex   int        `json:"ifindex"`
		IfName    string     `json:"ifname"`
		Flags     []string   `json:"flags"`
		Mtu       int        `json:"mtu"`
		LinkType  string     `json:"link_type"`
		Address   string     `json:"address"`
		OperState string     `json:"operstate"`
		AddrInfo  []addrInfo `json:"addr_info"`
	}

	var ifaces []ipInterface
	if err := json.Unmarshal([]byte(stdout), &ifaces); err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-network-interfaces: JSON parse error: %v\n", err)
		return scope
	}

	var records []NetworkInterfaceRecord
	for _, iface := range ifaces {
		rec := NetworkInterfaceRecord{
			IfName:    iface.IfName,
			LinkType:  iface.LinkType,
			Address:   iface.Address,
			Flags:     iface.Flags,
			MTU:       iface.Mtu,
			OperState: iface.OperState,
		}
		if rec.Flags == nil {
			rec.Flags = []string{}
		}

		for _, ai := range iface.AddrInfo {
			if ai.Family == "inet" && rec.IP == "" {
				rec.IP = ai.Local
				rec.PrefixLen = fmt.Sprintf("%d", ai.PrefixLen)
				rec.Broadcast = ai.Broadcast
			} else if ai.Family == "inet6" && ai.Scope != "link" && rec.IP6 == "" {
				rec.IP6 = ai.Local
				rec.IP6PrefixLen = fmt.Sprintf("%d", ai.PrefixLen)
			}
		}

		records = append(records, rec)
	}

	scope.Elements = records
	return scope
}

// collectNetworkRouting collects IP routing table.
func collectNetworkRouting(cr CommandRunner) *ScopeWrapper[RouteRecord] {
	scope := &ScopeWrapper[RouteRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []RouteRecord{},
	}

	stdout, _, err := cr.Run("ip", []string{"-j", "route", "show"})
	if err != nil || stdout == "" {
		fmt.Fprintf(os.Stderr, "sitar: collect-network-routing: ip -j route show failed: %v\n", err)
		return scope
	}

	type ipRoute struct {
		Dst      string   `json:"dst"`
		Gateway  string   `json:"gateway"`
		Dev      string   `json:"dev"`
		Protocol string   `json:"protocol"`
		Scope    string   `json:"scope"`
		Type     string   `json:"type"`
		Metric   int      `json:"metric"`
		Flags    []string `json:"flags"`
	}

	var routes []ipRoute
	if err := json.Unmarshal([]byte(stdout), &routes); err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-network-routing: JSON parse error: %v\n", err)
		return scope
	}

	var records []RouteRecord
	for _, r := range routes {
		dst := r.Dst
		if dst == "" || dst == "0.0.0.0/0" {
			dst = "default"
		}
		flags := r.Flags
		if flags == nil {
			flags = []string{}
		}
		metric := ""
		if r.Metric != 0 {
			metric = fmt.Sprintf("%d", r.Metric)
		}
		records = append(records, RouteRecord{
			Dst:      dst,
			Gateway:  r.Gateway,
			Dev:      r.Dev,
			Protocol: r.Protocol,
			Scope:    r.Scope,
			Type:     r.Type,
			Metric:   metric,
			Flags:    flags,
		})
	}

	scope.Elements = records
	return scope
}

// collectNetworkFirewall collects packet filter rules.
func collectNetworkFirewall(fs Filesystem, cr CommandRunner) *ScopeWrapper[PacketFilterRecord] {
	scope := &ScopeWrapper[PacketFilterRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []PacketFilterRecord{},
	}

	// Step 1: ipfwadm
	if fs.Exists("/proc/net/ip_input") {
		scope.Elements = []PacketFilterRecord{
			{Engine: "ipfwadm", RawOutput: "ipfwadm is not supported."},
		}
		return scope
	}

	// Step 2: ipchains
	if fs.Exists("/proc/net/ip_fwnames") {
		var records []PacketFilterRecord
		if content, err := fs.ReadFile("/proc/net/ip_fwchains"); err == nil {
			records = append(records, PacketFilterRecord{
				Engine:    "ipchains",
				Table:     "filter",
				RawOutput: content,
			})
		}
		scope.Elements = records
		return scope
	}

	// Step 3: iptables
	if fs.Exists("/proc/net/ip_tables_names") && fs.IsExecutable("/sbin/iptables") {
		content, err := fs.ReadFile("/proc/net/ip_tables_names")
		if err == nil {
			var records []PacketFilterRecord
			for _, table := range strings.Split(strings.TrimSpace(content), "\n") {
				table = strings.TrimSpace(table)
				if table == "" {
					continue
				}
				stdout, _, err := cr.Run("iptables", []string{"-v", "-L", "-n", "-t", table})
				if err != nil {
					fmt.Fprintf(os.Stderr, "sitar: iptables -t %s: %v\n", table, err)
					continue
				}
				records = append(records, PacketFilterRecord{
					Engine:    "iptables",
					Table:     table,
					RawOutput: stdout,
				})
			}
			scope.Elements = records
			return scope
		}
	}

	// Step 4: none
	scope.Elements = []PacketFilterRecord{
		{Engine: "none", RawOutput: "No packet filter installed."},
	}
	return scope
}
