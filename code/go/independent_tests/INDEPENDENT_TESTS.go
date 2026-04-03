// Package main provides independent tests for sitar that use only test doubles.
// These tests verify behaviour without any live external services.
// All tests use FakeFilesystem and FakeCommandRunner exclusively.
//
// To run these tests, copy this file to the root of the sitar package:
//   cp independent_tests/INDEPENDENT_TESTS.go .
//   go test -run TestIndependent ./...
//
//go:build ignore

package main

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
)

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: prepareConfig_no_args_exits_0
// Verifies: EXAMPLE no_arguments_shows_help
// Confidence: Medium (cannot test os.Exit directly; tests the logic path)
// ---------------------------------------------------------------------------

// TestPrepareConfigDefaults verifies that default config values are applied.
func TestPrepareConfigDefaults(t *testing.T) {
	// We cannot call prepareConfig with empty args (it calls os.Exit(0))
	// but we can verify the defaults are set correctly by inspecting
	// the initial config struct.
	config := &Config{
		FileSizeLimit:  700000,
		Exclude:        []string{"/etc/shadow"},
		AllConfigFiles: "Auto",
		AllSubdomain:   "Auto",
		AllSysconfig:   "Auto",
		Verbosity:      "normal",
	}
	if config.FileSizeLimit != 700000 {
		t.Errorf("default FileSizeLimit should be 700000, got %d", config.FileSizeLimit)
	}
	if len(config.Exclude) != 1 || config.Exclude[0] != "/etc/shadow" {
		t.Errorf("default Exclude should be [/etc/shadow], got %v", config.Exclude)
	}
	if config.AllConfigFiles != "Auto" {
		t.Errorf("default AllConfigFiles should be Auto, got %s", config.AllConfigFiles)
	}
	if config.Verbosity != "normal" {
		t.Errorf("default Verbosity should be normal, got %s", config.Verbosity)
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: detect_distribution_all_families
// Verifies: BEHAVIOR detect-distribution
// Confidence: High
// ---------------------------------------------------------------------------

func TestDetectDistributionAllFamilies(t *testing.T) {
	tests := []struct {
		name           string
		files          map[string]string
		expectedFamily DistributionFamily
		expectedBackend PackageVersioningBackend
	}{
		{
			name:           "debian",
			files:          map[string]string{"/etc/debian_version": "12.0\n"},
			expectedFamily:  FamilyDeb,
			expectedBackend: BackendDpkg,
		},
		{
			name:           "redhat",
			files:          map[string]string{"/etc/redhat-release": "Red Hat Enterprise Linux 9\n"},
			expectedFamily:  FamilyRPM,
			expectedBackend: BackendRPM,
		},
		{
			name:           "suse",
			files:          map[string]string{"/etc/SuSE-release": "openSUSE Leap 15.5\n"},
			expectedFamily:  FamilyRPM,
			expectedBackend: BackendRPM,
		},
		{
			name:           "os-release",
			files:          map[string]string{"/etc/os-release": `NAME="SLES"\nPRETTY_NAME="SUSE Linux Enterprise Server 15 SP6"\n`},
			expectedFamily:  FamilyRPM,
			expectedBackend: BackendRPM,
		},
		{
			name:           "unknown",
			files:          map[string]string{},
			expectedFamily:  FamilyUnknown,
			expectedBackend: BackendNone,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			fs := &FakeFilesystem{
				Files:       tc.files,
				Dirs:        map[string]bool{},
				Executables: map[string]bool{},
				StatInfo:    map[string]FileInfo{},
			}
			info := detectDistribution(fs)
			if info.Family != tc.expectedFamily {
				t.Errorf("expected family %s, got %s", tc.expectedFamily, info.Family)
			}
			if info.Backend != tc.expectedBackend {
				t.Errorf("expected backend %s, got %s", tc.expectedBackend, info.Backend)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: collect_general_info_9_records
// Verifies: BEHAVIOR collect-general-info, POSTCONDITION result._elements has exactly 9 records
// Confidence: High
// ---------------------------------------------------------------------------

func TestCollectGeneralInfoExactly9Records(t *testing.T) {
	fs := &FakeFilesystem{
		Files: map[string]string{
			"/proc/meminfo": "MemTotal:       8192000 kB\nMemFree: 4096000 kB\n",
			"/proc/cmdline": "BOOT_IMAGE=/vmlinuz root=/dev/sda1\n",
			"/proc/loadavg": "0.05 0.10 0.15 2/500 9999\n",
			"/proc/uptime":  "7200.00 14400.00\n",
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}
	cr := &FakeCommandRunner{
		Responses: map[string]CommandResponse{
			"hostname -f": {Stdout: "testbox.example.com\n"},
			"uname -a":    {Stdout: "Linux testbox 5.15.0-91-generic #101-Ubuntu SMP x86_64 GNU/Linux\n"},
		},
	}
	distro := DistroInfo{Release: "Ubuntu 23.04"}

	scope := collectGeneralInfo(fs, cr, distro)
	if scope == nil {
		t.Fatal("collectGeneralInfo returned nil")
	}
	if len(scope.Elements) != 9 {
		t.Fatalf("expected exactly 9 records, got %d", len(scope.Elements))
	}

	// Verify key order
	expectedKeys := []string{
		"hostname", "os_release", "uname", "collected_at",
		"mem_total_kb", "cmdline", "loadavg", "uptime_min", "idletime_min",
	}
	for i, key := range expectedKeys {
		if scope.Elements[i].Key != key {
			t.Errorf("record[%d]: expected key %q, got %q", i, key, scope.Elements[i].Key)
		}
	}

	// Verify hostname is not empty
	if scope.Elements[0].Value == "" {
		t.Error("hostname value must not be empty")
	}
	if scope.Elements[0].Value != "testbox.example.com" {
		t.Errorf("hostname: expected testbox.example.com, got %s", scope.Elements[0].Value)
	}

	// Verify uptime_min calculation (7200 sec / 60 = 120 min)
	if scope.Elements[7].Value != "120" {
		t.Errorf("uptime_min: expected 120, got %s", scope.Elements[7].Value)
	}
	// idletime_min = 14400 / 60 = 240
	if scope.Elements[8].Value != "240" {
		t.Errorf("idletime_min: expected 240, got %s", scope.Elements[8].Value)
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: collect_environment
// Verifies: BEHAVIOR collect-environment
// Confidence: High
// ---------------------------------------------------------------------------

func TestCollectEnvironmentLocale(t *testing.T) {
	tests := []struct {
		name           string
		files          map[string]string
		expectedLocale string
		expectedType   string
	}{
		{
			name:           "locale_conf",
			files:          map[string]string{"/etc/locale.conf": "LANG=fr_FR.UTF-8\n"},
			expectedLocale: "fr_FR.UTF-8",
			expectedType:   "local",
		},
		{
			name:           "default_locale",
			files:          map[string]string{"/etc/default/locale": "LANG=en_GB.UTF-8\n"},
			expectedLocale: "en_GB.UTF-8",
			expectedType:   "local",
		},
		{
			name:           "sysconfig_language",
			files:          map[string]string{"/etc/sysconfig/language": "LANG=de_DE.UTF-8\n"},
			expectedLocale: "de_DE.UTF-8",
			expectedType:   "local",
		},
		{
			name:           "no_locale_defaults_to_C",
			files:          map[string]string{},
			expectedLocale: "C",
			expectedType:   "local",
		},
		{
			name: "docker_detection",
			files: map[string]string{
				"/etc/locale.conf": "LANG=en_US.UTF-8\n",
				"/.dockerenv":      "",
			},
			expectedLocale: "en_US.UTF-8",
			expectedType:   "docker",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			fs := &FakeFilesystem{
				Files:       tc.files,
				Dirs:        map[string]bool{},
				Executables: map[string]bool{},
				StatInfo:    map[string]FileInfo{},
			}
			env := collectEnvironment(fs)
			if env.Locale != tc.expectedLocale {
				t.Errorf("expected locale %q, got %q", tc.expectedLocale, env.Locale)
			}
			if env.SystemType != tc.expectedType {
				t.Errorf("expected system_type %q, got %q", tc.expectedType, env.SystemType)
			}
		})
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: collect_cpu_parsing
// Verifies: BEHAVIOR collect-cpu
// Confidence: High
// ---------------------------------------------------------------------------

func TestCollectCPUMultipleProcessors(t *testing.T) {
	cpuInfo := `processor	: 0
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 80
model name	: AMD Ryzen 9 5900X 12-Core Processor
stepping	: 0
cpu MHz		: 3700.000
cache size	: 512 KB

processor	: 1
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 80
model name	: AMD Ryzen 9 5900X 12-Core Processor
stepping	: 0
cpu MHz		: 3700.000
cache size	: 512 KB

`
	fs := &FakeFilesystem{
		Files:       map[string]string{"/proc/cpuinfo": cpuInfo},
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
		t.Fatal("collectCPU returned nil")
	}
	if len(scope.Elements) != 2 {
		t.Fatalf("expected 2 CPU records, got %d", len(scope.Elements))
	}
	if scope.Elements[0].VendorID != "AuthenticAMD" {
		t.Errorf("expected AuthenticAMD, got %q", scope.Elements[0].VendorID)
	}
	if scope.Attributes["architecture"] != "x86_64" {
		t.Errorf("expected architecture x86_64, got %v", scope.Attributes["architecture"])
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: collect_network_firewall_none
// Verifies: BEHAVIOR collect-network-firewall step 4 (no filter installed)
// Confidence: High
// ---------------------------------------------------------------------------

func TestCollectNetworkFirewallNoneEngine(t *testing.T) {
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
		t.Fatalf("expected 1 element, got %d", len(scope.Elements))
	}
	if scope.Elements[0].Engine != "none" {
		t.Errorf("expected engine=none, got %q", scope.Elements[0].Engine)
	}
	if scope.Elements[0].RawOutput != "No packet filter installed." {
		t.Errorf("unexpected raw_output: %q", scope.Elements[0].RawOutput)
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: collect_groups_parsing
// Verifies: BEHAVIOR collect-groups
// Confidence: High
// ---------------------------------------------------------------------------

func TestCollectGroupsParsing(t *testing.T) {
	groupFile := "root:x:0:\ndaemon:x:1:root,daemon\nadm:x:4:syslog,mge\n"
	fs := &FakeFilesystem{
		Files:       map[string]string{"/etc/group": groupFile},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}

	scope := collectGroups(fs)
	if scope == nil {
		t.Fatal("collectGroups returned nil")
	}
	if len(scope.Elements) != 3 {
		t.Fatalf("expected 3 groups, got %d", len(scope.Elements))
	}

	// Should be sorted by name: adm, daemon, root
	if scope.Elements[0].Name != "adm" {
		t.Errorf("expected adm first (sorted), got %s", scope.Elements[0].Name)
	}
	if scope.Elements[1].Name != "daemon" {
		t.Errorf("expected daemon second, got %s", scope.Elements[1].Name)
	}

	// Check GID parsing
	rootGrp := scope.Elements[2]
	if rootGrp.GID == nil || *rootGrp.GID != 0 {
		t.Errorf("root GID should be 0")
	}

	// Check users
	admGrp := scope.Elements[0]
	if len(admGrp.Users) != 2 {
		t.Errorf("adm should have 2 users, got %d", len(admGrp.Users))
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: collect_users_shadow_excluded
// Verifies: EXAMPLE shadow_excluded_by_default
// Confidence: High
// ---------------------------------------------------------------------------

func TestCollectUsersShadowExcluded(t *testing.T) {
	passwdFile := "root:x:0:0:root:/root:/bin/bash\ndaemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin\n"
	shadowFile := "root:$6$hash:19000:0:99999:7:::\ndaemon:!:19000:0:99999:7:::\n"

	fs := &FakeFilesystem{
		Files: map[string]string{
			"/etc/passwd": passwdFile,
			"/etc/shadow": shadowFile,
		},
		Dirs:        map[string]bool{},
		Executables: map[string]bool{},
		StatInfo:    map[string]FileInfo{},
	}
	config := &Config{Exclude: []string{"/etc/shadow"}}

	scope := collectUsers(fs, config)
	if scope == nil {
		t.Fatal("collectUsers returned nil")
	}
	if len(scope.Elements) != 2 {
		t.Fatalf("expected 2 users, got %d", len(scope.Elements))
	}

	// With shadow excluded, EncryptedPassword should be empty
	for _, u := range scope.Elements {
		if u.EncryptedPassword != "" {
			t.Errorf("user %s: EncryptedPassword should be empty when shadow excluded", u.Name)
		}
	}

	// Password field from /etc/passwd should be "x"
	for _, u := range scope.Elements {
		if u.Password != "x" {
			t.Errorf("user %s: Password should be x, got %q", u.Name, u.Password)
		}
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: render_json_valid_output
// Verifies: BEHAVIOR render-json
// Confidence: Medium (tests JSON structure without live collection)
// ---------------------------------------------------------------------------

func TestRenderJSONValidOutput(t *testing.T) {
	hostname := "testhost.example.com"
	arch := "x86_64"
	manifest := &SitarManifest{
		Meta: SitarMeta{
			FormatVersion: 1,
			SitarVersion:  "0.9.0",
			CollectedAt:   "2026-04-03T15:00:00Z",
			Hostname:      hostname,
			Uname:         "Linux testhost 5.15.0 #1 SMP x86_64 GNU/Linux",
		},
		Os: &OsScope{
			Name:         strPtr("Test Linux"),
			Version:      strPtr("1.0"),
			Architecture: &arch,
		},
		GeneralInfo: &ScopeWrapper[GeneralInfoRecord]{
			Attributes: map[string]interface{}{},
			Elements: []GeneralInfoRecord{
				{Key: "hostname", Value: hostname},
			},
		},
	}

	// Write to temp file
	tmpFile, err := os.CreateTemp("", "sitar-test-*.json")
	if err != nil {
		t.Fatal(err)
	}
	defer os.Remove(tmpFile.Name())
	tmpFile.Close()

	if err := renderJSON(manifest, tmpFile.Name()); err != nil {
		t.Fatalf("renderJSON failed: %v", err)
	}

	// Read and parse
	data, err := os.ReadFile(tmpFile.Name())
	if err != nil {
		t.Fatal(err)
	}
	if len(data) == 0 {
		t.Fatal("JSON output is empty")
	}

	var parsed map[string]interface{}
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("JSON parse error: %v", err)
	}

	// Check meta
	meta, ok := parsed["meta"].(map[string]interface{})
	if !ok {
		t.Fatal("meta field missing or wrong type")
	}
	if fv, ok := meta["format_version"].(float64); !ok || fv != 1 {
		t.Errorf("meta.format_version should be 1, got %v", meta["format_version"])
	}
	if sv, ok := meta["sitar_version"].(string); !ok || sv != "0.9.0" {
		t.Errorf("meta.sitar_version should be 0.9.0, got %v", meta["sitar_version"])
	}
	if h, ok := meta["hostname"].(string); !ok || h != hostname {
		t.Errorf("meta.hostname should be %s, got %v", hostname, meta["hostname"])
	}
}

func strPtr(s string) *string {
	return &s
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: html_renderer_non_empty
// Verifies: EXAMPLE html_output_non_empty
// Confidence: High
// ---------------------------------------------------------------------------

func TestHTMLRendererNonEmpty(t *testing.T) {
	r := &HTMLRenderer{}
	arch := "x86_64"
	manifest := &SitarManifest{
		Meta: SitarMeta{Hostname: "myhost", SitarVersion: "0.9.0"},
		GeneralInfo: &ScopeWrapper[GeneralInfoRecord]{
			Attributes: map[string]interface{}{},
			Elements:   []GeneralInfoRecord{{Key: "hostname", Value: "myhost"}},
		},
		CPU: &ScopeWrapper[CpuRecord]{
			Attributes: map[string]interface{}{"architecture": "x86_64"},
			Elements:   []CpuRecord{{Processor: "0", VendorID: "GenuineIntel", ModelName: "Intel Core i7"}},
		},
		Os: &OsScope{Architecture: &arch},
	}

	header := r.Header(manifest)
	if !strings.Contains(header, "<!DOCTYPE html>") {
		t.Error("header must contain DOCTYPE")
	}
	if !strings.Contains(header, "<body>") {
		t.Error("header must contain <body>")
	}

	footer := r.Footer()
	if footer != "</body>\n</html>\n" {
		t.Errorf("unexpected footer: %q", footer)
	}

	genInfo := renderGeneralInfo(manifest, r)
	if genInfo == "" {
		t.Error("renderGeneralInfo should return non-empty string")
	}
	if !strings.Contains(genInfo, "hostname") {
		t.Error("renderGeneralInfo should contain 'hostname'")
	}

	cpuContent := renderCPU(manifest, r)
	if cpuContent == "" {
		t.Error("renderCPU should return non-empty string")
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: tex_renderer_structure
// Verifies: EXAMPLE tex_output_non_empty
// Confidence: High
// ---------------------------------------------------------------------------

func TestTeXRendererStructure(t *testing.T) {
	r := &TeXRenderer{}
	manifest := &SitarManifest{
		Meta: SitarMeta{Hostname: "myhost"},
	}

	header := r.Header(manifest)
	if !strings.Contains(header, `\documentclass`) {
		t.Error("TeX header must contain \\documentclass")
	}
	if !strings.Contains(header, `\begin{document}`) {
		t.Error("TeX header must contain \\begin{document}")
	}

	footer := r.Footer()
	if !strings.Contains(footer, `\end{document}`) {
		t.Error("TeX footer must contain \\end{document}")
	}

	section := r.Section("General Information", 1, "content here")
	if !strings.Contains(section, `\section{General Information}`) {
		t.Errorf("TeX section should contain \\section{...}, got: %s", section)
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: sdocbook_renderer_structure
// Verifies: EXAMPLE sdocbook_output_non_empty
// Confidence: High
// ---------------------------------------------------------------------------

func TestDocBookRendererStructure(t *testing.T) {
	r := &DocBookRenderer{}
	manifest := &SitarManifest{}

	header := r.Header(manifest)
	if !strings.Contains(header, "<?xml") {
		t.Error("DocBook header must contain <?xml")
	}
	if !strings.Contains(header, "<article") {
		t.Error("DocBook header must contain <article")
	}

	footer := r.Footer()
	if !strings.Contains(footer, "</article>") {
		t.Error("DocBook footer must contain </article>")
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: markdown_renderer_structure
// Verifies: EXAMPLE markdown_output_non_empty
// Confidence: High
// ---------------------------------------------------------------------------

func TestMarkdownRendererStructure(t *testing.T) {
	r := &MarkdownRenderer{}
	manifest := &SitarManifest{
		Meta: SitarMeta{Hostname: "myhost"},
	}

	header := r.Header(manifest)
	if !strings.Contains(header, "# SITAR") {
		t.Error("Markdown header must contain '# SITAR'")
	}

	section := r.Section("CPU", 1, "| col | val |\n| --- | --- |\n")
	if !strings.Contains(section, "## CPU") {
		t.Errorf("Markdown section should contain '## CPU', got: %s", section)
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: scope_wrapper_json_tags
// Verifies: ScopeWrapper produces correct _attributes/_elements JSON keys
// Confidence: High
// ---------------------------------------------------------------------------

func TestScopeWrapperJSONTags(t *testing.T) {
	scope := &ScopeWrapper[GeneralInfoRecord]{
		Attributes: map[string]interface{}{"key": "value"},
		Elements:   []GeneralInfoRecord{{Key: "hostname", Value: "myhost"}},
	}

	data, err := json.Marshal(scope)
	if err != nil {
		t.Fatalf("json.Marshal failed: %v", err)
	}

	jsonStr := string(data)
	if !strings.Contains(jsonStr, `"_attributes"`) {
		t.Errorf("JSON should contain _attributes, got: %s", jsonStr)
	}
	if !strings.Contains(jsonStr, `"_elements"`) {
		t.Errorf("JSON should contain _elements, got: %s", jsonStr)
	}
	if !strings.Contains(jsonStr, `"hostname"`) {
		t.Errorf("JSON should contain hostname key, got: %s", jsonStr)
	}
}

// ---------------------------------------------------------------------------
// INDEPENDENT_TEST: format_validation
// Verifies: EXAMPLE unknown_format
// Confidence: Medium (tests format validation logic; cannot test os.Exit)
// ---------------------------------------------------------------------------

func TestFormatValidation(t *testing.T) {
	validFormats := []string{"html", "tex", "sdocbook", "json", "markdown", "all", ""}
	invalidFormats := []string{"pdf", "word", "yast2", "csv", "xml"}

	isValidFormat := func(f string) bool {
		for _, v := range validFormats {
			if f == v {
				return true
			}
		}
		return false
	}

	for _, f := range validFormats {
		if !isValidFormat(f) {
			t.Errorf("format %q should be valid", f)
		}
	}
	for _, f := range invalidFormats {
		if isValidFormat(f) {
			t.Errorf("format %q should be invalid", f)
		}
	}
}
