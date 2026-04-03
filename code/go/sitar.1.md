% SITAR(1) sitar 0.9.0
% Matthias G. Eckermann
% April 2026

# NAME

sitar - System InformaTion At Runtime

# SYNOPSIS

**sitar** [*COMMAND*] [*OPTION=VALUE* ...]

# DESCRIPTION

**sitar** collects hardware, kernel, network, storage, security, and package
information from a running Linux system and renders it in one or more
structured output formats, including a machine-readable JSON format aligned
with the Machinery system description schema.

sitar must be run as root (uid=0). It reads from /proc, /sys, and various
system files, invoking external tools (lsblk, ip, rpm, dpkg, etc.) as needed.

# COMMANDS

**all**
: Collect all system information. Run check-consistency and find-unpacked
  cache generation. Produce all output formats.

**check-consistency**
: Check that RPM-declared configuration files have not been modified since
  installation. Write results to /var/lib/support/Configuration_Consistency.include.
  May take 5-20 minutes.

**find-unpacked**
: Find files below /etc that do not belong to any installed RPM. Write results
  to /var/lib/support/Find_Unpacked.include. May take 5-20 minutes.

**help**
: Print usage information and exit 0.

**version**
: Print version string and exit 0.

**debug**
: Enable debug verbosity (equivalent to setting SITAR_DEBUG=1).

# OPTIONS

**format=**_FORMAT_
: Output format. One of: **html**, **tex**, **sdocbook**, **json**, **markdown**, **all**.
  Default: all formats when using the **all** command, otherwise html.

**outfile=**_PATH_
: Write output to PATH. Only valid when a single format is requested.

**outdir=**_PATH_
: Write output files into directory PATH. Created if it does not exist.
  Default: /tmp/sitar-HOSTNAME-TIMESTAMP when multiple formats are requested.

**limit=**_N_
: Skip configuration files larger than N bytes. 0 means no limit.
  Default: 700000.

# EXIT CODES

**0**
: Success.

**1**
: Fatal error (not running as root, output file not writable).

**2**
: Invocation error (bad arguments, unknown format).

# FILES

**/etc/sysconfig/sitar**
: Configuration file. Recognised keys: SITAR_OPT_FORMAT, SITAR_OPT_OUTDIR,
  SITAR_OPT_OUTFILE, SITAR_OPT_LIMIT, SITAR_OPT_GCONF, SITAR_OPT_ALLCONFIGFILES,
  SITAR_OPT_ALLSUBDOMAIN, SITAR_OPT_ALLSYSCONFIG, SITAR_OPT_EXCLUDE,
  SITAR_OPT_LVMARCHIVE.

**/var/lib/support/**
: Cache directory for check-consistency and find-unpacked results.
  Drop *.include files here to extend the configuration file collection.

# EXAMPLES

Collect all information and write to /tmp/sitar-report/:

    sitar all outdir=/tmp/sitar-report

Collect in JSON format only:

    sitar format=json outfile=/tmp/snapshot.json

Generate HTML report:

    sitar format=html outdir=/var/tmp/sitar-html

Pre-generate consistency cache:

    sitar check-consistency

# INSTALLATION

sitar is distributed via the Open Build Service (build.opensuse.org).

For openSUSE/SLES:

    zypper install sitar

For Debian/Ubuntu:

    apt install sitar

For Red Hat/RHEL:

    dnf install sitar

# SEE ALSO

rpm(8), dpkg(1), lsblk(8), ip(8), systemctl(1)

# AUTHOR

Matthias G. Eckermann <pcd@mailbox.org>

# LICENSE

GPL-2.0-or-later <https://spdx.org/licenses/GPL-2.0-or-later.html>
