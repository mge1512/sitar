# sitar translation analysis: Go and Rust vs sitar-009.md

Date: 2026-04-05  
Scope: sitar/code/go and sitar/code/rs vs sitar/spec/sitar.md  
Purpose: identify spec gaps and deviations; no code changes; no re-translation

---

## Executive summary

Both translations are substantially correct. The spec successfully drove two
independent implementations in different languages to consistent structural
outcomes: same file layout, same 30 render functions, same interface test
doubles, same milestone coverage. The scaffold-first approach worked — no
structural deviations were found in either translation.

Four issues warrant spec changes. One (cache file format) explains the
consistency table bug you observed. The remaining issues are minor.

---

## Issue 1 — Cache file format mismatch (explains the consistency bug)

**Severity: High**

The Go translation writes cache files in Perl array syntax:

```
@files = (
  "/etc/sysconfig/network",
  "/etc/hosts",
);
```

File names: `Configuration_Consistency.include`, `Find_Unpacked.include`  
Reading method: regex `"([^"]+)"` to extract quoted strings

The Rust translation writes cache files as JSON arrays:

```json
[
  "/etc/sysconfig/network",
  "/etc/hosts"
]
```

File names: `Configuration_Consistency.json`, `Find_Unpacked.json`  
Reading method: standard JSON parser

**Root cause:** The Go translation ran at approximately 11:09 on 2026-04-03,
using `sitar.go.implementation.hints.md` (the old Go-specific hints, still
specifying Perl format). The spec and language-neutral hints were updated
to JSON at 20:54 the same day. The Rust translation ran after that update
and correctly uses JSON throughout.

The Go implementation is therefore internally consistent with itself
(writes `.include`, reads `*.include`) but inconsistent with the current
spec and with the Rust implementation.

**The consistency table "error":** The `checkConsistency` function runs
`rpm -V` correctly and collects changed config file paths correctly. The
issue is upstream of rendering — the output path and format of the cache
file diverge from what the current spec and other tooling expect. The
rendering function `renderChangedConfigFiles` is correct and would work
properly once the collection feeds into `manifest.ChangedConfigFiles`, but
the cache file mechanism is writing to the wrong format.

**Spec status:** The spec is correct (JSON). The Go code needs updating.
Since you are not patching code, this is a tracked known deviation in the
Go implementation: it predates the Perl→JSON spec change.

**Spec action:** None — the spec is already correct. Document in the Go
translation report addendum that this is a pre-change artefact.

**Recommendation for the next Go milestone pass or maintenance run:**
The fix is confined to `collect_config.go` (three locations: cache file
names, write logic, read glob pattern). It is a targeted change, not a
re-translation.

---

## Issue 2 — Absent optional scopes serialise as null in Rust

**Severity: Medium**

In Rust, `security_apparmor` and `dmi` are declared as `Option<T>` without
`#[serde(skip_serializing_if = "Option::is_none")]`. When absent (AppArmor
not present, dmidecode not executable), they serialise as JSON `null` rather
than being omitted from the output.

Go handles this correctly: both fields use pointer types with `json:",omitempty"`
tags, so they are absent from JSON when nil.

The spec says "omit section if null" for the render functions, but does not
explicitly state that the JSON serialisation itself must omit null fields.
The Rust report correctly identifies this as a known minor deviation.

**Spec action required:** Add an INVARIANT:

```
- [observable]  Optional scopes absent from collection (not found or not
                applicable) MUST be omitted from JSON output entirely.
                They MUST NOT appear as JSON null.
                Translators MUST use the appropriate serialisation annotation
                for the target language:
                  Go:   json:",omitempty" on pointer fields
                  Rust: #[serde(skip_serializing_if = "Option::is_none")]
                  Java: @JsonInclude(Include.NON_NULL)
                  C++:  omit field from serialisation when std::optional empty
```

---

## Issue 3 — SITAR_READFILE_LIMIT not a named constant in Go

**Severity: Low**

The spec defines `SITAR_READFILE_LIMIT := 32767` as a named constant in
the TYPES section, used by `collect-kernel-params`, `collect-net-params`,
and `collect-security-apparmor`.

Rust: correct — `const SITAR_READFILE_LIMIT: usize = 32767;` defined at
the top of `collect_hw.rs` and `collect_config.rs`, referenced by name.

Go: uses the magic number `32767` as a local `const limit = 32767` in two
separate function bodies, and passes `32767` directly in one call in
`collect_config.go`. The name `SITAR_READFILE_LIMIT` does not appear anywhere
in the Go code.

This is a minor violation of the spec's intent — the named constant exists
precisely so that the limit is defined in one place and can be changed
consistently.

**Spec action:** Strengthen the TYPES definition with a note:

```
SITAR_READFILE_LIMIT := 32767
// This MUST be declared as a named constant in the implementation,
// not as a magic number. Used by: collect-kernel-params,
// collect-net-params, collect-security-apparmor.
```

---

## Issue 4 — Per-package checksum query omitted in both translations

**Severity: Low**

The spec's `collect-installed-rpm` STEP 4 says: "Retrieve checksum: run
`rpm -q --queryformat '%{MD5SUM}\n' <n>`. Store as checksum field."

Both Go and Rust omit this step for performance reasons (one subprocess
per package, thousands of packages = prohibitively slow). Both document
the deviation in their translation reports. The `checksum` field exists in
`PackageRecord` but is always empty string.

Both translators independently made the same pragmatic decision, suggesting
the spec requirement is unrealistic as written.

**Spec action required:** Mark the step as conditionally optional:

```
4. OPTIONAL: Retrieve checksum per package via
   `rpm -q --queryformat '%{MD5SUM}\n' <n>`.
   This step MAY be omitted when the installed package count exceeds
   a configurable threshold (suggested: 100 packages), as it requires
   one subprocess invocation per package.
   When omitted: checksum field MUST be set to "".
   When performed: store result as checksum field.
```

---

## Issue 5 — detect-distribution step 7 should check rpm presence

**Severity: Low**

The spec's `detect-distribution` STEP 7 says: if `/etc/os-release` is
readable, set `family=rpm`. Both Go and Rust implement this correctly for
rpm-family systems. However, on a system where `/etc/os-release` exists
but rpm is absent (e.g. a minimal Alpine-based container using /etc/os-release
without an rpm backend), the spec would cause a false `family=rpm` detection.

Go passes `/usr/bin/rpm` as `RpmCmd` regardless. Rust does the same. Neither
checks whether rpm is actually executable in this fallback path.

**Spec action:** Add a guard to step 7:

```
7. If /etc/os-release is readable:
   Read PRETTY_NAME value as release.
   If `rpm` is executable in PATH: family=rpm, backend=rpm.
   Else: family=unknown, backend=none.
   Return.
```

---

## What worked well — observations for the PCD generalisation letter

These are worth recording as positive findings:

**1. File layout was identical in both languages.** The hints file
recommendation produced the exact same module split (collect_hw, collect_storage,
collect_network, collect_pkg, collect_config, render_human, render_json) in
both Go and Rust. This validates the hints file approach.

**2. All 30 render functions present in both.** Both implementations have
exactly 30 typed render functions in the correct SECTION-MAP order. Neither
produced a generic dispatcher. The explicit function list in the hints file
was decisive.

**3. The `safeRun` pattern (Go) vs `unwrap_or_default` (Rust).** Both
translators independently solved the "one module failure must not abort
the entire collection" requirement in idiomatic ways for their languages.
The spec does not prescribe the mechanism — it states the requirement
("skip gracefully if not executable", "partial results are acceptable").
This is the correct level of abstraction in the spec.

**4. Distribution detection precedence was identical.** Both follow the
9-step order exactly. The spec was unambiguous enough that both translators
produced the same precedence chain without any hints file guidance on this.

**5. OSCommandRunner.Run implemented correctly in both.** Both use the
platform subprocess API (os/exec in Go, std::process::Command in Rust)
with PATH isolation. Neither stubbed it. The hints file warning was heeded.

**6. Storage source field tracking.** Both set `source: "lsblk"` or
`source: "fdisk"` correctly, and both set begin_sector/end_sector to empty
string when lsblk is the source. The spec was precise enough here.

---

## Summary of spec changes recommended

| # | Change | Location | Priority |
|---|--------|----------|----------|
| 1 | Cache format already correct in spec; document Go as pre-change artefact | — | Tracking only |
| 2 | Add INVARIANT: absent optional scopes must be omitted from JSON, not null | INVARIANTS section | High |
| 3 | Strengthen SITAR_READFILE_LIMIT: must be named constant, not magic number | TYPES section | Low |
| 4 | Mark per-package checksum query as optional above 100-package threshold | collect-installed-rpm STEP 4 | Low |
| 5 | Add rpm presence check to detect-distribution STEP 7 | detect-distribution STEP 7 | Low |

Only change 2 is High priority. Changes 3-5 are quality improvements based
on what both translators independently chose to do. Change 1 requires no
spec edit — the spec is already correct.

---

## Go implementation: known deviations from current spec

For reference — deviations that exist in the Go code relative to sitar-009.md
as it stands today (not failures, just tracked divergences):

| Deviation | Location | Impact |
|-----------|----------|--------|
| Cache files use `.include` Perl format | collect_config.go | Consistency table rendering broken |
| Cache file glob reads `*.include` | collect_config.go | Will not read Rust-generated `.json` caches |
| `SITAR_READFILE_LIMIT` not a named constant | collect_hw.go, collect_config.go | Minor maintainability |
| Per-package checksum skipped | collect_pkg.go | checksum field always "" |
| `renderJSON` in main.go not render_json.go | main.go | Structural, no functional impact |

## Rust implementation: known deviations from current spec

| Deviation | Location | Impact |
|-----------|----------|--------|
| `security_apparmor` and `dmi` serialise as null when absent | types.rs | JSON output has extra null fields |
| Signal handling uses OS default (not explicit handler) | main.rs | Acceptable per hints file; no functional impact for v0.9.0 |
| `.cargo/config.toml` crt-static removed; RUSTFLAGS in Makefile instead | Cargo/Makefile | Documented in report; correct approach |
| Per-package checksum skipped | collect_pkg.rs | checksum field always "" |
