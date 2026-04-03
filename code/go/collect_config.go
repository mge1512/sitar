package main

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
)

// collectSecurityApparmor collects AppArmor kernel state and profile list.
func collectSecurityApparmor(fs Filesystem, cr CommandRunner, config *Config) *SecurityApparmorScope {
	// Locate AppArmor kernel interface
	apparmorPaths := []string{
		"/proc/sys/kernel/security/apparmor",
		"/sys/kernel/security/apparmor",
	}
	var kernelPath string
	for _, p := range apparmorPaths {
		if fs.Exists(p) {
			kernelPath = p
			break
		}
	}
	if kernelPath == "" {
		return nil
	}

	scope := &SecurityApparmorScope{
		KernelParams: &ScopeWrapper[ApparmorKernelRecord]{
			Attributes: map[string]interface{}{"apparmor_path": kernelPath},
			Elements:   []ApparmorKernelRecord{},
		},
		Profiles: &ScopeWrapper[ApparmorProfileRecord]{
			Attributes: map[string]interface{}{},
			Elements:   []ApparmorProfileRecord{},
		},
		ConfigFiles: []string{},
	}

	// Walk kernel path and collect files
	err := fs.WalkDir(kernelPath, func(path string, isDir bool) error {
		if isDir {
			return nil
		}
		content, err := fs.ReadFileLimited(path, 32767)
		if err != nil {
			return nil
		}
		content = strings.TrimSpace(content)
		if content == "" {
			return nil
		}

		rel, _ := filepath.Rel(kernelPath, path)

		if rel == "profiles" {
			// Parse profiles: name (mode)\n
			profileRe := regexp.MustCompile(`^(.+) \((enforce|complain|unconfined)\)$`)
			for _, line := range strings.Split(content, "\n") {
				line = strings.TrimSpace(line)
				m := profileRe.FindStringSubmatch(line)
				if m != nil {
					scope.Profiles.Elements = append(scope.Profiles.Elements, ApparmorProfileRecord{
						Name: m[1],
						Mode: m[2],
					})
				}
			}
		} else {
			scope.KernelParams.Elements = append(scope.KernelParams.Elements, ApparmorKernelRecord{
				Key:   rel,
				Value: content,
			})
		}
		return nil
	})
	if err != nil {
		fmt.Fprintf(os.Stderr, "sitar: collect-security-apparmor: %v\n", err)
	}

	// Collect config files
	profileDirs := []string{
		"/etc/apparmor", "/etc/apparmor.d",
		"/etc/subdomain", "/etc/subdomain.d",
	}
	for _, dir := range profileDirs {
		if fs.Exists(dir) {
			scope.ConfigFiles = append(scope.ConfigFiles, dir)
			if config.AllSubdomain == "On" ||
				(config.AllSubdomain == "Auto") {
				files, _ := fs.Glob(filepath.Join(dir, "*"))
				scope.ConfigFiles = append(scope.ConfigFiles, files...)
			}
		}
	}

	return scope
}

// collectConfigFiles collects configuration file contents.
// Results are used in human-readable output; not a distinct JSON scope.
func collectConfigFiles(fs Filesystem, config *Config, consistencyCacheExists, unpackedCacheExists bool) []string {
	var collected []string

	// Determine effective source mode for allconfigfiles
	useHardcoded := false
	switch config.AllConfigFiles {
	case "On":
		useHardcoded = true
	case "Off":
		useHardcoded = false
	case "Auto":
		useHardcoded = !consistencyCacheExists && !unpackedCacheExists
	}

	if useHardcoded {
		candidates := getConfigFileCandidates()
		for _, pattern := range candidates {
			paths, _ := fs.Glob(pattern)
			if len(paths) == 0 && !strings.ContainsAny(pattern, "*?[") {
				paths = []string{pattern}
			}
			for _, p := range paths {
				if shouldSkipConfigFile(p, config) {
					continue
				}
				if fs.Exists(p) {
					collected = append(collected, p)
				}
			}
		}
	}

	// Always include certain files
	alwaysInclude := []string{
		"/etc/ssh/sshd_config", "/etc/sshd_config",
		"/etc/named.conf", "/etc/bind/named.conf",
		"/etc/samba/smb.conf", "/etc/smb.conf",
		"/etc/openldap/slapd.conf", "/etc/ldap/slapd.conf", "/etc/slapd.conf",
		"/etc/openldap/ldap.conf", "/etc/ldap/ldap.conf", "/etc/ldap.conf",
		"/etc/aliases",
	}
	for _, p := range alwaysInclude {
		if fs.Exists(p) {
			collected = append(collected, p)
		}
	}

	// Process .include files from /var/lib/support/
	includeFiles, _ := fs.Glob("/var/lib/support/*.include")
	for _, f := range includeFiles {
		content, err := fs.ReadFile(f)
		if err != nil {
			continue
		}
		re := regexp.MustCompile(`"([^"]+)"`)
		matches := re.FindAllStringSubmatch(content, -1)
		for _, m := range matches {
			p := m[1]
			if !shouldSkipConfigFile(p, config) && fs.Exists(p) {
				collected = append(collected, p)
			}
		}
	}

	// Deduplicate
	seen := map[string]bool{}
	var result []string
	for _, p := range collected {
		if !seen[p] {
			seen[p] = true
			result = append(result, p)
		}
	}
	sort.Strings(result)
	return result
}

func shouldSkipConfigFile(path string, config *Config) bool {
	// Skip /proc paths
	if strings.HasPrefix(path, "/proc") {
		return true
	}
	// Skip excluded paths
	for _, ex := range config.Exclude {
		if path == ex {
			return true
		}
	}
	// Skip backup files
	base := filepath.Base(path)
	backupPatterns := []string{"*.orig", "*.org", "*.ori", "*.bak", "*.bac", "*~"}
	for _, pat := range backupPatterns {
		if matched, _ := filepath.Match(pat, base); matched {
			return true
		}
	}
	if strings.HasPrefix(base, "#") {
		return true
	}
	// Skip gconf if disabled
	if !config.GConf && strings.HasPrefix(path, "/etc/opt/gnome") {
		return true
	}
	// Skip lvm archive if disabled
	if !config.LvmArchive && strings.HasPrefix(path, "/etc/lvm/archive") {
		return true
	}
	return false
}

func getConfigFileCandidates() []string {
	return []string{
		// Authentication and identity
		"/etc/passwd", "/etc/group",
		"/etc/nsswitch.conf", "/etc/pam.d/*", "/etc/security/*",
		"/etc/krb5.conf",
		// Network
		"/etc/hosts", "/etc/hostname", "/etc/resolv.conf",
		"/etc/networks", "/etc/protocols", "/etc/services",
		"/etc/hosts.allow", "/etc/hosts.deny",
		"/etc/ntp.conf", "/etc/chrony.conf", "/etc/chrony.d/*",
		"/etc/network/interfaces", "/etc/network/interfaces.d/*",
		"/etc/netconfig", "/etc/gai.conf",
		// SSH
		"/etc/ssh/sshd_config", "/etc/ssh/ssh_config", "/etc/sshd_config",
		// DNS
		"/etc/named.conf", "/etc/bind/named.conf",
		// Mail
		"/etc/postfix/main.cf", "/etc/postfix/master.cf", "/etc/aliases",
		"/etc/sendmail.cf", "/etc/mail/sendmail.cf",
		// Web / proxy
		"/etc/apache2/httpd.conf", "/etc/apache2/apache2.conf",
		"/etc/httpd/conf/httpd.conf", "/etc/nginx/nginx.conf",
		"/etc/squid/squid.conf", "/etc/squid.conf",
		// File sharing
		"/etc/samba/smb.conf", "/etc/smb.conf",
		"/etc/exports", "/etc/exports.d/*",
		// Directory services
		"/etc/openldap/slapd.conf", "/etc/ldap/slapd.conf", "/etc/slapd.conf",
		"/etc/openldap/ldap.conf", "/etc/ldap/ldap.conf", "/etc/ldap.conf",
		// Logging
		"/etc/syslog.conf", "/etc/syslog-ng/syslog-ng.conf",
		"/etc/rsyslog.conf", "/etc/rsyslog.d/*",
		// Init and boot
		"/etc/inittab", "/etc/grub.conf", "/etc/lilo.conf",
		"/boot/grub/menu.lst", "/boot/grub2/grub.cfg",
		"/etc/default/grub", "/etc/grub2.cfg",
		// Cron
		"/etc/crontab", "/etc/cron.d/*", "/etc/cron.daily/*",
		"/etc/cron.weekly/*", "/etc/cron.monthly/*",
		// Printing
		"/etc/cups/cupsd.conf", "/etc/printcap",
		// Firewall
		"/etc/sysconfig/SuSEfirewall2", "/etc/sysconfig/iptables",
		// Misc system
		"/etc/fstab", "/etc/mtab", "/etc/mdadm.conf", "/etc/mdadm/mdadm.conf",
		"/etc/modules", "/etc/modprobe.d/*", "/etc/modprobe.conf",
		"/etc/sysctl.conf", "/etc/sysctl.d/*",
		"/etc/securetty", "/etc/shells", "/etc/environment",
		"/etc/profile", "/etc/profile.d/*.sh", "/etc/bashrc", "/etc/bash.bashrc",
		"/etc/login.defs", "/etc/logrotate.conf", "/etc/logrotate.d/*",
		"/etc/updatedb.conf", "/etc/ld.so.conf", "/etc/ld.so.conf.d/*",
	}
}

// findUnpacked finds files below /etc that do not belong to any installed RPM.
func findUnpacked(cr CommandRunner) (string, error) {
	configDir := "/var/lib/support"
	cacheFile := filepath.Join(configDir, "Find_Unpacked.include")

	// Get all files owned by RPMs under /etc
	stdout, _, err := cr.Run("rpm", []string{"-qla"})
	if err != nil {
		return "", fmt.Errorf("rpm -qla: %w", err)
	}

	rpmFiles := map[string]bool{}
	for _, line := range strings.Split(stdout, "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "/etc/") {
			rpmFiles[line] = true
		}
	}

	// Walk /etc and find files not owned by RPMs
	var unpackedFiles []string
	err = filepath.WalkDir("/etc", func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return nil
		}
		if d.IsDir() {
			return nil
		}
		if strings.HasSuffix(path, "~") {
			return nil
		}
		if !rpmFiles[path] {
			unpackedFiles = append(unpackedFiles, path)
		}
		return nil
	})
	if err != nil {
		return "", fmt.Errorf("walk /etc: %w", err)
	}

	sort.Strings(unpackedFiles)

	// Write Perl array format
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return "", fmt.Errorf("mkdir %s: %w", configDir, err)
	}

	var sb strings.Builder
	sb.WriteString("@files = (\n")
	for _, f := range unpackedFiles {
		sb.WriteString(fmt.Sprintf("  %q,\n", f))
	}
	sb.WriteString(");\n")

	if err := os.WriteFile(cacheFile, []byte(sb.String()), 0644); err != nil {
		return "", fmt.Errorf("write %s: %w", cacheFile, err)
	}

	return cacheFile, nil
}

// checkConsistency checks that RPM config files have not been modified since installation.
func checkConsistency(cr CommandRunner) (string, error) {
	configDir := "/var/lib/support"
	cacheFile := filepath.Join(configDir, "Configuration_Consistency.include")

	// Get all config files with package names
	stdout, _, err := cr.Run("rpm", []string{"-qca", "--queryformat", "%{NAME}\n"})
	if err != nil {
		return "", fmt.Errorf("rpm -qca: %w", err)
	}

	pkgSet := map[string]bool{}
	for _, line := range strings.Split(stdout, "\n") {
		line = strings.TrimSpace(line)
		if line != "" {
			pkgSet[line] = true
		}
	}

	var changedFiles []string
	for pkg := range pkgSet {
		verOut, _, _ := cr.Run("rpm", []string{"-V", "--nodeps", "--noscript", pkg})
		for _, line := range strings.Split(verOut, "\n") {
			if len(line) < 12 {
				continue
			}
			fileType := strings.TrimSpace(string(line[9]))
			if fileType != "c" {
				continue
			}
			path := strings.TrimSpace(line[11:])
			if path != "" && !strings.HasPrefix(path, "missing") {
				changedFiles = append(changedFiles, path)
			}
		}
	}

	sort.Strings(changedFiles)

	if err := os.MkdirAll(configDir, 0755); err != nil {
		return "", fmt.Errorf("mkdir %s: %w", configDir, err)
	}

	var sb strings.Builder
	sb.WriteString("@files = (\n")
	for _, f := range changedFiles {
		sb.WriteString(fmt.Sprintf("  %q,\n", f))
	}
	sb.WriteString(");\n")

	if err := os.WriteFile(cacheFile, []byte(sb.String()), 0644); err != nil {
		return "", fmt.Errorf("write %s: %w", cacheFile, err)
	}

	return cacheFile, nil
}
