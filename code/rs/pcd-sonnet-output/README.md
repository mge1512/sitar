# sitar — System InformaTion At Runtime

**Version:** 0.9.0  
**License:** GPL-2.0-or-later  
**Author:** Matthias G. Eckermann <pcd@mailbox.org>

## Overview

`sitar` collects hardware, kernel, network, storage, security, and package
information from a running Linux system and renders it in one or more
structured output formats, including a machine-readable JSON format aligned
with the Machinery system description schema.

`sitar` must be run as **root** (uid=0).

## Installation

### openSUSE / SLES (zypper)

```sh
zypper install sitar
```

### Debian / Ubuntu (apt)

```sh
apt install sitar
```

### Red Hat / RHEL / AlmaLinux / Rocky (dnf)

```sh
dnf install sitar
```

Packages are distributed via [build.opensuse.org (OBS)](https://build.opensuse.org/).
curl-based installation is not supported (supply chain security requirement).

## Usage

```
sitar [COMMAND] [OPTIONS]
```

### Commands

| Command             | Description |
|---------------------|-------------|
| `all`               | Produce all output formats; run consistency + find-unpacked cache |
| `check-consistency` | Pre-run cache: RPM config file consistency check |
| `find-unpacked`     | Pre-run cache: find files not owned by any RPM |
| `help`              | Print usage and exit 0 |
| `version`           | Print version and exit 0 |
| `debug`             | Enable debug output to stderr |

### Options (key=value)

| Option         | Description |
|----------------|-------------|
| `format=<fmt>` | html \| tex \| sdocbook \| json \| markdown \| all |
| `outfile=<p>`  | Output file path (single format only) |
| `outdir=<p>`   | Output directory |
| `limit=<n>`    | Max config file size in bytes (default: 700000) |

### Examples

```sh
# Collect all information, all formats
sitar all outdir=/tmp/myreport

# Single JSON output
sitar format=json outfile=/tmp/snapshot.json

# HTML output to a directory
sitar format=html outdir=/var/tmp/sitar-out

# Pre-generate consistency cache
sitar check-consistency
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0    | Success |
| 1    | Fatal error (e.g. not running as root) |
| 2    | Invocation error (bad arguments, unknown format) |

## Output Formats

| Format     | Extension         | Description |
|------------|-------------------|-------------|
| `html`     | `.html`           | HTML document with table of contents and inline CSS |
| `tex`      | `.tex`            | LaTeX document (scrartcl class) |
| `sdocbook` | `.sdocbook.xml`   | Simplified DocBook XML |
| `json`     | `.json`           | Machine-readable; Machinery schema compatible |
| `markdown` | `.md`             | GitHub Flavoured Markdown |

## Configuration File

`/etc/sysconfig/sitar`

```sh
#SITAR_OPT_FORMAT=""
#SITAR_OPT_OUTDIR=""
#SITAR_OPT_OUTFILE=""
#SITAR_OPT_LIMIT="700000"
#SITAR_OPT_GCONF="false"
#SITAR_OPT_ALLCONFIGFILES="Auto"
#SITAR_OPT_ALLSUBDOMAIN="Auto"
#SITAR_OPT_ALLSYSCONFIG="Auto"
#SITAR_OPT_EXCLUDE="/etc/shadow"
#SITAR_OPT_LVMARCHIVE="false"
```

Command-line options always override sysconfig values.

## Files

| Path | Description |
|------|-------------|
| `/usr/bin/sitar` | Binary |
| `/etc/sysconfig/sitar` | Configuration file |
| `/var/lib/support/` | Cache directory (JSON cache files) |
| `/usr/share/man/man1/sitar.1` | Man page |

## Security

- Must run as root
- `/etc/shadow` is **never read** by default (always in exclude list)
- No network calls at runtime
- No environment variable control (use key=value args or sysconfig)
- Install via OBS only; curl-based installation is forbidden

## JSON Output

The JSON output follows the [Machinery system description format](https://github.com/SUSE/machinery)
for all shared scopes. Sitar-specific scopes (cpu, kernel_params, kernel_config,
devices, pci, storage, network, security_apparmor, processes, dmi) are additions
beyond Machinery's base schema.

Format version: 1

## Building from Source

```sh
cargo build --release
# Static binary (Linux):
# Binary at: target/release/sitar
```

Requirements: Rust 1.70+, cargo, pandoc (for man page)

## License

GPL-2.0-or-later — <https://spdx.org/licenses/GPL-2.0-or-later.html>
