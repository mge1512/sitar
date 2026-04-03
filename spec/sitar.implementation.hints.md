# Hints: sitar implementation

Template:  cli-tool
Component: sitar
Spec:      sitar-009.md

These hints are sitar-specific and language-neutral. They describe
what to implement, not how to express it in any particular language.
Read the spec first, then the language-specific cli-tool hints file
for your target language, then this file.

These hints are advisory. They cannot override spec invariants.

NOTE FOR TESTING: A correct M0 scaffold is achievable from the spec
and the language-specific hints file alone. This file improves quality
and prevents known failure modes but is not required for correctness.

---

## Recommended source file grouping

Split the implementation into files following the milestone groupings.
Each milestone should touch at most two or three files. Suggested
grouping by BEHAVIOR, independent of language file naming conventions:

| File / Module          | BEHAVIORs                                                     |
|------------------------|---------------------------------------------------------------|
| main / entry           | prepare-config, main entry point                              |
| types / model          | all type definitions (SitarManifest and all scopes)           |
| interfaces / ports     | Filesystem, CommandRunner, Renderer, PackageBackend           |
|                        | + all production and test-double implementations              |
| detect                 | detect-distribution                                           |
| collect (core)         | collect orchestrator, collect-general-info,                   |
|                        | collect-environment, collect-os                               |
| collect_hw             | collect-cpu, collect-kernel-params, collect-net-params,       |
|                        | collect-devices, collect-pci, collect-processes, collect-dmi  |
| collect_storage        | collect-storage, collect-btrfs                                |
| collect_network        | collect-network-interfaces, collect-network-routing,          |
|                        | collect-network-firewall                                      |
| collect_pkg            | collect-installed-rpm, collect-installed-deb,                 |
|                        | collect-repositories, collect-services, collect-chkconfig,    |
|                        | collect-groups, collect-users, collect-changed-config-files,  |
|                        | collect-changed-managed-files, collect-kernel-config          |
| collect_config         | collect-config-files, collect-security-apparmor,              |
|                        | find-unpacked, check-consistency                              |
| render (dispatch)      | render, output path resolution                                |
| render_human           | render-human + all 30 render section functions                |
| render_json            | render-json                                                   |
| tests                  | unit tests using FakeFilesystem and FakeCommandRunner         |

---

## SitarMeta — required JSON field names

The meta scope must serialise to these exact JSON keys regardless of
how the language represents them internally:

```
format_version   integer   always 1
sitar_version    string    e.g. "0.9.0"
collected_at     string    UTC ISO 8601
hostname         string    FQDN from hostname -f
uname            string    full output of uname -a
```

In languages where field names do not naturally serialise to
underscore_style (e.g. Java camelCase, C++ member names), explicit
serialisation mapping is required — do not rely on automatic name
conversion.

---

## getHostname and getUname — must call CommandRunner

Both functions must invoke real commands via the CommandRunner
interface. Never return hardcoded strings.

```
getHostname: call Run("hostname", ["-f"]), trim whitespace, return result
getUname:    call Run("uname",    ["-a"]), trim whitespace, return result
```

Returning "localhost" or "" caused silent failures in all previous
translation runs. The spec POSTCONDITIONS forbid both. These functions
are called during M1 and must be real in that milestone.

---

## Debug logging — env var name

The component-specific environment variable for debug logging is:

    SITAR_DEBUG=1

When set: stubs and collection modules may emit diagnostic messages
to stderr. At normal verbosity (SITAR_DEBUG unset or != "1") all
stubs must be silent.

---

## render_human — 30 required section functions

Create stubs for all 30 functions in M0. Fill them in during M6.
Each function takes the manifest and a Renderer instance and returns
a string (empty string is the correct stub return value).

In SECTION-MAP order — adapt to your language's naming convention
(camelCase, snake_case, PascalCase) but preserve the logical names:

```
render_general_info         render_cpu                render_kernel_params
render_net_params           render_devices            render_pci
render_software_raid        render_partitions         render_btrfs
render_fstab                render_lvm_conf           render_evms
render_multipath            render_ide                render_scsi
render_cciss                render_areca              render_dac960
render_gdth                 render_ips                render_compaq_smart
render_network_interfaces   render_routing            render_packet_filter
render_apparmor             render_dmi                render_services
render_changed_config_files render_packages           render_kernel_config
```

Do not implement a single generic render function that dispatches on
type at runtime. Write 30 separate typed functions, one per scope.
A generic dispatcher that covers only some known types silently drops
all others — this was the failure mode in all previous translation runs.

---

## Machinery-compatible scope serialisation

Every scope in the JSON output must serialise as an object with both
`_attributes` and `_elements` keys, even when empty:

```json
"cpu": { "_attributes": {}, "_elements": [] }
```

Never:
```json
"cpu": null
"cpu": {}
```

`null` is produced when the scope field is a null pointer / None /
null reference. `{}` is produced when the scope object exists but
has no serialisable fields. Both are wrong.

The correct zero value is an initialised scope object with an empty
attributes map and an empty elements collection. In every language,
initialise all scope fields to this empty-but-valid state in M0 stubs
before any real collection logic is added.

---

## .include file parsing — no Perl execution

collect-config-files reads `/var/lib/support/*.include` files.
Each file contains a Perl array literal:

```
@files = ( "/path/to/file1", "/path/to/file2" );
```

Parse this with string or regex operations in the implementation
language. Do NOT invoke Perl, do NOT use eval or equivalent.
Extract all double-quoted strings between the outer parentheses.

The pattern: match all occurrences of `"([^"]+)"` in the file content.
This is sufficient — the format is fixed and simple.

---

## format=yast2 — do not implement

`yast2` is explicitly removed from the OutputFormat type in the spec.
Do not add a yast2 case to the format dispatcher, the output path
resolver, or anywhere else.

Previous translators added it as dead code after reading the original
`sitar.pl` source. The spec is the authority, not the Perl original.

---

## check-consistency and find-unpacked write Perl-syntax cache files

These two BEHAVIORs produce output files at:

```
/var/lib/support/Configuration_Consistency.include
/var/lib/support/Find_Unpacked.include
```

The file format is a Perl array literal (so that legacy tools reading
`/var/lib/support/*.include` can parse them):

```
@files = ( "/path/a", "/path/b" );
```

The implementation writes this string using normal file I/O in the
implementation language. It does not invoke Perl. The output is plain
text that happens to be valid Perl syntax.

A previous translation generated actual Perl scripts for these
BEHAVIORs instead of implementation-language code that writes the
cache file. The BEHAVIORs are implemented in the target language;
only their output format is Perl syntax.

---

## Milestone verification commands

Run as root after each milestone pass. These commands are
language-independent — they examine only the binary's output.
Requires: jq (available on all target distributions).

**M0 (scaffold):**
```bash
./sitar version
./sitar help
./sitar format=unknown_format   # must exit 2
```

**M1 (identity):**
```bash
./sitar format=json outfile=/tmp/sitar-m1.json
jq -e '
  .meta.format_version == 1                                    and
  .meta.sitar_version == "0.9.0"                              and
  (.meta.hostname | length) > 0                               and
  (.meta.uname | length) > 0                                  and
  (.os.architecture | length) > 0                             and
  (.general_info._elements | length) == 9                     and
  (.general_info._elements[] | select(.key=="hostname")
                              | .value | length) > 0
' /tmp/sitar-m1.json && echo "M1 PASS"
```

**M2 (hardware):**
```bash
./sitar format=json outfile=/tmp/sitar-m2.json
jq -e '
  (.cpu._elements | length) > 0                               and
  (.cpu._elements[0].vendor_id | length) > 0                  and
  (.kernel_params._elements | length) >= 20                   and
  (.net_params._elements | length) > 0                        and
  (.devices._elements | length) > 0                           and
  (.processes._elements | length) > 0                         and
  (.processes._elements[0].pid | length) > 0
' /tmp/sitar-m2.json && echo "M2 PASS"
```

**M3 (storage):**
```bash
./sitar format=json outfile=/tmp/sitar-m3.json
jq -e '
  (.storage.partitions._elements | length) > 0                and
  (.storage.partitions._elements[0].device | length) > 0      and
  (.storage.partitions._elements[0].source | IN("lsblk","fdisk"))
' /tmp/sitar-m3.json && echo "M3 PASS"
```

**M4 (network):**
```bash
./sitar format=json outfile=/tmp/sitar-m4.json
jq -e '
  (.network.interfaces._elements | length) > 0                and
  (.network.interfaces._elements[] | select(.ifname=="lo"))   and
  (.network.routes._elements | length) > 0                    and
  (.network.packet_filter._elements | length) > 0             and
  (.network.packet_filter._elements[0].engine | length) > 0
' /tmp/sitar-m4.json && echo "M4 PASS"
```

**M5 (packages):**
```bash
./sitar format=json outfile=/tmp/sitar-m5.json
jq -e '
  (.packages._elements | length) > 0                          and
  (.packages._elements[0].name | length) > 0                  and
  (.packages._attributes.package_system | IN("rpm","dpkg"))   and
  (.services._elements | length) > 0                          and
  (.groups._elements | length) > 0                            and
  (.users._elements | length) > 0
' /tmp/sitar-m5.json && echo "M5 PASS"
```

**M6 (renderers):**
```bash
./sitar all outdir=/tmp/sitar-m6
grep -q "General Information" /tmp/sitar-m6/*.html && echo "html: General Information OK"
grep -q "\\documentclass"     /tmp/sitar-m6/*.tex  && echo "tex: documentclass OK"
grep -q "<?xml"               /tmp/sitar-m6/*.xml  && echo "xml: header OK"
grep -q "# SITAR"             /tmp/sitar-m6/*.md   && echo "md: title OK"
test $(stat -c%s /tmp/sitar-m6/*.html) -gt 0       && echo "html: non-empty"
```

**M7 (complete):**
```bash
./sitar all outdir=/tmp/sitar-m7
test $(stat -c%s /tmp/sitar-m7/*.html) -gt 50000   && echo "html: size OK"
test $(stat -c%s /tmp/sitar-m7/*.json) -gt 100000  && echo "json: size OK"
./sitar check-consistency
test -f /var/lib/support/Configuration_Consistency.include && echo "cache: OK"
./sitar find-unpacked
test -f /var/lib/support/Find_Unpacked.include             && echo "unpacked: OK"
```

---

## Known sitar-specific failure modes from previous runs

Language-neutral descriptions of failures observed in real translation
runs. All four apply regardless of target language:

1. **getHostname returned "localhost"** — must call Run("hostname", ["-f"]).
   The CommandRunner interface exists precisely to make this testable.

2. **getUname returned ""** — must call Run("uname", ["-a"]).

3. **The render_human module was absent entirely from M0** — all 30
   section render functions must be created as stubs in M0. Later
   milestone translators cannot safely add new functions to a module
   that does not yet exist.

4. **A single generic render dispatcher replaced 30 typed functions** —
   covering only 4 of 30 scopes, silently dropping the other 26.
   Write 30 separate functions, one per scope.

5. **format=yast2 was implemented as dead code** — carried forward from
   reading sitar.pl. The spec removes yast2. The spec is the authority.

6. **check-consistency and find-unpacked were generated as Perl scripts**
   rather than implementation-language code that writes a Perl-syntax
   output file. The BEHAVIORs are in the target language; only their
   output format is a Perl array literal.
