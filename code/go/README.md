# sitar — System InformaTion At Runtime

sitar collects hardware, kernel, network, storage, security, and package
information from a running Linux system and renders it in one or more
structured output formats, including a machine-readable JSON format aligned
with the Machinery system description schema.

## Installation

### openSUSE / SUSE Linux Enterprise Server

```sh
zypper install sitar
```

### Debian / Ubuntu

```sh
apt install sitar
```

### Red Hat / CentOS / AlmaLinux / Rocky Linux

```sh
dnf install sitar
```

All packages are published via the [Open Build Service](https://build.opensuse.org).
curl-based installation scripts are not provided (supply chain security requirement).

## Usage

sitar must be run as **root** (uid=0).

```
sitar                          # print help and exit
sitar all                      # collect everything, all formats, auto outdir
sitar format=html outdir=DIR   # HTML report to DIR
sitar format=json outfile=FILE # machine-readable JSON to FILE
sitar check-consistency        # pre-generate consistency cache
sitar find-unpacked            # pre-generate unpacked files cache
sitar version                  # print version and exit
sitar help                     # print usage and exit
```

## Options

| Option | Description | Default |
|--------|-------------|---------|
| `format=FORMAT` | Output format: html, tex, sdocbook, json, markdown, all | all |
| `outfile=PATH` | Write output to PATH (single format only) | auto |
| `outdir=PATH` | Write output files to directory PATH | /tmp/sitar-HOSTNAME-TS |
| `limit=N` | Skip config files larger than N bytes (0=no limit) | 700000 |

## Commands

| Command | Description |
|---------|-------------|
| `all` | Collect all information + run check-consistency and find-unpacked |
| `check-consistency` | Check RPM config files for modifications; write cache |
| `find-unpacked` | Find /etc files not owned by any RPM; write cache |
| `help` | Print usage and exit 0 |
| `version` | Print version string and exit 0 |
| `debug` | Enable debug verbosity |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Fatal error (not root, write failure) |
| 2 | Invocation error (bad arguments, unknown format) |

## Output Formats

| Format | Extension | Description |
|--------|-----------|-------------|
| `html` | `.html` | HTML document with table of contents and inline CSS |
| `tex` | `.tex` | LaTeX document (scrartcl class); compile to PDF with pdflatex |
| `sdocbook` | `.sdocbook.xml` | Simplified DocBook XML |
| `json` | `.json` | Machine-readable; aligned with Machinery system description format |
| `markdown` | `.md` | GitHub Flavoured Markdown |

## JSON Schema

The JSON output follows the [Machinery](https://github.com/SUSE/machinery) system
description format (schema version 10) for all shared scopes. sitar-specific scopes
(cpu, kernel_params, kernel_config, devices, pci, storage, network,
security_apparmor, processes, dmi) are additional extensions.

Format version: always `1` in the `meta.format_version` field.

## Configuration File

`/etc/sysconfig/sitar` — optional; values overridden by command-line arguments.

```sh
SITAR_OPT_FORMAT="html"
SITAR_OPT_OUTDIR="/var/tmp/sitar"
SITAR_OPT_LIMIT="700000"
SITAR_OPT_ALLCONFIGFILES="Auto"
SITAR_OPT_ALLSUBDOMAIN="Auto"
SITAR_OPT_ALLSYSCONFIG="Auto"
SITAR_OPT_EXCLUDE="/etc/shadow"
```

## Extension Mechanism

Drop `*.include` files into `/var/lib/support/` to add configuration files
to the collection. Each file must contain:

```perl
@files = ( "/path/one", "/path/two" );
```

## Target Distributions

- RHEL / CentOS / AlmaLinux / Rocky: 8, 9, 10
- SUSE Linux Enterprise Server: 12 SP5, 15 SP1+, 16
- openSUSE Leap: 15.5+
- Ubuntu: 23.04+
- Debian: 12+

## License

GPL-2.0-or-later — see <https://spdx.org/licenses/GPL-2.0-or-later.html>

## Author

Matthias G. Eckermann <pcd@mailbox.org>
