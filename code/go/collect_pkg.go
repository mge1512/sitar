package main

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// collectInstalledRPM collects installed RPM packages.
func collectInstalledRPM(cr CommandRunner) (*ScopeWrapper[PackageRecord], *ScopeWrapper[PatternRecord]) {
	pkgScope := &ScopeWrapper[PackageRecord]{
		Attributes: map[string]interface{}{"package_system": "rpm"},
		Elements:   []PackageRecord{},
	}
	patScope := &ScopeWrapper[PatternRecord]{
		Attributes: map[string]interface{}{"patterns_system": "zypper"},
		Elements:   []PatternRecord{},
	}

	// Query format: NAME::VERSION-RELEASE::SIZE::SUMMARY::DISTRIBUTION::PACKAGER::ARCH
	format := `%{NAME}::%{VERSION}-%{RELEASE}::%{SIZE}::%{SUMMARY}::%{DISTRIBUTION}::%{PACKAGER}::%{ARCH}::a\n`
	stdout, _, err := cr.Run("rpm", []string{"-qa", "--queryformat", format})
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-installed-rpm: %v\n", err)
		return pkgScope, patScope
	}

	var records []PackageRecord
	for _, line := range strings.Split(stdout, "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		parts := strings.Split(line, "::")
		if len(parts) < 7 {
			continue
		}
		name := parts[0]
		versionRelease := parts[1]
		sizeStr := parts[2]
		summary := parts[3]
		distribution := parts[4]
		packager := parts[5]
		arch := parts[6]

		// Split version-release
		version := versionRelease
		release := ""
		if idx := strings.LastIndex(versionRelease, "-"); idx >= 0 {
			version = versionRelease[:idx]
			release = versionRelease[idx+1:]
		}

		var size int64
		if v, err := strconv.ParseInt(sizeStr, 10, 64); err == nil {
			size = v
		}

		rec := PackageRecord{
			Name:         name,
			Version:      version,
			Release:      release,
			Arch:         arch,
			Summary:      summary,
			Distribution: distribution,
			Packager:     packager,
			Size:         size,
		}
		records = append(records, rec)
	}

	// Get checksums (batched per package would be slow; skip for now - collect separately if needed)
	// For performance, we skip individual checksum queries in bulk collection

	sort.Slice(records, func(i, j int) bool {
		return records[i].Name < records[j].Name
	})
	pkgScope.Elements = records

	// Collect patterns via zypper if available
	patternsOut, _, err := cr.Run("zypper", []string{"patterns", "--installed-only"})
	if err == nil {
		for _, line := range strings.Split(patternsOut, "\n") {
			line = strings.TrimSpace(line)
			if line == "" || strings.HasPrefix(line, "#") || strings.HasPrefix(line, "S") {
				continue
			}
			fields := strings.Fields(line)
			if len(fields) >= 3 {
				patScope.Elements = append(patScope.Elements, PatternRecord{
					Name:    fields[1],
					Version: fields[2],
				})
			}
		}
	}

	return pkgScope, patScope
}

// collectInstalledDeb collects installed Debian packages from dpkg status file.
func collectInstalledDeb(fs Filesystem) *ScopeWrapper[PackageRecord] {
	scope := &ScopeWrapper[PackageRecord]{
		Attributes: map[string]interface{}{"package_system": "dpkg"},
		Elements:   []PackageRecord{},
	}

	content, err := fs.ReadFile("/var/lib/dpkg/status")
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-installed-deb: %v\n", err)
		return scope
	}

	var records []PackageRecord
	var current map[string]string

	flush := func() {
		if current == nil {
			return
		}
		status := current["Status"]
		if !strings.HasSuffix(status, "installed") {
			return
		}
		rec := PackageRecord{
			Name:    current["Package"],
			Version: current["Version"],
			Arch:    current["Architecture"],
			Summary: current["Description"],
		}
		if sizeStr, ok := current["Installed-Size"]; ok {
			if v, err := strconv.ParseInt(strings.TrimSpace(sizeStr), 10, 64); err == nil {
				rec.Size = v * 1024 // Convert KiB to bytes
			}
		}
		records = append(records, rec)
	}

	for _, line := range strings.Split(content, "\n") {
		if line == "" {
			flush()
			current = nil
			continue
		}
		if current == nil {
			current = map[string]string{}
		}
		if idx := strings.Index(line, ": "); idx >= 0 {
			key := line[:idx]
			val := line[idx+2:]
			if key == "Description" {
				// Only take first line of description
				current[key] = val
			} else {
				current[key] = val
			}
		}
	}
	flush()

	sort.Slice(records, func(i, j int) bool {
		return records[i].Name < records[j].Name
	})
	scope.Elements = records
	return scope
}

// collectRepositories collects configured package repositories.
func collectRepositories(fs Filesystem, cr CommandRunner, backend PackageVersioningBackend) *ScopeWrapper[RepositoryRecord] {
	scope := &ScopeWrapper[RepositoryRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []RepositoryRecord{},
	}

	// Try zypp repos
	for _, repoDir := range []string{"/etc/zypp/repos.d", "/etc/zypper/repos.d"} {
		if fs.Exists(repoDir) {
			files, _ := fs.Glob(filepath.Join(repoDir, "*.repo"))
			var records []RepositoryRecord
			for _, f := range files {
				content, err := fs.ReadFile(f)
				if err != nil {
					continue
				}
				records = append(records, parseRepoFile(content, "zypp")...)
			}
			scope.Attributes["repository_system"] = "zypp"
			scope.Elements = records
			return scope
		}
	}

	// Try yum repos
	if fs.Exists("/etc/yum.repos.d") {
		files, _ := fs.Glob("/etc/yum.repos.d/*.repo")
		var records []RepositoryRecord
		for _, f := range files {
			content, err := fs.ReadFile(f)
			if err != nil {
				continue
			}
			records = append(records, parseRepoFile(content, "yum")...)
		}
		scope.Attributes["repository_system"] = "yum"
		scope.Elements = records
		return scope
	}

	// Try apt sources
	for _, srcFile := range []string{"/etc/apt/sources.list"} {
		if fs.Exists(srcFile) {
			content, err := fs.ReadFile(srcFile)
			if err != nil {
				continue
			}
			var records []RepositoryRecord
			for _, line := range strings.Split(content, "\n") {
				line = strings.TrimSpace(line)
				if line == "" || strings.HasPrefix(line, "#") {
					continue
				}
				fields := strings.Fields(line)
				if len(fields) < 3 {
					continue
				}
				rec := RepositoryRecord{
					Type:         fields[0],
					URL:          fields[1],
					Distribution: fields[2],
					Components:   fields[3:],
				}
				records = append(records, rec)
			}
			scope.Attributes["repository_system"] = "apt"
			scope.Elements = records
			return scope
		}
	}

	// Try apt sources.list.d
	if fs.Exists("/etc/apt/sources.list.d") {
		files, _ := fs.Glob("/etc/apt/sources.list.d/*.list")
		var records []RepositoryRecord
		for _, f := range files {
			content, err := fs.ReadFile(f)
			if err != nil {
				continue
			}
			for _, line := range strings.Split(content, "\n") {
				line = strings.TrimSpace(line)
				if line == "" || strings.HasPrefix(line, "#") {
					continue
				}
				fields := strings.Fields(line)
				if len(fields) < 3 {
					continue
				}
				rec := RepositoryRecord{
					Type:         fields[0],
					URL:          fields[1],
					Distribution: fields[2],
					Components:   fields[3:],
				}
				records = append(records, rec)
			}
		}
		if len(records) > 0 {
			scope.Attributes["repository_system"] = "apt"
			scope.Elements = records
			return scope
		}
	}

	return nil
}

func parseRepoFile(content, system string) []RepositoryRecord {
	var records []RepositoryRecord
	var current map[string]string
	var currentName string

	flush := func() {
		if current == nil || currentName == "" {
			return
		}
		rec := RepositoryRecord{
			Alias:       currentName,
			Name:        current["name"],
			URL:         current["baseurl"],
			Type:        current["type"],
			Enabled:     current["enabled"],
			GPGCheck:    current["gpgcheck"],
			AutoRefresh: current["autorefresh"],
			Priority:    current["priority"],
		}
		records = append(records, rec)
	}

	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		if strings.HasPrefix(line, "[") && strings.HasSuffix(line, "]") {
			flush()
			currentName = line[1 : len(line)-1]
			current = map[string]string{}
			continue
		}
		if current != nil {
			if idx := strings.Index(line, "="); idx >= 0 {
				key := strings.TrimSpace(line[:idx])
				val := strings.TrimSpace(line[idx+1:])
				current[key] = val
			}
		}
	}
	flush()
	return records
}

// collectServices collects service startup configuration.
func collectServices(fs Filesystem, cr CommandRunner) *ScopeWrapper[ServiceRecord] {
	scope := &ScopeWrapper[ServiceRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []ServiceRecord{},
	}

	// Detect init system
	initSystem := ""
	if fs.Exists("/run/systemd/private") || fs.Exists("/sys/fs/cgroup/systemd") {
		initSystem = "systemd"
	} else if fs.IsExecutable("/sbin/chkconfig") || fs.IsExecutable("/usr/sbin/chkconfig") {
		initSystem = "sysvinit"
	} else if fs.IsExecutable("/sbin/initctl") || fs.IsExecutable("/usr/sbin/initctl") {
		initSystem = "upstart"
	} else {
		return nil
	}

	scope.Attributes["init_system"] = initSystem

	switch initSystem {
	case "systemd":
		stdout, _, err := cr.Run("systemctl", []string{"list-unit-files", "--type=service", "--no-legend"})
		if err != nil {
			fmt.Fprintf(os.Stderr, "sitar: collect-services: systemctl: %v\n", err)
			return nil
		}
		var records []ServiceRecord
		for _, line := range strings.Split(stdout, "\n") {
			fields := strings.Fields(line)
			if len(fields) < 2 {
				continue
			}
			records = append(records, ServiceRecord{
				Name:  fields[0],
				State: fields[1],
			})
		}
		scope.Elements = records

	case "sysvinit":
		stdout, _, err := cr.Run("chkconfig", []string{"--list"})
		if err != nil {
			fmt.Fprintf(os.Stderr, "sitar: collect-services: chkconfig: %v\n", err)
			return nil
		}
		var records []ServiceRecord
		for _, line := range strings.Split(stdout, "\n") {
			fields := strings.Fields(line)
			if len(fields) < 2 {
				continue
			}
			name := fields[0]
			state := "disabled"
			// Check runlevel 3 or 5
			for _, f := range fields[1:] {
				if strings.Contains(f, "3:on") || strings.Contains(f, "5:on") {
					state = "enabled"
					break
				}
			}
			records = append(records, ServiceRecord{Name: name, State: state})
		}
		scope.Elements = records
	}

	return scope
}

// collectChkconfig is an alias for collectServices for RPM-family systems.
func collectChkconfig(fs Filesystem, cr CommandRunner) *ScopeWrapper[ServiceRecord] {
	return collectServices(fs, cr)
}

// collectGroups collects system groups from /etc/group.
func collectGroups(fs Filesystem) *ScopeWrapper[GroupRecord] {
	scope := &ScopeWrapper[GroupRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []GroupRecord{},
	}

	content, err := fs.ReadFile("/etc/group")
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-groups: %v\n", err)
		return scope
	}

	var records []GroupRecord
	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		parts := strings.Split(line, ":")
		if len(parts) < 4 {
			continue
		}
		name := parts[0]
		password := parts[1]
		gidStr := parts[2]
		members := parts[3]

		var gid *int
		if g, err := strconv.Atoi(gidStr); err == nil {
			gid = &g
		}

		var users []string
		if members != "" {
			users = strings.Split(members, ",")
		} else {
			users = []string{}
		}

		records = append(records, GroupRecord{
			Name:     name,
			Password: password,
			GID:      gid,
			Users:    users,
		})
	}

	sort.Slice(records, func(i, j int) bool {
		return records[i].Name < records[j].Name
	})
	scope.Elements = records
	return scope
}

// collectUsers collects system users from /etc/passwd and optionally /etc/shadow.
func collectUsers(fs Filesystem, config *Config) *ScopeWrapper[UserRecord] {
	scope := &ScopeWrapper[UserRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []UserRecord{},
	}

	content, err := fs.ReadFile("/etc/passwd")
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-users: /etc/passwd: %v\n", err)
		os.Exit(1)
	}

	userMap := map[string]*UserRecord{}
	var userOrder []string

	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		parts := strings.Split(line, ":")
		if len(parts) < 7 {
			continue
		}
		name := parts[0]
		password := parts[1]
		uidStr := parts[2]
		gidStr := parts[3]
		comment := parts[4]
		home := parts[5]
		shell := parts[6]

		var uid, gid *int
		if u, err := strconv.Atoi(uidStr); err == nil {
			uid = &u
		}
		if g, err := strconv.Atoi(gidStr); err == nil {
			gid = &g
		}

		rec := &UserRecord{
			Name:     name,
			Password: password,
			UID:      uid,
			GID:      gid,
			Comment:  comment,
			Home:     home,
			Shell:    shell,
		}
		userMap[name] = rec
		userOrder = append(userOrder, name)
	}

	// Check if /etc/shadow is excluded
	shadowExcluded := false
	for _, ex := range config.Exclude {
		if ex == "/etc/shadow" {
			shadowExcluded = true
			break
		}
	}

	if !shadowExcluded {
		if shadowContent, err := fs.ReadFile("/etc/shadow"); err == nil {
			for _, line := range strings.Split(shadowContent, "\n") {
				line = strings.TrimSpace(line)
				if line == "" || strings.HasPrefix(line, "#") {
					continue
				}
				parts := strings.Split(line, ":")
				if len(parts) < 2 {
					continue
				}
				name := parts[0]
				if rec, ok := userMap[name]; ok {
					rec.EncryptedPassword = parts[1]
					if len(parts) >= 3 {
						if v, err := strconv.Atoi(parts[2]); err == nil {
							rec.LastChangedDate = v
						}
					}
					if len(parts) >= 4 {
						if v, err := strconv.Atoi(parts[3]); err == nil {
							rec.MinDays = v
						}
					}
					if len(parts) >= 5 {
						if v, err := strconv.Atoi(parts[4]); err == nil {
							rec.MaxDays = v
						}
					}
					if len(parts) >= 6 {
						if v, err := strconv.Atoi(parts[5]); err == nil {
							rec.WarnDays = v
						}
					}
					if len(parts) >= 7 {
						if v, err := strconv.Atoi(parts[6]); err == nil {
							rec.DisableDays = v
						}
					}
					if len(parts) >= 8 {
						if v, err := strconv.Atoi(parts[7]); err == nil {
							rec.DisabledDate = v
						}
					}
				}
			}
		}
	}

	sort.Strings(userOrder)
	var records []UserRecord
	for _, name := range userOrder {
		if rec, ok := userMap[name]; ok {
			records = append(records, *rec)
		}
	}

	scope.Elements = records
	return scope
}

// collectChangedConfigFiles identifies RPM config files that differ from packaged state.
func collectChangedConfigFiles(cr CommandRunner) *ScopeWrapper[ChangedConfigFileRecord] {
	scope := &ScopeWrapper[ChangedConfigFileRecord]{
		Attributes: map[string]interface{}{"extracted": false},
		Elements:   []ChangedConfigFileRecord{},
	}

	// Get list of RPM config files with package names
	stdout, _, err := cr.Run("rpm", []string{"-qca", "--queryformat", "%{NAME}\n"})
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-changed-config-files: rpm -qca: %v\n", err)
		return scope
	}

	pkgSet := map[string]bool{}
	for _, line := range strings.Split(stdout, "\n") {
		line = strings.TrimSpace(line)
		if line != "" {
			pkgSet[line] = true
		}
	}

	var records []ChangedConfigFileRecord
	for pkg := range pkgSet {
		verOut, _, err := cr.Run("rpm", []string{"-V", "--nodeps", "--noscript", pkg})
		if err != nil && verOut == "" {
			continue
		}
		for _, line := range strings.Split(verOut, "\n") {
			line = strings.TrimRight(line, "\r")
			if len(line) < 12 {
				continue
			}
			flags := line[:9]
			fileType := strings.TrimSpace(string(line[9]))
			if fileType != "c" {
				continue
			}
			path := strings.TrimSpace(line[11:])
			if path == "" {
				continue
			}
			var changes []string
			if len(flags) >= 1 && flags[0] == 'S' {
				changes = append(changes, "size")
			}
			if len(flags) >= 2 && flags[1] == 'M' {
				changes = append(changes, "mode")
			}
			if len(flags) >= 3 && flags[2] == '5' {
				changes = append(changes, "md5")
			}
			if len(flags) >= 6 && flags[5] == 'U' {
				changes = append(changes, "user")
			}
			if len(flags) >= 7 && flags[6] == 'G' {
				changes = append(changes, "group")
			}
			if len(flags) >= 8 && flags[7] == 'T' {
				changes = append(changes, "time")
			}
			rec := ChangedConfigFileRecord{
				Name:        path,
				PackageName: pkg,
				Status:      "changed",
				Changes:     changes,
			}
			records = append(records, rec)
		}
	}

	scope.Elements = records
	return scope
}

// collectChangedManagedFiles identifies RPM non-config files that differ from packaged state.
func collectChangedManagedFiles(cr CommandRunner) *ScopeWrapper[ChangedManagedFileRecord] {
	scope := &ScopeWrapper[ChangedManagedFileRecord]{
		Attributes: map[string]interface{}{"extracted": false},
		Elements:   []ChangedManagedFileRecord{},
	}

	stdout, _, err := cr.Run("rpm", []string{"-Va", "--nodeps", "--noscript"})
	if err != nil && stdout == "" {
		fmt.Fprintf(os.Stderr, "sitar: collect-changed-managed-files: rpm -Va: %v\n", err)
		return scope
	}

	var records []ChangedManagedFileRecord
	for _, line := range strings.Split(stdout, "\n") {
		line = strings.TrimRight(line, "\r")
		if len(line) < 12 {
			continue
		}
		fileType := strings.TrimSpace(string(line[9]))
		if fileType == "c" {
			continue // Skip config files
		}
		path := strings.TrimSpace(line[11:])
		if path == "" {
			continue
		}
		rec := ChangedManagedFileRecord{
			Name:   path,
			Status: "changed",
		}
		records = append(records, rec)
	}

	scope.Elements = records
	return scope
}

// collectKernelConfig collects active kernel configuration.
func collectKernelConfig(fs Filesystem, cr CommandRunner) *ScopeWrapper[KernelConfigRecord] {
	scope := &ScopeWrapper[KernelConfigRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []KernelConfigRecord{},
	}

	var content string

	// Try /proc/config.gz first
	if fs.Exists("/proc/config.gz") {
		stdout, _, err := cr.Run("gzip", []string{"-dc", "/proc/config.gz"})
		if err == nil {
			content = stdout
		}
	}

	// Fallback to /boot/config-$(uname -r)
	if content == "" {
		unameOut, _, _ := cr.Run("uname", []string{"-r"})
		kernelRelease := strings.TrimSpace(unameOut)
		if kernelRelease != "" {
			bootConfig := "/boot/config-" + kernelRelease
			if c, err := fs.ReadFile(bootConfig); err == nil {
				content = c
			}
		}
	}

	if content == "" {
		fmt.Fprintf(os.Stderr, "sitar: collect-kernel-config: no kernel config source found\n")
		return nil
	}

	var records []KernelConfigRecord
	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		idx := strings.Index(line, "=")
		if idx < 0 {
			continue
		}
		key := line[:idx]
		val := line[idx+1:]
		records = append(records, KernelConfigRecord{Key: key, Value: val})
	}

	sort.Slice(records, func(i, j int) bool {
		return records[i].Key < records[j].Key
	})
	scope.Elements = records
	return scope
}
