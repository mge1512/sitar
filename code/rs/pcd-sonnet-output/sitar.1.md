% SITAR(1) sitar 0.9.0 | User Commands
% Matthias G. Eckermann
% April 2026

# NAME

sitar - System InformaTion At Runtime

# SYNOPSIS

**sitar** [*COMMAND*] [*OPTIONS*]

# DESCRIPTION

**sitar** collects hardware, kernel, network, storage, security, and
package information from a running Linux system and renders it in one
or more structured output formats, including a machine-readable JSON
format aligned with the Machinery system description schema.

**sitar** must be run as root (uid=0).

# COMMANDS

**all**
:   Produce all output formats (html, tex, sdocbook, json, markdown) in
    one run. Also runs check-consistency and find-unpacked cache generation.

**check-consistency**
:   Check that RPM-declared configuration files have not been modified
    since installation. Writes cache to /var/lib/support/Configuration_Consistency.json.

**find-unpacked**
:   Find files below /etc that do not belong to any installed RPM.
    Writes cache to /var/lib/support/Find_Unpacked.json.

**help**
:   Print usage text and exit 0.

**version**
:   Print version string and exit 0.

**debug**
:   Enable debug output to stderr.

# OPTIONS

**format=**_FORMAT_
:   Output format. One of: html, tex, sdocbook, json, markdown, all.

**outfile=**_PATH_
:   Output file path. Used with single-format runs only.

**outdir=**_PATH_
:   Output directory. Default: /tmp/sitar-_hostname_-_datetime_.

**limit=**_N_
:   Maximum config file size in bytes for verbatim inclusion.
    Default: 700000. Set to 0 for no limit.

# EXIT CODES

**0**
:   Success.

**1**
:   Logical error or fatal error (e.g. not running as root).

**2**
:   Invocation error (bad arguments, unknown format value).

# CONFIGURATION FILE

**/etc/sysconfig/sitar**

Recognised keys (shell variable format, values may be double-quoted):

- **SITAR_OPT_FORMAT** — default output format
- **SITAR_OPT_OUTDIR** — default output directory
- **SITAR_OPT_OUTFILE** — default output file
- **SITAR_OPT_LIMIT** — default file size limit
- **SITAR_OPT_GCONF** — include GNOME config files (true/false)
- **SITAR_OPT_ALLCONFIGFILES** — On/Off/Auto
- **SITAR_OPT_ALLSUBDOMAIN** — On/Off/Auto
- **SITAR_OPT_ALLSYSCONFIG** — On/Off/Auto
- **SITAR_OPT_EXCLUDE** — colon-separated list of paths to exclude
- **SITAR_OPT_LVMARCHIVE** — include LVM archive (true/false)

Command-line options always override sysconfig values.

# EXAMPLES

Collect all information and produce all output formats:

    sitar all outdir=/tmp/myreport

Produce a single JSON output file:

    sitar format=json outfile=/tmp/snapshot.json

Produce HTML output in a specific directory:

    sitar format=html outdir=/var/tmp/sitar-out

Pre-generate the consistency cache:

    sitar check-consistency

# INSTALLATION

Install via OBS (openSUSE Build Service):

**openSUSE/SLES (zypper):**

    zypper install sitar

**Debian/Ubuntu (apt):**

    apt install sitar

**Red Hat/RHEL (dnf):**

    dnf install sitar

# FILES

**/usr/bin/sitar**
:   The sitar binary.

**/etc/sysconfig/sitar**
:   Configuration file.

**/var/lib/support/**
:   Cache directory for JSON cache files (Configuration_Consistency.json,
    Find_Unpacked.json, and drop-in extension files).

**/usr/share/man/man1/sitar.1**
:   This man page.

# SECURITY

sitar must run as root. It reads /etc/passwd but does NOT read /etc/shadow
by default (it is always in the default exclude list).

curl-based installation is not supported. Use the OBS package repository.

# SEE ALSO

**rpm**(8), **dpkg**(1), **lsblk**(8), **ip**(8), **dmidecode**(8)

# LICENSE

GPL-2.0-or-later — <https://spdx.org/licenses/GPL-2.0-or-later.html>
