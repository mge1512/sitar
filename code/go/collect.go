package main

import (
	"fmt"
	"math"
	"os"
	"strconv"
	"strings"
	"time"
)

func getHostname(cr CommandRunner) string {
	stdout, _, _ := cr.Run("hostname", []string{"-f"})
	return strings.TrimSpace(stdout)
}

func getUname(cr CommandRunner) string {
	stdout, _, _ := cr.Run("uname", []string{"-a"})
	return strings.TrimSpace(stdout)
}

func collect(config *Config, fs Filesystem, cr CommandRunner) *SitarManifest {
	if os.Geteuid() != 0 {
		fmt.Fprintf(os.Stderr, "Please run sitar as user root.\n")
		os.Exit(1)
	}

	manifest := &SitarManifest{}
	manifest.Meta = SitarMeta{
		FormatVersion: 1,
		SitarVersion:  "0.9.0",
		CollectedAt:   time.Now().UTC().Format(time.RFC3339),
		Hostname:      getHostname(cr),
		Uname:         getUname(cr),
	}

	distro := detectDistribution(fs)

	safeRun := func(name string, fn func()) {
		fmt.Fprintf(os.Stderr, "sitar: starting %s\n", name)
		defer func() {
			if r := recover(); r != nil {
				fmt.Fprintf(os.Stderr, "sitar: module %s panicked: %v\n", name, r)
			} else {
				fmt.Fprintf(os.Stderr, "sitar: finished %s\n", name)
			}
		}()
		fn()
	}

	safeRun("collect-general-info", func() {
		manifest.GeneralInfo = collectGeneralInfo(fs, cr, distro)
	})
	safeRun("collect-environment", func() {
		manifest.Environment = collectEnvironment(fs)
	})
	safeRun("collect-os", func() {
		manifest.Os = collectOS(fs, cr, distro)
	})
	safeRun("collect-cpu", func() {
		manifest.CPU = collectCPU(fs, cr)
	})
	safeRun("collect-kernel-params", func() {
		manifest.KernelParams = collectKernelParams(fs)
	})
	safeRun("collect-net-params", func() {
		manifest.NetParams = collectNetParams(fs)
	})
	safeRun("collect-devices", func() {
		manifest.Devices = collectDevices(fs)
	})
	safeRun("collect-pci", func() {
		manifest.PCI = collectPCI(fs, cr)
	})
	safeRun("collect-storage", func() {
		manifest.Storage = collectStorage(fs, cr)
	})
	safeRun("collect-btrfs", func() {
		if manifest.Storage != nil {
			manifest.Storage.Btrfs = collectBtrfs(fs, cr, manifest.Storage)
		}
	})

	manifest.Network = &NetworkScope{}
	safeRun("collect-network-interfaces", func() {
		manifest.Network.Interfaces = collectNetworkInterfaces(cr)
	})
	safeRun("collect-network-routing", func() {
		manifest.Network.Routes = collectNetworkRouting(cr)
	})
	safeRun("collect-network-firewall", func() {
		manifest.Network.PacketFilter = collectNetworkFirewall(fs, cr)
	})

	safeRun("collect-security-apparmor", func() {
		manifest.SecurityApparmor = collectSecurityApparmor(fs, cr, config)
	})
	safeRun("collect-processes", func() {
		manifest.Processes = collectProcesses(fs)
	})
	safeRun("collect-dmi", func() {
		manifest.DMI = collectDMI(fs, cr)
	})

	// Slow RPM operations get a dedicated long-timeout runner (15 min each).
	slowCR := &OSCommandRunner{Timeout: 15 * time.Minute}

	if config.FindUnpacked {
		safeRun("find-unpacked", func() {
			_, err := findUnpacked(slowCR)
			if err != nil {
				fmt.Fprintf(os.Stderr, "sitar: find-unpacked: %v\n", err)
			}
		})
	}
	if config.CheckConsistency {
		safeRun("check-consistency", func() {
			_, err := checkConsistency(slowCR)
			if err != nil {
				fmt.Fprintf(os.Stderr, "sitar: check-consistency: %v\n", err)
			}
		})
	}

	switch distro.Family {
	case FamilyRPM:
		safeRun("collect-installed-rpm", func() {
			manifest.Packages, manifest.Patterns = collectInstalledRPM(cr)
		})
		safeRun("collect-repositories", func() {
			manifest.Repositories = collectRepositories(fs, cr, distro.Backend)
		})
		safeRun("collect-services", func() {
			manifest.Services = collectServices(fs, cr)
		})
		safeRun("collect-groups", func() {
			manifest.Groups = collectGroups(fs)
		})
		safeRun("collect-users", func() {
			manifest.Users = collectUsers(fs, config)
		})
		safeRun("collect-changed-config-files", func() {
			manifest.ChangedConfigFiles = collectChangedConfigFiles(cr)
		})
		safeRun("collect-changed-managed-files", func() {
			manifest.ChangedManagedFiles = collectChangedManagedFiles(cr)
		})
		safeRun("collect-kernel-config", func() {
			manifest.KernelConfig = collectKernelConfig(fs, cr)
		})
	case FamilyDeb:
		safeRun("collect-installed-deb", func() {
			manifest.Packages = collectInstalledDeb(fs)
		})
		safeRun("collect-groups", func() {
			manifest.Groups = collectGroups(fs)
		})
		safeRun("collect-users", func() {
			manifest.Users = collectUsers(fs, config)
		})
		safeRun("collect-kernel-config", func() {
			manifest.KernelConfig = collectKernelConfig(fs, cr)
		})
	}

	return manifest
}

// collectGeneralInfo collects basic system identity and runtime metrics.
func collectGeneralInfo(fs Filesystem, cr CommandRunner, distro DistroInfo) *ScopeWrapper[GeneralInfoRecord] {
	hostname := getHostname(cr)
	osRelease := distro.Release
	uname := getUname(cr)
	collectedAt := time.Now().Format("Mon Jan 2 15:04:05 MST 2006")

	memTotalKB := "0"
	if content, err := fs.ReadFile("/proc/meminfo"); err == nil {
		for _, line := range strings.Split(content, "\n") {
			if strings.HasPrefix(line, "MemTotal:") {
				fields := strings.Fields(line)
				if len(fields) >= 2 {
					memTotalKB = fields[1]
				}
				break
			}
		}
	}

	cmdline := ""
	if content, err := fs.ReadFile("/proc/cmdline"); err == nil {
		cmdline = strings.TrimRight(content, "\n")
	}

	loadavg := ""
	if content, err := fs.ReadFile("/proc/loadavg"); err == nil {
		loadavg = strings.TrimRight(content, "\n")
	}

	uptimeMin := "0"
	idletimeMin := "0"
	if content, err := fs.ReadFile("/proc/uptime"); err == nil {
		fields := strings.Fields(content)
		if len(fields) >= 2 {
			if upSec, err := strconv.ParseFloat(fields[0], 64); err == nil {
				uptimeMin = strconv.Itoa(int(math.Floor(upSec / 60)))
			}
			if idleSec, err := strconv.ParseFloat(fields[1], 64); err == nil {
				idletimeMin = strconv.Itoa(int(math.Floor(idleSec / 60)))
			}
		}
	}

	records := []GeneralInfoRecord{
		{Key: "hostname", Value: hostname},
		{Key: "os_release", Value: osRelease},
		{Key: "uname", Value: uname},
		{Key: "collected_at", Value: collectedAt},
		{Key: "mem_total_kb", Value: memTotalKB},
		{Key: "cmdline", Value: cmdline},
		{Key: "loadavg", Value: loadavg},
		{Key: "uptime_min", Value: uptimeMin},
		{Key: "idletime_min", Value: idletimeMin},
	}

	return &ScopeWrapper[GeneralInfoRecord]{
		Attributes: map[string]interface{}{},
		Elements:   records,
	}
}

// collectEnvironment collects system environment metadata.
func collectEnvironment(fs Filesystem) *EnvironmentScope {
	locale := "C"
	localeFiles := []string{"/etc/locale.conf", "/etc/default/locale", "/etc/sysconfig/language"}
	for _, f := range localeFiles {
		if content, err := fs.ReadFile(f); err == nil {
			for _, line := range strings.Split(content, "\n") {
				line = strings.TrimSpace(line)
				if strings.HasPrefix(line, "LANG=") || strings.HasPrefix(line, "LC_ALL=") {
					idx := strings.IndexByte(line, '=')
					if idx >= 0 {
						val := strings.Trim(line[idx+1:], `"'`)
						if val != "" {
							locale = val
							goto localeFound
						}
					}
				}
			}
		}
	}
localeFound:

	systemType := "local"
	if fs.Exists("/.dockerenv") {
		systemType = "docker"
	} else if content, err := fs.ReadFile("/proc/1/cgroup"); err == nil {
		if strings.Contains(content, "docker") {
			systemType = "docker"
		}
	}
	if systemType == "local" {
		if content, err := fs.ReadFile("/proc/1/environ"); err == nil {
			if strings.Contains(content, "container") {
				systemType = "remote"
			}
		}
	}

	return &EnvironmentScope{
		Locale:     locale,
		SystemType: systemType,
	}
}

// collectOS collects OS identification.
func collectOS(fs Filesystem, cr CommandRunner, distro DistroInfo) *OsScope {
	scope := &OsScope{}

	if content, err := fs.ReadFile("/etc/os-release"); err == nil {
		kv := parseOsRelease(content)
		if name, ok := kv["NAME"]; ok && name != "" {
			scope.Name = &name
		}
		if ver, ok := kv["VERSION"]; ok {
			scope.Version = &ver
		} else if vid, ok := kv["VERSION_ID"]; ok {
			scope.Version = &vid
		}
	} else {
		if distro.Release != "" {
			r := distro.Release
			scope.Name = &r
		}
		fmt.Fprintf(os.Stderr, "sitar: collect-os: /etc/os-release unreadable: %v\n", err)
	}

	archOut, _, _ := cr.Run("uname", []string{"-m"})
	arch := strings.TrimSpace(archOut)
	if arch != "" {
		scope.Architecture = &arch
	}

	return scope
}
