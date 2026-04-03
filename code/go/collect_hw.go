package main

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// collectCPU collects CPU information from /proc/cpuinfo.
func collectCPU(fs Filesystem, cr CommandRunner) *ScopeWrapper[CpuRecord] {
	archOut, _, _ := cr.Run("uname", []string{"-m"})
	arch := strings.TrimSpace(archOut)

	scope := &ScopeWrapper[CpuRecord]{
		Attributes: map[string]interface{}{"architecture": arch},
		Elements:   []CpuRecord{},
	}

	content, err := fs.ReadFile("/proc/cpuinfo")
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-cpu: %v\n", err)
		return scope
	}

	var records []CpuRecord
	var current *CpuRecord

	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimRight(line, "\r")
		if line == "" {
			if current != nil {
				records = append(records, *current)
				current = nil
			}
			continue
		}
		idx := strings.Index(line, ":")
		if idx < 0 {
			continue
		}
		key := strings.TrimSpace(line[:idx])
		val := strings.TrimSpace(line[idx+1:])

		switch key {
		case "processor":
			if current != nil {
				records = append(records, *current)
			}
			current = &CpuRecord{Processor: val}
		case "vendor_id":
			if current != nil {
				current.VendorID = val
			}
		case "model name":
			if current != nil {
				current.ModelName = val
			}
		case "cpu MHz":
			if current != nil {
				current.CpuMHz = val
			}
		case "cache size":
			if current != nil {
				current.CacheSize = val
			}
		case "stepping":
			if current != nil {
				current.Stepping = val
			}
		case "cpu family":
			if current != nil {
				current.CpuFamily = val
			}
		case "model":
			if current != nil && current.Model == "" {
				current.Model = val
			}
		}
	}
	if current != nil {
		records = append(records, *current)
	}

	scope.Elements = records
	return scope
}

// collectKernelParams collects all readable sysctl values under /proc/sys/kernel/.
func collectKernelParams(fs Filesystem) *ScopeWrapper[KernelParamRecord] {
	const base = "/proc/sys/kernel"
	const limit = 32767

	scope := &ScopeWrapper[KernelParamRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []KernelParamRecord{},
	}

	var records []KernelParamRecord
	err := fs.WalkDir(base, func(path string, isDir bool) error {
		if isDir {
			return nil
		}
		content, err := fs.ReadFileLimited(path, limit)
		if err != nil {
			return nil
		}
		content = strings.TrimRight(content, "\n")
		if content == "" {
			return nil
		}
		rel, _ := filepath.Rel(base, path)
		records = append(records, KernelParamRecord{Key: rel, Value: content})
		return nil
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-kernel-params: %v\n", err)
	}

	sort.Slice(records, func(i, j int) bool { return records[i].Key < records[j].Key })
	scope.Elements = records
	return scope
}

// collectNetParams collects all readable sysctl values under /proc/sys/net/.
func collectNetParams(fs Filesystem) *ScopeWrapper[NetParamRecord] {
	const base = "/proc/sys/net"
	const limit = 32767

	subtrees := []string{
		"802", "appletalk", "ax25", "bridge", "core", "decnet",
		"ethernet", "ipv4", "ipv6", "irda", "ipx", "netfilter",
		"rose", "unix", "x25",
	}

	scope := &ScopeWrapper[NetParamRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []NetParamRecord{},
	}

	var records []NetParamRecord
	for _, sub := range subtrees {
		dir := filepath.Join(base, sub)
		if !fs.Exists(dir) {
			continue
		}
		err := fs.WalkDir(dir, func(path string, isDir bool) error {
			if isDir {
				return nil
			}
			content, err := fs.ReadFileLimited(path, limit)
			if err != nil {
				return nil
			}
			content = strings.TrimRight(content, "\n")
			if content == "" {
				return nil
			}
			rel, _ := filepath.Rel(base, path)
			records = append(records, NetParamRecord{Key: rel, Value: content})
			return nil
		})
		if err != nil {
			fmt.Fprintf(os.Stderr, "sitar: collect-net-params: %s: %v\n", sub, err)
		}
	}

	sort.Slice(records, func(i, j int) bool { return records[i].Key < records[j].Key })
	scope.Elements = records
	return scope
}

// collectDevices collects device DMA/IRQ/port assignments from /proc.
func collectDevices(fs Filesystem) *ScopeWrapper[DeviceRecord] {
	scope := &ScopeWrapper[DeviceRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []DeviceRecord{},
	}

	type devInfo struct {
		irqs  []string
		dmas  []string
		ports []string
	}
	devMap := map[string]*devInfo{}

	getOrCreate := func(name string) *devInfo {
		if d, ok := devMap[name]; ok {
			return d
		}
		d := &devInfo{}
		devMap[name] = d
		return d
	}

	// Parse /proc/interrupts
	if content, err := fs.ReadFile("/proc/interrupts"); err == nil {
		for _, line := range strings.Split(content, "\n") {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			// Skip header line (starts with "CPU")
			if strings.HasPrefix(line, "CPU") {
				continue
			}
			fields := strings.Fields(line)
			if len(fields) < 2 {
				continue
			}
			// First field is IRQ number (may end with ":")
			irq := strings.TrimSuffix(fields[0], ":")
			// Device name is typically the last field
			devName := fields[len(fields)-1]
			if devName == "" || devName == irq {
				continue
			}
			d := getOrCreate(devName)
			d.irqs = append(d.irqs, irq)
		}
	} else {
		fmt.Fprintf(os.Stderr, "sitar: collect-devices: /proc/interrupts: %v\n", err)
	}

	// Parse /proc/dma
	if content, err := fs.ReadFile("/proc/dma"); err == nil {
		for _, line := range strings.Split(content, "\n") {
			parts := strings.SplitN(line, ": ", 2)
			if len(parts) != 2 {
				continue
			}
			dma := strings.TrimSpace(parts[0])
			devName := strings.Fields(parts[1])[0]
			devName = strings.Split(devName, "(")[0]
			devName = strings.TrimSpace(devName)
			if devName == "" {
				continue
			}
			d := getOrCreate(devName)
			d.dmas = append(d.dmas, dma)
		}
	}

	// Parse /proc/ioports
	if content, err := fs.ReadFile("/proc/ioports"); err == nil {
		for _, line := range strings.Split(content, "\n") {
			parts := strings.SplitN(line, " : ", 2)
			if len(parts) != 2 {
				continue
			}
			portRange := strings.TrimSpace(parts[0])
			devName := strings.Fields(parts[1])[0]
			devName = strings.Split(devName, "(")[0]
			devName = strings.TrimSpace(devName)
			if devName == "" {
				continue
			}
			d := getOrCreate(devName)
			d.ports = append(d.ports, portRange)
		}
	}

	var records []DeviceRecord
	for name, d := range devMap {
		records = append(records, DeviceRecord{
			Name:  name,
			IRQ:   strings.Join(d.irqs, ","),
			DMA:   strings.Join(d.dmas, ","),
			Ports: strings.Join(d.ports, ","),
		})
	}

	sort.Slice(records, func(i, j int) bool {
		return strings.ToLower(records[i].Name) < strings.ToLower(records[j].Name)
	})

	scope.Elements = records
	return scope
}

// collectPCI collects PCI device information.
func collectPCI(fs Filesystem, cr CommandRunner) *ScopeWrapper[PciRecord] {
	scope := &ScopeWrapper[PciRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []PciRecord{},
	}

	// Try lspci -vm
	stdout, _, err := cr.Run("lspci", []string{"-vm"})
	if err == nil && stdout != "" {
		var records []PciRecord
		var current *PciRecord
		for _, line := range strings.Split(stdout, "\n") {
			line = strings.TrimRight(line, "\r")
			if line == "" {
				if current != nil {
					records = append(records, *current)
					current = nil
				}
				continue
			}
			idx := strings.Index(line, ":")
			if idx < 0 {
				continue
			}
			key := strings.TrimSpace(line[:idx])
			val := strings.TrimSpace(line[idx+1:])
			switch key {
			case "Slot":
				if current != nil {
					records = append(records, *current)
				}
				current = &PciRecord{PCI: val}
			case "Device":
				if current != nil && current.Device == "" {
					current.Device = val
				}
			case "Class":
				if current != nil {
					current.Class = val
				}
			case "Vendor":
				if current != nil {
					current.Vendor = val
				}
			case "SVendor":
				if current != nil {
					current.SVendor = val
				}
			case "SDevice":
				if current != nil {
					current.SDevice = val
				}
			case "Rev":
				if current != nil {
					current.Rev = val
				}
			}
		}
		if current != nil {
			records = append(records, *current)
		}
		scope.Elements = records
		return scope
	}

	// Fallback: try /proc/pci
	if content, err := fs.ReadFile("/proc/pci"); err == nil {
		var records []PciRecord
		for _, line := range strings.Split(content, "\n") {
			line = strings.TrimSpace(line)
			if line == "" {
				continue
			}
			records = append(records, PciRecord{Device: line})
		}
		scope.Elements = records
	}

	return scope
}

// collectProcesses collects process list from /proc.
func collectProcesses(fs Filesystem) *ScopeWrapper[ProcessRecord] {
	scope := &ScopeWrapper[ProcessRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []ProcessRecord{},
	}

	// Read /proc directly — only look at top-level numeric directories.
	// Do NOT use WalkDir on /proc: it recurses into /proc/sys, /proc/net,
	// /proc/tty etc. and never terminates on a live system.
	entries, err := os.ReadDir("/proc")
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-processes: ReadDir /proc: %v\n", err)
		return scope
	}

	var records []ProcessRecord
	for _, entry := range entries {
		if !entry.IsDir() {
			continue
		}
		pid := entry.Name()
		if _, err := strconv.Atoi(pid); err != nil {
			continue // skip non-numeric entries (net, sys, tty, etc.)
		}

		statContent, err := fs.ReadFile("/proc/" + pid + "/stat")
		if err != nil {
			continue // process may have exited
		}

		// Parse: pid (comm) state ppid ...
		// comm is enclosed in parens and may contain spaces.
		openParen := strings.Index(statContent, "(")
		closeParen := strings.LastIndex(statContent, ")")
		if openParen < 0 || closeParen < 0 || closeParen <= openParen {
			continue
		}
		comm := statContent[openParen+1 : closeParen]
		rest := strings.TrimSpace(statContent[closeParen+1:])
		fields := strings.Fields(rest)
		state := ""
		ppid := ""
		if len(fields) >= 1 {
			state = fields[0]
		}
		if len(fields) >= 2 {
			ppid = fields[1]
		}

		cmdlineContent, _ := fs.ReadFile("/proc/" + pid + "/cmdline")
		cmdline := strings.ReplaceAll(cmdlineContent, "\x00", " ")
		cmdline = strings.TrimRight(cmdline, " \n")

		records = append(records, ProcessRecord{
			PID:     pid,
			PPID:    ppid,
			Comm:    comm,
			State:   state,
			CmdLine: cmdline,
		})
	}

	sort.Slice(records, func(i, j int) bool {
		a, _ := strconv.Atoi(records[i].PID)
		b, _ := strconv.Atoi(records[j].PID)
		return a < b
	})

	scope.Elements = records
	return scope
}

// collectDMI collects DMI/SMBIOS information via dmidecode.
func collectDMI(fs Filesystem, cr CommandRunner) *DmiScope {
	paths := []string{"/usr/sbin/dmidecode", "/sbin/dmidecode", "/usr/bin/dmidecode"}
	found := false
	for _, p := range paths {
		if fs.IsExecutable(p) {
			found = true
			break
		}
	}
	if !found {
		return nil
	}

	stdout, _, err := cr.Run("dmidecode", []string{})
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-dmi: %v\n", err)
		return nil
	}
	return &DmiScope{RawOutput: stdout}
}
