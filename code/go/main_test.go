package main

import (
	"strings"
	"testing"
)

// TestPrepareConfigHelp verifies that empty argv produces help text and exits 0.
// (We can't test os.Exit directly, but we can test the config parsing logic.)

// TestPrepareConfigVersion verifies version command recognition.
func TestDetectDistributionDebian(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/etc/debian_version": "12.0\n",
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}

	info := detectDistribution(fs)
	if info.Family != FamilyDeb {
		t.Errorf("expected FamilyDeb, got %s", info.Family)
	}
	if info.Backend != BackendDpkg {
		t.Errorf("expected BackendDpkg, got %s", info.Backend)
	}
	if info.DpkgStatus != "/var/lib/dpkg/status" {
		t.Errorf("expected /var/lib/dpkg/status, got %s", info.DpkgStatus)
	}
}

func TestDetectDistributionRPM(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/etc/redhat-release": "Red Hat Enterprise Linux 9\n",
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}

	info := detectDistribution(fs)
	if info.Family != FamilyRPM {
		t.Errorf("expected FamilyRPM, got %s", info.Family)
	}
	if info.Backend != BackendRPM {
		t.Errorf("expected BackendRPM, got %s", info.Backend)
	}
	if info.Release != "Red Hat Enterprise Linux 9" {
		t.Errorf("unexpected release: %s", info.Release)
	}
}

func TestDetectDistributionUnknown(t *testing.T) {
	fs := &FakeFilesystem{
		Files:       map[string]string{},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}

	info := detectDistribution(fs)
	if info.Family != FamilyUnknown {
		t.Errorf("expected FamilyUnknown, got %s", info.Family)
	}
}

func TestCollectEnvironmentDocker(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/etc/locale.conf": "LANG=en_US.UTF-8\n",
		},
		Dirs:        map[string]bool{"/.dockerenv": true},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}
	// Make /.dockerenv exist as a file too
	fs.Files["/.dockerenv"] = ""

	env := collectEnvironment(fs)
	if env.Locale != "en_US.UTF-8" {
		t.Errorf("expected en_US.UTF-8, got %s", env.Locale)
	}
	if env.SystemType != "docker" {
		t.Errorf("expected docker, got %s", env.SystemType)
	}
}

func TestCollectEnvironmentLocal(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/etc/locale.conf": "LANG=de_DE.UTF-8\n",
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}

	env := collectEnvironment(fs)
	if env.Locale != "de_DE.UTF-8" {
		t.Errorf("expected de_DE.UTF-8, got %s", env.Locale)
	}
	if env.SystemType != "local" {
		t.Errorf("expected local, got %s", env.SystemType)
	}
}

func TestCollectGeneralInfoRecordCount(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/proc/meminfo": "MemTotal:       16384000 kB\n",
			"/proc/cmdline": "BOOT_IMAGE=/vmlinuz\n",
			"/proc/loadavg": "0.10 0.20 0.30 1/100 1234\n",
			"/proc/uptime":  "3600.00 7200.00\n",
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}
	cr := &FakeCommandRunner{
		Responses: map[string]CommandResponse{
			"hostname -f": {Stdout: "myhost.example.com\n"},
			"uname -a":    {Stdout: "Linux myhost 5.15.0 #1 SMP x86_64 GNU/Linux\n"},
		},
	}
	distro := DistroInfo{Release: "Test Linux 1.0"}

	scope := collectGeneralInfo(fs, cr, distro)
	if scope == nil {
		t.Fatal("expected non-nil scope")
	}
	if len(scope.Elements) != 9 {
		t.Errorf("expected 9 records, got %d", len(scope.Elements))
	}
	if scope.Elements[0].Key != "hostname" {
		t.Errorf("first key should be hostname, got %s", scope.Elements[0].Key)
	}
	if scope.Elements[0].Value != "myhost.example.com" {
		t.Errorf("hostname value should be myhost.example.com, got %s", scope.Elements[0].Value)
	}
}

func TestCollectCPUParsing(t *testing.T) {
	cpuInfo := `processor	: 0
vendor_id	: GenuineIntel
cpu family	: 6
model		: 142
model name	: Intel(R) Core(TM) i7-8550U CPU @ 1.80GHz
stepping	: 10
cpu MHz		: 1992.000
cache size	: 8192 KB

processor	: 1
vendor_id	: GenuineIntel
cpu family	: 6
model		: 142
model name	: Intel(R) Core(TM) i7-8550U CPU @ 1.80GHz
stepping	: 10
cpu MHz		: 1992.000
cache size	: 8192 KB

`
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/proc/cpuinfo": cpuInfo,
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}
	cr := &FakeCommandRunner{
		Responses: map[string]CommandResponse{
			"uname -m": {Stdout: "x86_64\n"},
		},
	}

	scope := collectCPU(fs, cr)
	if scope == nil {
		t.Fatal("expected non-nil scope")
	}
	if len(scope.Elements) != 2 {
		t.Errorf("expected 2 CPU records, got %d", len(scope.Elements))
	}
	if scope.Elements[0].VendorID != "GenuineIntel" {
		t.Errorf("expected GenuineIntel, got %s", scope.Elements[0].VendorID)
	}
	if scope.Elements[0].ModelName != "Intel(R) Core(TM) i7-8550U CPU @ 1.80GHz" {
		t.Errorf("unexpected model name: %s", scope.Elements[0].ModelName)
	}
}

func TestCollectNetworkFirewallNone(t *testing.T) {
	fs := &FakeFilesystem{
		Files:       map[string]string{},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}
	cr := &FakeCommandRunner{Responses: map[string]CommandResponse{}}

	scope := collectNetworkFirewall(fs, cr)
	if scope == nil {
		t.Fatal("expected non-nil scope")
	}
	if len(scope.Elements) != 1 {
		t.Errorf("expected 1 element, got %d", len(scope.Elements))
	}
	if scope.Elements[0].Engine != "none" {
		t.Errorf("expected engine=none, got %s", scope.Elements[0].Engine)
	}
}

func TestFakeCommandRunner(t *testing.T) {
	cr := &FakeCommandRunner{
		Responses: map[string]CommandResponse{
			"hostname -f": {Stdout: "test.host\n"},
		},
	}
	stdout, _, _ := cr.Run("hostname", []string{"-f"})
	if strings.TrimSpace(stdout) != "test.host" {
		t.Errorf("expected test.host, got %q", stdout)
	}
}

func TestFakeFilesystem(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/etc/test.conf": "key=value\n",
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{"/usr/bin/test": true},
		StatInfo:    map[string]FileInfo{},
	}

	content, err := fs.ReadFile("/etc/test.conf")
	if err != nil {
		t.Errorf("unexpected error: %v", err)
	}
	if content != "key=value\n" {
		t.Errorf("unexpected content: %q", content)
	}

	if !fs.Exists("/etc/test.conf") {
		t.Error("expected /etc/test.conf to exist")
	}
	if fs.Exists("/etc/nonexistent") {
		t.Error("expected /etc/nonexistent to not exist")
	}
	if !fs.IsExecutable("/usr/bin/test") {
		t.Error("expected /usr/bin/test to be executable")
	}
}

func TestHTMLRendererHeader(t *testing.T) {
	r := &HTMLRenderer{}
	manifest := &SitarManifest{}
	manifest.Meta.Hostname = "testhost"
	header := r.Header(manifest)
	if !strings.Contains(header, "<!DOCTYPE html>") {
		t.Error("HTML header should contain DOCTYPE")
	}
	if !strings.Contains(header, "<body>") {
		t.Error("HTML header should contain <body>")
	}
}

func TestTeXRendererHeader(t *testing.T) {
	r := &TeXRenderer{}
	manifest := &SitarManifest{}
	header := r.Header(manifest)
	if !strings.Contains(header, `\documentclass`) {
		t.Error("TeX header should contain \\documentclass")
	}
	if !strings.Contains(header, `\begin{document}`) {
		t.Error("TeX header should contain \\begin{document}")
	}
}

func TestMarkdownRendererHeader(t *testing.T) {
	r := &MarkdownRenderer{}
	manifest := &SitarManifest{}
	header := r.Header(manifest)
	if !strings.Contains(header, "# SITAR") {
		t.Error("Markdown header should contain # SITAR")
	}
}

func TestDocBookRendererHeader(t *testing.T) {
	r := &DocBookRenderer{}
	manifest := &SitarManifest{}
	header := r.Header(manifest)
	if !strings.Contains(header, "<?xml") {
		t.Error("DocBook header should contain <?xml")
	}
	if !strings.Contains(header, "<article") {
		t.Error("DocBook header should contain <article")
	}
}

func TestHTMLEscape(t *testing.T) {
	r := &HTMLRenderer{}
	result := r.Escape("<script>alert('xss')</script>")
	if strings.Contains(result, "<script>") {
		t.Error("HTML escape should escape <")
	}
	if !strings.Contains(result, "&lt;") {
		t.Error("HTML escape should produce &lt;")
	}
}

func TestRenderTableMarkdown(t *testing.T) {
	r := &MarkdownRenderer{}
	headers := []string{"key", "value"}
	rows := [][]string{{"hostname", "myhost"}, {"os", "Linux"}}
	result := renderTableMarkdown(r, headers, rows)
	if !strings.Contains(result, "| key | value |") {
		t.Errorf("expected header row, got: %s", result)
	}
	if !strings.Contains(result, "| hostname | myhost |") {
		t.Errorf("expected data row, got: %s", result)
	}
}

func TestGroupParsing(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/etc/group": "root:x:0:\ndaemon:x:1:root,daemon\n",
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}

	scope := collectGroups(fs)
	if scope == nil {
		t.Fatal("expected non-nil scope")
	}
	if len(scope.Elements) != 2 {
		t.Errorf("expected 2 groups, got %d", len(scope.Elements))
	}
	if scope.Elements[0].Name != "daemon" {
		t.Errorf("expected daemon (sorted), got %s", scope.Elements[0].Name)
	}
}
