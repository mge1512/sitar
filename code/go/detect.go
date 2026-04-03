package main

import (
	"fmt"
	"os"
	"strings"
)

// readFirstLine reads the file at path via fs and returns the first non-empty
// line, trimmed of leading and trailing whitespace.  An empty string is
// returned when the file cannot be read or contains no non-empty lines.
func readFirstLine(fs Filesystem, path string) string {
	content, err := fs.ReadFile(path)
	if err != nil {
		return ""
	}
	for _, line := range strings.Split(content, "\n") {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" {
			return trimmed
		}
	}
	return ""
}

// parseOsRelease parses a KEY=VALUE formatted file (as used by /etc/os-release)
// and returns a map of keys to values.  Surrounding double- or single-quote
// characters are stripped from each value.
func parseOsRelease(content string) map[string]string {
	result := make(map[string]string)
	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		// Skip blank lines and comments.
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		idx := strings.IndexByte(line, '=')
		if idx < 0 {
			continue
		}
		key := strings.TrimSpace(line[:idx])
		val := strings.TrimSpace(line[idx+1:])
		// Strip a single layer of matching outer quotes (double or single).
		if len(val) >= 2 {
			if (val[0] == '"' && val[len(val)-1] == '"') ||
				(val[0] == '\'' && val[len(val)-1] == '\'') {
				val = val[1 : len(val)-1]
			}
		}
		result[key] = val
	}
	return result
}

// detectDistribution probes well-known release files via fs and returns a
// DistroInfo describing the detected Linux distribution family, release
// string, and package-management backend.
//
// Detection order (first match wins):
//  1. /etc/debian_version      → deb / dpkg
//  2. /etc/redhat-release      → rpm / /bin/rpm
//  3. /etc/UnitedLinux-release AND /etc/SuSE-release → rpm / /bin/rpm (combined release)
//  4. /etc/UnitedLinux-release → rpm / /bin/rpm
//  5. /etc/SLOX-release        → rpm / /bin/rpm
//  6. /etc/SuSE-release        → rpm / /bin/rpm
//  7. /etc/os-release          → rpm / /usr/bin/rpm  (PRETTY_NAME value)
//  8. unknown / none           → warning emitted to stderr
func detectDistribution(fs Filesystem) DistroInfo {
	// ── Step 1: Debian / Ubuntu ───────────────────────────────────────────────
	if fs.Exists("/etc/debian_version") {
		return DistroInfo{
			Family:     FamilyDeb,
			Release:    readFirstLine(fs, "/etc/debian_version"),
			Backend:    BackendDpkg,
			DpkgStatus: "/var/lib/dpkg/status",
		}
	}

	// ── Step 2: Red Hat / CentOS / Fedora / RHEL ─────────────────────────────
	if fs.Exists("/etc/redhat-release") {
		return DistroInfo{
			Family:  FamilyRPM,
			Release: readFirstLine(fs, "/etc/redhat-release"),
			Backend: BackendRPM,
			RpmCmd:  "/bin/rpm",
		}
	}

	// ── Step 3: UnitedLinux + SuSE (both present) ────────────────────────────
	if fs.Exists("/etc/UnitedLinux-release") && fs.Exists("/etc/SuSE-release") {
		ulLine := readFirstLine(fs, "/etc/UnitedLinux-release")
		suseLine := readFirstLine(fs, "/etc/SuSE-release")
		return DistroInfo{
			Family:  FamilyRPM,
			Release: ulLine + ", " + suseLine,
			Backend: BackendRPM,
			RpmCmd:  "/bin/rpm",
		}
	}

	// ── Step 4: UnitedLinux only ──────────────────────────────────────────────
	if fs.Exists("/etc/UnitedLinux-release") {
		return DistroInfo{
			Family:  FamilyRPM,
			Release: readFirstLine(fs, "/etc/UnitedLinux-release"),
			Backend: BackendRPM,
			RpmCmd:  "/bin/rpm",
		}
	}

	// ── Step 5: SLOX ──────────────────────────────────────────────────────────
	if fs.Exists("/etc/SLOX-release") {
		return DistroInfo{
			Family:  FamilyRPM,
			Release: readFirstLine(fs, "/etc/SLOX-release"),
			Backend: BackendRPM,
			RpmCmd:  "/bin/rpm",
		}
	}

	// ── Step 6: SuSE only ────────────────────────────────────────────────────
	if fs.Exists("/etc/SuSE-release") {
		return DistroInfo{
			Family:  FamilyRPM,
			Release: readFirstLine(fs, "/etc/SuSE-release"),
			Backend: BackendRPM,
			RpmCmd:  "/bin/rpm",
		}
	}

	// ── Step 7: systemd /etc/os-release (modern fallback) ────────────────────
	if fs.Exists("/etc/os-release") {
		content, err := fs.ReadFile("/etc/os-release")
		release := ""
		if err == nil {
			fields := parseOsRelease(content)
			release = fields["PRETTY_NAME"]
		}
		return DistroInfo{
			Family:  FamilyRPM,
			Release: release,
			Backend: BackendRPM,
			RpmCmd:  "/usr/bin/rpm",
		}
	}

	// ── Step 8: Unknown ───────────────────────────────────────────────────────
	fmt.Fprintln(os.Stderr, "distribution not supported")
	return DistroInfo{
		Family:  FamilyUnknown,
		Backend: BackendNone,
	}
}
