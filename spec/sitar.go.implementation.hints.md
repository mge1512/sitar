# Hints: sitar implementation — Go

Template:  cli-tool
Language:  Go
Component: sitar
Spec:      sitar-009.md

These hints are sitar-specific. They complement cli-tool.go.milestones.hints.md,
which covers Go scaffold-first patterns that apply to any cli-tool.
Read the spec, then cli-tool.go.milestones.hints.md, then this file.
These hints are advisory. They cannot override spec invariants.

NOTE FOR TESTING: This file intentionally separates sitar-specific knowledge
from general Go cli-tool knowledge. A translation attempt without this file
should still produce a correct M0 scaffold using only the spec and the
generic hints file. The sitar-specific details here improve quality but are
not required for correctness.

---

## Recommended file layout for sitar

Following the milestone groupings in the spec:

```
main.go             — entry point, prepareConfig(), main()
types.go            — all type definitions (SitarManifest and all scopes)
interfaces.go       — Filesystem, CommandRunner, Renderer, PackageBackend
                      interfaces + all production and test-double implementations
detect.go           — detectDistribution()
collect.go          — collect() orchestrator + collect-general-info,
                      collect-environment, collect-os
collect_hw.go       — collect-cpu, collect-kernel-params, collect-net-params,
                      collect-devices, collect-pci, collect-processes, collect-dmi
collect_storage.go  — collect-storage, collect-btrfs
collect_network.go  — collect-network-interfaces, collect-network-routing,
                      collect-network-firewall
collect_pkg.go      — collect-installed-rpm, collect-installed-deb,
                      collect-repositories, collect-services, collect-chkconfig,
                      collect-groups, collect-users, collect-changed-config-files,
                      collect-changed-managed-files, collect-kernel-config
collect_config.go   — collect-config-files, collect-security-apparmor,
                      find-unpacked, check-consistency
render.go           — render() dispatch, output path resolution
render_human.go     — render-human + all 30 renderXxx() functions
render_json.go      — render-json
main_test.go        — unit tests using FakeFilesystem and FakeCommandRunner
```

---

## SitarMeta struct — JSON tag example

```go
type SitarMeta struct {
    FormatVersion int    `json:"format_version"`
    SitarVersion  string `json:"sitar_version"`
    CollectedAt   string `json:"collected_at"`
    Hostname      string `json:"hostname"`
    Uname         string `json:"uname"`
}
```

---

## getHostname and getUname — must call CommandRunner

Both functions must invoke real commands via OSCommandRunner.Run.
Never return hardcoded strings:

```go
func getHostname(cr CommandRunner) string {
    stdout, _, _ := cr.Run("hostname", []string{"-f"})
    return strings.TrimSpace(stdout)
}

func getUname(cr CommandRunner) string {
    stdout, _, _ := cr.Run("uname", []string{"-a"})
    return strings.TrimSpace(stdout)
}
```

Returning "localhost" or "" caused silent failures in all previous runs.

---

## debugLog env var for sitar

Use SITAR_DEBUG as the environment variable name:

```go
func debugLog(msg string) {
    if os.Getenv("SITAR_DEBUG") == "1" {
        fmt.Fprintf(os.Stderr, "DEBUG: %s\n", msg)
    }
}
```

---

## render_human.go — all 30 stub function names

Create render_human.go in M0 with these 30 stub functions.
Each has signature: func renderXxx(manifest *SitarManifest, r SitarRenderer) string

In SECTION-MAP order:
```
renderGeneralInfo        renderCPU               renderKernelParams
renderNetParams          renderDevices           renderPCI
renderSoftwareRAID       renderPartitions        renderBtrfs
renderFstab              renderLvmConf           renderEVMS
renderMultipath          renderIDE               renderSCSI
renderCCISS              renderAreca             renderDAC960
renderGDTH               renderIPS               renderCompaqSmart
renderNetworkInterfaces  renderRouting           renderPacketFilter
renderAppArmor           renderDMI               renderServices
renderChangedConfigFiles renderPackages          renderKernelConfig
```

---

## Machinery-compatible JSON scope initialisation

The spec requires _attributes and _elements even for empty scopes.
For sitar scopes, initialise as:

```go
manifest.CPU = &CpuScope{
    Attributes: map[string]interface{}{},
    Elements:   []CpuRecord{},
}
```

Not:
```go
manifest.CPU = nil      // produces "cpu": null — wrong
manifest.CPU = &CpuScope{}  // produces "cpu": {} — missing _elements key
```

---

## .include file parsing — no Perl execution

collect-config-files reads /var/lib/support/*.include files.
Each contains: @files = ( "/path/a", "/path/b" );
Parse with a regex — do NOT execute Perl:

```go
re := regexp.MustCompile(`"([^"]+)"`)
matches := re.FindAllStringSubmatch(content, -1)
for _, m := range matches {
    files = append(files, m[1])
}
```

---

## format=yast2 — do not implement

yast2 is explicitly removed from scope in the spec's OutputFormat type.
Do not add a yast2 case anywhere. Previous translators added it as dead
code from reading the sitar.pl original.

---

## Milestone verification commands

After running each milestone as root, verify with these commands.
Save as a script or run inline.

**M1 (identity):**
```bash
./sitar format=json outfile=/tmp/sitar-m1.json
python3 - << 'EOF'
import json
d = json.load(open('/tmp/sitar-m1.json'))
assert d['meta']['format_version'] == 1
assert d['meta']['sitar_version'] == '0.9.0'
assert d['meta']['hostname'] != ''
assert d['meta']['uname'] != ''
assert d['os']['architecture'] is not None
assert len(d['general_info']['_elements']) == 9
assert next(r for r in d['general_info']['_elements'] if r['key']=='hostname')['value'] != ''
print("M1 PASS")
EOF
```

**M2 (hardware):**
```bash
./sitar format=json outfile=/tmp/sitar-m2.json
python3 - << 'EOF'
import json
d = json.load(open('/tmp/sitar-m2.json'))
assert len(d['cpu']['_elements']) > 0
assert d['cpu']['_elements'][0]['vendor_id'] != ''
assert len(d['kernel_params']['_elements']) >= 20
assert len(d['net_params']['_elements']) > 0
assert len(d['devices']['_elements']) > 0
assert len(d['processes']['_elements']) > 0
assert d['processes']['_elements'][0]['pid'] != ''
print("M2 PASS")
EOF
```

**M3 (storage):**
```bash
./sitar format=json outfile=/tmp/sitar-m3.json
python3 - << 'EOF'
import json
d = json.load(open('/tmp/sitar-m3.json'))
assert len(d['storage']['partitions']['_elements']) > 0
p = d['storage']['partitions']['_elements'][0]
assert p['device'] != ''
assert p['source'] in ('lsblk', 'fdisk')
print("M3 PASS")
EOF
```

**M4 (network):**
```bash
./sitar format=json outfile=/tmp/sitar-m4.json
python3 - << 'EOF'
import json
d = json.load(open('/tmp/sitar-m4.json'))
ifaces = d['network']['interfaces']['_elements']
assert len(ifaces) > 0
assert any(i['ifname'] == 'lo' for i in ifaces)
assert len(d['network']['routes']['_elements']) > 0
assert d['network']['packet_filter']['_elements'][0]['engine'] != ''
print("M4 PASS")
EOF
```

**M5 (packages):**
```bash
./sitar format=json outfile=/tmp/sitar-m5.json
python3 - << 'EOF'
import json
d = json.load(open('/tmp/sitar-m5.json'))
assert len(d['packages']['_elements']) > 0
assert d['packages']['_elements'][0]['name'] != ''
assert d['packages']['_attributes']['package_system'] in ('rpm', 'dpkg')
assert len(d['services']['_elements']) > 0
assert len(d['groups']['_elements']) > 0
assert len(d['users']['_elements']) > 0
print("M5 PASS")
EOF
```

**M6 (renderers):**
```bash
./sitar all outdir=/tmp/sitar-m6
grep -q "General Information" /tmp/sitar-m6/*.html && echo "html OK"
grep -q "\\documentclass"     /tmp/sitar-m6/*.tex  && echo "tex OK"
grep -q "<?xml"               /tmp/sitar-m6/*.xml  && echo "xml OK"
grep -q "# SITAR"             /tmp/sitar-m6/*.md   && echo "md OK"
test $(stat -c%s /tmp/sitar-m6/*.html) -gt 0       && echo "html non-empty"
```

**M7 (complete):**
```bash
./sitar all outdir=/tmp/sitar-m7
test $(stat -c%s /tmp/sitar-m7/*.html) -gt 50000   && echo "html size OK"
test $(stat -c%s /tmp/sitar-m7/*.json) -gt 100000  && echo "json size OK"
./sitar check-consistency
test -f /var/lib/support/Configuration_Consistency.include && echo "cache OK"
./sitar find-unpacked
test -f /var/lib/support/Find_Unpacked.include             && echo "unpacked OK"
```

---

## Known sitar-specific failure modes from previous runs

1. **getHostname() returned "localhost"** — must call Run("hostname", ["-f"]).
2. **getUname() returned ""** — must call Run("uname", ["-a"]).
3. **render_human.go absent** — create with all 30 stubs in M0.
4. **renderSection(interface{}) with 4-case type switch** — write 30 typed functions.
5. **format=yast2 guard implemented** — yast2 is removed from scope; do not add it.
6. **Sonnet generated Perl scripts** for check-consistency and find-unpacked
   instead of Go code — these are Go BEHAVIORs that happen to write Perl-syntax
   cache files. The Go code writes the Perl array string; it does not invoke Perl.
