# sitar

System InformaTion At Runtime — collects hardware, kernel, network,
storage, security, and package information from a running Linux system
and renders it in one or more structured output formats, including a
machine-readable JSON format aligned with the Machinery system
description schema.

## META

Deployment:   cli-tool
Version:      0.9.0
Spec-Schema:  0.3.20
Author:       Matthias G. Eckermann <pcd@mailbox.org>
License:      GPL-2.0-or-later
Verification: none
Safety-Level: QM

---

## TYPES

```
OutputFormat := html | tex | sdocbook | json | markdown | all
// html:      HTML document with table of contents and inline CSS
// tex:       LaTeX document (scrartcl class); can be compiled to PDF
// sdocbook:  Simplified DocBook XML (article/section structure)
// json:      Machine-readable structured output; schema aligned with
//            the Machinery system description format (see JSON-SCHEMA
//            section below). Format version: 1 (sitar-specific).
// markdown:  Plain GitHub Flavoured Markdown
// all:       Produce all active OutputFormats in one run
//
// Historical formats removed from scope:
//   yast1 / yast2 — YaST selection files; AutoYaST supersedes this
//   sql           — never implemented

Verbosity := normal | verbose | debug
// normal:  progress messages to stderr only
// verbose: progress + section names to stderr
// debug:   verbose + timestamps on every collection step

DistributionFamily := rpm | deb | unknown
// rpm:  SUSE (openSUSE, SLES), Red Hat, UnitedLinux
// deb:  Debian, Ubuntu
// unknown: /etc/os-release present but family not determined

PackageVersioningBackend := rpm | dpkg | none
// rpm:  rpm(8) query interface
// dpkg: dpkg(1) / /var/lib/dpkg/status
// none: package information unavailable (fallback)

ConfigFileSource := hardcoded | consistency-cache | unpacked-cache | include-dir
// hardcoded:         built-in list of well-known config files
// consistency-cache: /var/lib/support/Configuration_Consistency.include
//                    produced by: sitar check-consistency
// unpacked-cache:    /var/lib/support/Find_Unpacked.include
//                    produced by: sitar find-unpacked
// include-dir:       any *.include files in /var/lib/support/
//                    (drop-in extension mechanism, PaDS-inherited)

FileSizeLimit := integer where value >= 0
// Maximum file size in bytes for config files included verbatim.
// 0 = no limit. Default: 700000.

SITAR_READFILE_LIMIT := 32767
// Maximum bytes read from a single /proc or /sys pseudo-file in one call.
// Used by: collect-kernel-params, collect-net-params, collect-security-apparmor.

// -----------------------------------------------------------------------
// JSON output schema types
// All JSON field names use underscore_style to match Machinery conventions.
// New sitar-specific scopes (cpu, kernel_params, storage, network,
// security_apparmor, processes, dmi) are additions beyond Machinery's
// base schema; they do not conflict with existing Machinery scopes.
// -----------------------------------------------------------------------

SitarManifest := {
  meta:                    SitarMeta,
  general_info:            GeneralInfoScope,         // sitar-specific
  environment:             EnvironmentScope,          // from Machinery
  os:                      OsScope,                  // from Machinery
  packages:                PackagesScope,             // from Machinery, extended
  patterns:                PatternsScope,             // from Machinery (rpm only)
  repositories:            RepositoriesScope,         // from Machinery
  services:                ServicesScope,             // from Machinery
  groups:                  GroupsScope,               // from Machinery
  users:                   UsersScope,                // from Machinery
  changed_config_files:    ChangedConfigFilesScope,   // from Machinery
  changed_managed_files:   ChangedManagedFilesScope,  // from Machinery
  unmanaged_files:         UnmanagedFilesScope,       // from Machinery
  // sitar-specific scopes (not in Machinery):
  cpu:                     CpuScope,
  kernel_params:           KernelParamsScope,         // /proc/sys/kernel/ only
  net_params:              NetParamsScope,            // /proc/sys/net/ only
  kernel_config:           KernelConfigScope,
  devices:                 DevicesScope,
  pci:                     PciScope,
  storage:                 StorageScope,
  network:                 NetworkScope,
  security_apparmor:       SecurityApparmorScope,
  processes:               ProcessesScope,
  dmi:                     DmiScope,
}

SitarMeta := {
  format_version:  integer   // always 1 for sitar JSON output
  sitar_version:   string    // e.g. "0.3.0"
  collected_at:    string    // UTC ISO 8601 timestamp
  hostname:        string    // FQDN if resolvable
  uname:           string    // full uname -a output
  // Per-scope timestamps follow Machinery convention:
  // Each scope key in meta maps to { modified: string, hostname: string }
}

// -----------------------------------------------------------------------
// Shared infrastructure for scope elements (mirrors Machinery pattern)
// -----------------------------------------------------------------------

ScopeWrapper<T> := {
  _attributes: object | null
  _elements:   []T
}

// -----------------------------------------------------------------------
// Machinery-compatible scopes (schema version 10, extended for sitar)
// -----------------------------------------------------------------------

EnvironmentScope := {
  locale:      string    // e.g. "en_US.UTF-8"
  system_type: "local" | "remote" | "docker"
}

OsScope := {
  name:         string | null   // e.g. "SUSE Linux Enterprise Server 15 SP6"
  version:      string | null   // e.g. "15-SP6"
  architecture: string | null   // e.g. "x86_64"
}

PackageRecord := {
  // RPM packages
  name:     string    // package name
  version:  string    // version string
  release:  string    // release string (RPM) or "" (dpkg)
  arch:     string    // e.g. "x86_64", "noarch"
  vendor:   string    // package vendor
  checksum: string    // MD5 checksum (hex digits; "" if not available)
  // sitar extensions (not in Machinery):
  size:     integer   // installed size in bytes (from %{SIZE} or dpkg Installed-Size)
  summary:  string    // one-line description
  distribution: string // RPM %{DISTRIBUTION}; "" for dpkg
  packager: string    // RPM %{PACKAGER}; "" for dpkg
}
PackagesScope := ScopeWrapper<PackageRecord> where
  _attributes.package_system := "rpm" | "dpkg"

PatternRecord := {
  name:    string
  version: string   // "" for tasksel patterns
  release: string   // "" for tasksel patterns
}
PatternsScope := ScopeWrapper<PatternRecord> where
  _attributes.patterns_system := "zypper" | "tasksel"

// Repository records follow Machinery schema exactly (zypp / yum / apt).
// No sitar extensions needed.
RepositoriesScope := ScopeWrapper<RepositoryRecord> where
  _attributes.repository_system := "zypp" | "yum" | "apt"
// zypp fields: alias, name, url, type, enabled, gpgcheck, autorefresh, priority
// yum fields:  alias, name, url[], type, enabled, gpgcheck, gpgkey[], mirrorlist
// apt fields:  url, type, distribution, components[]

ServiceRecord := {
  name:        string
  state:       string   // enabled | disabled | static | masked
  legacy_sysv: bool     // only when init_system = upstart
}
ServicesScope := ScopeWrapper<ServiceRecord> where
  _attributes.init_system := "systemd" | "sysvinit" | "upstart"

GroupRecord := {
  name:     string
  password: string
  gid:      integer | null
  users:    []string
}
GroupsScope := ScopeWrapper<GroupRecord>

UserRecord := {
  name:               string
  password:           string    // "x" or "!" (shadow-masked)
  uid:                integer | null
  gid:                integer | null
  comment:            string
  home:               string
  shell:              string
  // shadow fields (present when readable):
  encrypted_password: string
  last_changed_date:  integer   // days since epoch
  min_days:           integer
  max_days:           integer
  warn_days:          integer
  disable_days:       integer
  disabled_date:      integer
}
UsersScope := ScopeWrapper<UserRecord>

// ChangedConfigFilesScope and ChangedManagedFilesScope follow Machinery
// schema version 10 exactly. No sitar extensions.
// fields: name, package_name, package_version, status, changes[], mode,
//         user, group, type, target (links), error_message (errors)

UnmanagedFilesScope := ScopeWrapper<UnmanagedFileRecord> where
  _attributes.extracted   := bool
  _attributes.has_metadata := bool
// file record fields: name, type, user, group, size, mode,
//                     files (count), dirs (count) or file_objects (count)

// -----------------------------------------------------------------------
// sitar-specific scopes
// -----------------------------------------------------------------------

CpuRecord := {
  processor:   string   // processor index, e.g. "0"
  vendor_id:   string   // e.g. "GenuineIntel"
  model_name:  string   // e.g. "Intel(R) Core(TM) i7..."
  cpu_family:  string
  model:       string
  stepping:    string
  cpu_mhz:     string
  cache_size:  string   // e.g. "8192 KB"
}
CpuScope := ScopeWrapper<CpuRecord>
// _attributes: { architecture: string }  e.g. "x86_64", "alpha", "aarch64"

KernelParamRecord := {
  key:   string   // path relative to /proc/sys/kernel/, e.g. "hostname"
  value: string
}
KernelParamsScope := ScopeWrapper<KernelParamRecord>
// All readable files under /proc/sys/kernel/ only.

NetParamRecord := {
  key:   string   // path relative to /proc/sys/net/, e.g. "ipv4/ip_forward"
  value: string
}
NetParamsScope := ScopeWrapper<NetParamRecord>
// All readable files under /proc/sys/net/ only.
// Subtrees collected: 802, appletalk, ax25, bridge, core, decnet,
//   ethernet, ipv4, ipv6, irda, ipx, netfilter, rose, unix, x25.

KernelConfigRecord := {
  key:   string   // e.g. "CONFIG_SMP"
  value: string   // e.g. "y", "m", "n", or a quoted string
}
KernelConfigScope := ScopeWrapper<KernelConfigRecord>
// Sourced from /proc/config.gz (preferred) or /boot/config-$(uname -r).
// Comment lines stripped. Empty lines stripped.

DeviceRecord := {
  name:  string    // device name, e.g. "timer", "ata_piix"
  dma:   string    // DMA channel(s); "" if none
  irq:   string    // IRQ number(s); "" if none
  ports: string    // I/O port range(s); "" if none
}
DevicesScope := ScopeWrapper<DeviceRecord>
// Source: /proc/interrupts, /proc/dma, /proc/ioports

PciRecord := {
  pci:     string   // PCI address, e.g. "00:1f.2"
  device:  string   // device slot description
  class:   string   // PCI class, e.g. "SATA controller"
  vendor:  string
  svendor: string   // subsystem vendor
  sdevice: string   // subsystem device
  rev:     string   // revision
}
PciScope := ScopeWrapper<PciRecord>
// Source: lspci -vm (preferred), /proc/pci (legacy), lshal (fallback)

// Storage — combined scope covering partitions, mounts, RAID, LVM, IDE, SCSI
PartitionRecord := {
  // Primary fields from lsblk -J (always populated when lsblk available):
  device:       string    // e.g. "/dev/sda1"
  maj_min:      string    // major:minor device number, e.g. "8:1"
  type:         string    // "disk", "part", "lvm", "raid1", etc.
  size:         string    // human-readable size from lsblk, e.g. "50G"
  fstype:       string    // e.g. "ext4", "xfs", "btrfs", "swap"
  mountpoint:   string    // "" if not mounted
  uuid:         string    // filesystem UUID; "" if none
  label:        string    // filesystem label; "" if none
  ro:           string    // "0" or "1"
  // Secondary fields from fdisk -l (populated when lsblk unavailable):
  partition_type: string  // e.g. "Linux", "LVM-PV", "Extended"
  type_id:      string    // hex partition type id, e.g. "8e", "83"; "" if from lsblk
  begin_sector: string    // fdisk begin sector; "" when sourced from lsblk
  end_sector:   string    // fdisk end sector; "" when sourced from lsblk
  raw_size_kb:  integer   // raw partition size in KiB from fdisk; 0 when from lsblk
  mount_options: string   // mount options string; "" if not mounted
  // ext2/3/4 attributes from tune2fs -l (empty string if not ext* or unavailable):
  reserved_blocks: string
  block_size:      string
  inode_density:   string
  max_mount_count: string
  // df -PPk usage attributes (empty string if not mounted):
  df_blocks_kb:    integer
  df_used_kb:      integer
  df_avail_kb:     integer
  df_use_percent:  string   // e.g. "42%"
  // Source tracking:
  source:       string    // "lsblk" or "fdisk" — indicates which tool was primary
}

SoftwareRaidRecord := {
  device:     string    // e.g. "md0"
  level:      string    // e.g. "raid0", "raid1", "raid5"
  partitions: []string  // member devices
  blocks:     string
  chunk_size: string    // "" for raid1
  algorithm:  string    // "" for raid0/raid1
}

IdeRecord := {
  device:       string   // e.g. "/dev/hda"
  media:        string   // "disk", "cdrom", etc.
  model:        string
  driver:       string
  geometry_phys: string  // e.g. "cylinders/heads/sectors"
  geometry_log:  string
  capacity_blocks: string
}

ScsiRecord := {
  host:     string
  channel:  string
  id:       string
  lun:      string
  vendor:   string
  model:    string
  revision: string
  type:     string    // e.g. "Direct-Access"
  ansi_rev: string
}

BtrfsDeviceRecord := {
  devid: string   // device ID within the filesystem
  size:  string   // total device size, e.g. "50.00GiB"
  used:  string   // used space on device
  path:  string   // block device path, e.g. "/dev/sda2"
}

BtrfsSubvolumeRecord := {
  id:        string   // subvolume ID number
  gen:       string   // generation number
  top_level: string   // parent subvolume ID
  path:      string   // subvolume path relative to filesystem root
}

BtrfsFilesystemRecord := {
  label:          string   // filesystem label; "<unlabeled>" if none
  uuid:           string
  total_devices:  string
  bytes_used:     string
  mount_point:    string   // mountpoint this record was collected from
  devices:        []BtrfsDeviceRecord
  subvolumes:     []BtrfsSubvolumeRecord
  // space usage from "btrfs filesystem df <mountpoint>":
  data_total:     string
  data_used:      string
  metadata_total: string
  metadata_used:  string
  system_total:   string
  system_used:    string
}

StorageScope := {
  partitions:    ScopeWrapper<PartitionRecord>,
  software_raid: ScopeWrapper<SoftwareRaidRecord>,
  btrfs:         ScopeWrapper<BtrfsFilesystemRecord>,  // present only when btrfs mounted
  ide:           ScopeWrapper<IdeRecord>,
  scsi:          ScopeWrapper<ScsiRecord>,
  // RAID controller sub-scopes (present only when hardware detected):
  cciss:         ScopeWrapper<RawControllerRecord>,   // HP SmartArray (hpacucli)
  areca:         ScopeWrapper<RawControllerRecord>,   // Areca RAID
  dac960:        ScopeWrapper<RawControllerRecord>,   // Mylex DAC960
  gdth:          ScopeWrapper<RawControllerRecord>,   // ICP Vortex
  ips:           ScopeWrapper<RawControllerRecord>,   // IBM ServeRAID
  compaq_smart:  ScopeWrapper<RawControllerRecord>,   // COMPAQ Smart Array
  evms:          RawTextRecord,                       // EVMS if present
  multipath:     RawTextRecord,                       // dm-multipath if present
  fstab:         RawTextRecord,                       // /etc/fstab verbatim
  lvm_conf:      RawTextRecord,                       // /etc/lvm/lvm.conf verbatim
}

RawControllerRecord := {
  controller_id: string
  raw_output:    string   // verbatim tool output; not further structured
}

RawTextRecord := {
  path:    string
  content: string   // file/command output verbatim
}

NetworkInterfaceRecord := {
  // Primary fields from ip -j addr show:
  ifname:        string   // interface name, e.g. "eth0", "lo", "enp3s0"
  link_type:     string   // e.g. "ether", "loopback", "none"
  address:       string   // MAC address; "" for loopback
  flags:         []string // e.g. ["UP", "BROADCAST", "RUNNING", "MULTICAST"]
  mtu:           integer
  operstate:     string   // "UP", "DOWN", "UNKNOWN"
  // IPv4 address info (from addr_info array, type="inet"):
  ip:            string   // first IPv4 address; "" if unassigned
  prefixlen:     string   // prefix length, e.g. "24"
  broadcast:     string   // broadcast address; "" if none
  // IPv6 address info (from addr_info array, type="inet6"):
  ip6:           string   // first global IPv6 address; "" if unassigned
  ip6_prefixlen: string   // IPv6 prefix length
}

RouteRecord := {
  // Fields from ip -j route show:
  dst:      string   // destination prefix, e.g. "192.168.1.0/24" or "default"
  gateway:  string   // next-hop gateway; "" for directly connected
  dev:      string   // output interface, e.g. "eth0"
  protocol: string   // routing protocol, e.g. "kernel", "static", "dhcp"
  scope:    string   // e.g. "link", "global", "host"
  type:     string   // e.g. "unicast", "broadcast", "local"
  metric:   string   // route metric; "" if not set
  flags:    []string // e.g. ["onlink"]
}

PacketFilterRecord := {
  engine: string     // "iptables" | "ipchains" | "ipfwadm" | "none"
  table:  string     // e.g. "filter", "nat", "mangle"
  raw_output: string // verbatim iptables -v -L -n -t <table>
}

NetworkScope := {
  interfaces:    ScopeWrapper<NetworkInterfaceRecord>,
  routes:        ScopeWrapper<RouteRecord>,
  packet_filter: ScopeWrapper<PacketFilterRecord>,
}

ApparmorProfileRecord := {
  name:    string    // profile name, e.g. "/usr/bin/firefox"
  mode:    string    // "enforce" | "complain" | "unconfined"
}

ApparmorKernelRecord := {
  key:   string   // e.g. "enable", "audit"
  value: string
}

SecurityApparmorScope := {
  kernel_params:  ScopeWrapper<ApparmorKernelRecord>,
  // _attributes.apparmor_path: string — the active kernel interface path
  profiles:       ScopeWrapper<ApparmorProfileRecord>,
  config_files:   []string    // list of included config file paths
}

ProcessRecord := {
  pid:       string
  ppid:      string   // "" if not available from source
  comm:      string   // process name from /proc/<pid>/stat
  state:     string   // single-char state: R S D Z T W
  cmdline:   string   // full command line from /proc/<pid>/cmdline; "" for kernel threads
}
ProcessesScope := ScopeWrapper<ProcessRecord>
// Source: /proc/[0-9]* — one record per readable /proc/<pid>/stat

CrontabRecord := {
  minute:      string
  hour:        string
  day_of_month: string
  month:       string
  day_of_week: string
  user:        string    // "" for per-user crontabs (no user field)
  command:     string
  source:      string    // file path this entry was read from
}

DmiScope := {
  raw_output: string    // verbatim dmidecode output
}
// dmidecode produces multi-type output; structured parsing is out of scope.
// raw_output is preserved verbatim for human and downstream tool consumption.

GeneralInfoRecord := {
  key:   string   // field name, e.g. "hostname", "mem_total_kb"
  value: string   // string representation of the value
}
GeneralInfoScope := ScopeWrapper<GeneralInfoRecord>
// Keys collected (in order):
//   hostname, os_release, uname, collected_at,
//   mem_total_kb, cmdline, loadavg, uptime_min, idletime_min
```

---

## BEHAVIOR: prepare-config
Constraint: required

Load configuration from the sysconfig file and apply command-line
overrides. Command-line values always win over sysconfig values.

INPUTS:
```
config_file: path    // default: /etc/sysconfig/sitar
argv:        []string
```

OUTPUTS:
```
config: {
  format:            OutputFormat | ""  // "" means all; see render BEHAVIOR
  outfile:           string             // "" means use auto-derived name in render
  outdir:            string             // "" means use auto-derived dir in render
  file_size_limit:   FileSizeLimit      // default 700000
  check_consistency: bool
  find_unpacked:     bool
  all:               bool
  allconfigfiles:    "On" | "Off" | "Auto"   // default "Auto"
  allsubdomain:      "On" | "Off" | "Auto"   // default "Auto"
  allsysconfig:      "On" | "Off" | "Auto"   // default "Auto"
  gconf:             bool                    // default false
  lvmarchive:        bool                    // default false
  exclude:           []string                // default ["/etc/shadow"]
  verbosity:         Verbosity               // default normal
  debug:             bool                    // default false
}
```

PRECONDITIONS:
- sysconfig file need not exist; absence is silently ignored
- argv contains only recognised options (validated in STEPS)

STEPS:
1. If config_file is readable: parse it line by line.
   For each non-comment (`#`), non-empty line: split on first `=`,
   strip outer double-quotes from value, apply to matching config field.
   Recognised sysconfig keys:
     SITAR_OPT_FORMAT, SITAR_OPT_OUTDIR, SITAR_OPT_OUTFILE,
     SITAR_OPT_LIMIT, SITAR_OPT_GCONF, SITAR_OPT_ALLCONFIGFILES,
     SITAR_OPT_ALLSUBDOMAIN, SITAR_OPT_ALLSYSCONFIG,
     SITAR_OPT_EXCLUDE, SITAR_OPT_LVMARCHIVE.
   On unrecognised key: ignore silently.
2. Parse argv using key=value pairs and bare-word commands.
   If argv is empty (no arguments at all): print help text to stdout
   and exit 0. Do NOT proceed to collection. A sitar run can be
   lengthy; explicit invocation is always required.
   Recognised bare-word commands (no value):
     all                → config.all = true; config.format = ""
     check-consistency  → config.check_consistency = true
     find-unpacked      → config.find_unpacked = true
     help               → print help text to stdout and exit 0
     version            → print version string to stdout and exit 0
     debug              → config.verbosity = debug
   Recognised key=value options:
     format=<f>         → config.format = f
     outfile=<p>        → config.outfile = p
     outdir=<p>         → config.outdir = p
     limit=<n>          → config.file_size_limit = n
   On unrecognised token: print error to stderr, exit 2.
3. If format is set and unrecognised: print error to stderr, exit 2.
4. If config.all is true: set check_consistency = true, find_unpacked = true.
5. Normalise: config.format = "" and config.format = "all" are equivalent
   and both mean "produce all active formats". The string stored internally
   may be either; render MUST treat both identically.
6. Return merged config.

POSTCONDITIONS:
- config.format is one of the valid OutputFormat values or ""
- config.exclude is non-empty, defaulting to ["/etc/shadow"]
- config.file_size_limit >= 0

ERRORS:
- unrecognised CLI option → stderr + exit 2
- unrecognised format value → stderr + exit 2

---

## BEHAVIOR: detect-distribution
Constraint: required

Determine the Linux distribution family and select the appropriate
package versioning backend.

INPUTS:
```
filesystem: Filesystem
```

OUTPUTS:
```
family:        DistributionFamily
release:       string                    // human-readable release string
backend:       PackageVersioningBackend
rpm_cmd:       string                    // path to rpm binary; "" if none
dpkg_status:   string                    // path to dpkg status file; "" if none
```

PRECONDITIONS:
- Must run before any collection modules that branch on distribution

STEPS:
1. Check /etc/debian_version → family=deb, backend=dpkg,
   dpkg_status="/var/lib/dpkg/status", read first line as release.
2. Else check /etc/redhat-release → family=rpm, backend=rpm,
   rpm_cmd="/bin/rpm", read first line as release.
3. Else check /etc/UnitedLinux-release AND /etc/SuSE-release →
   family=rpm, backend=rpm, rpm_cmd="/bin/rpm",
   release = first line of UnitedLinux-release + ", " + first line of SuSE-release.
4. Else check /etc/UnitedLinux-release → family=rpm, backend=rpm,
   rpm_cmd="/bin/rpm", read first line.
5. Else check /etc/SLOX-release → family=rpm, backend=rpm,
   rpm_cmd="/bin/rpm", read first line.
6. Else check /etc/SuSE-release → family=rpm, backend=rpm,
   rpm_cmd="/bin/rpm", read first line.
7. Else check /etc/os-release → family=rpm, backend=rpm,
   rpm_cmd="/usr/bin/rpm", read PRETTY_NAME value as release.
8. Else: family=unknown, backend=none.
   MECHANISM: emit warning to stderr "distribution not supported".
9. Return family, release, backend, rpm_cmd, dpkg_status.

POSTCONDITIONS:
- Exactly one branch resolves; STEPS order is the precedence order
- release is a non-empty string for all known families
- backend is consistent with family

ERRORS:
- unknown family emits a warning but does not abort

---

## BEHAVIOR: collect
Constraint: required

Orchestrate all enabled collection modules in canonical order and
produce a SitarManifest (the intermediate representation used by all
renderers, including the JSON renderer).

INPUTS:
```
config:  Config
family:  DistributionFamily
backend: PackageVersioningBackend
```

OUTPUTS:
```
manifest: SitarManifest
```

PRECONDITIONS:
- Must run as root (uid=0); if not: print error to stderr and exit 1.
  MECHANISM: check effective uid before any collection step.
- /proc must be mounted and readable.

STEPS:
1. Verify uid=0; on failure → stderr "Please run sitar as user root." + exit 1.
2. If config.find_unpacked: run find-unpacked BEHAVIOR; write cache.
3. If config.check_consistency: run check-consistency BEHAVIOR; write cache.
4. Initialise manifest.meta with sitar_version, format_version=1,
   collected_at (UTC ISO 8601), hostname (from `hostname -f`),
   uname (from `uname -a`).
5. Run collect-general-info; store in manifest.general_info.
6. Run collect-environment; store in manifest.environment.
7. Run collect-os; store in manifest.os.
8. Run collect-cpu; store in manifest.cpu.
9. Run collect-kernel-params; store in manifest.kernel_params.
10. Run collect-net-params; store in manifest.net_params.
11. Run collect-devices; store in manifest.devices.
12. Run collect-pci; store in manifest.pci.
13. Run collect-storage (includes software-raid, mount/partition table,
    IDE, SCSI, controller sub-scopes, EVMS if present, multipath if present,
    fstab, lvm.conf); store in manifest.storage.
14. Run collect-btrfs; store in manifest.storage.btrfs
    (skip if no btrfs mountpoints found).
15. Run collect-network-interfaces; store in manifest.network.interfaces.
16. Run collect-network-routing; store in manifest.network.routes.
17. Run collect-network-firewall; store in manifest.network.packet_filter.
18. Run collect-security-apparmor (skip if no AppArmor kernel path found);
    store in manifest.security_apparmor.
19. Run collect-processes; store in manifest.processes.
20. Run collect-dmi (skip if dmidecode not executable);
    store in manifest.dmi.
21. Distribution-specific steps:
    a. rpm families (suse, sles, unitedlinux, redhat):
       - collect-chkconfig (if chkconfig executable)
       - collect-config-files (config.allconfigfiles, .allsubdomain,
         .allsysconfig, .include files in /var/lib/support/)
       - collect-installed-rpm; store in manifest.packages,
         manifest.patterns (zypp only)
       - collect-repositories; store in manifest.repositories
       - collect-services; store in manifest.services
       - collect-groups; store in manifest.groups
       - collect-users; store in manifest.users
       - collect-changed-config-files;
         store in manifest.changed_config_files
       - collect-changed-managed-files;
         store in manifest.changed_managed_files
       - collect-kernel-config; store in manifest.kernel_config
    b. deb family:
       - collect-config-files
       - collect-installed-deb; store in manifest.packages
       - collect-groups; store in manifest.groups
       - collect-users; store in manifest.users
       - collect-kernel-config; store in manifest.kernel_config
22. Set per-scope meta timestamps (UTC ISO 8601) for each populated scope.
23. Return manifest.

POSTCONDITIONS:
- manifest.meta.collected_at is a UTC ISO 8601 timestamp
- manifest.meta.hostname is the real FQDN from `hostname -f`, not a
  hardcoded string; MUST NOT be "localhost" unless that is the actual
  system hostname
- manifest.meta.uname is the real output of `uname -a`, not ""
- manifest.os.name is the real OS release string from detect-distribution,
  not a hardcoded string
- manifest.environment.locale reflects the actual system locale
- Every sub-scope for which the data source is present on the system
  MUST be populated (non-null, non-empty _elements); see INVARIANTS
- Each sub-scope that was skipped (missing data source or tool) is absent
  from the manifest or null; its meta timestamp is omitted
- No module failure aborts the run; failures are logged to stderr

ERRORS:
- uid != 0 → exit 1 (before any collection)
- individual module failure → log to stderr, continue with next module

---

## BEHAVIOR: collect-general-info
Constraint: required

Collect basic system identity and runtime metrics.

INPUTS: filesystem, command_runner

OUTPUTS: GeneralInfoScope

STEPS:
1. Read hostname from `hostname -f`. TrimSpace result.
2. Read OS release string from detect-distribution result.
3. Read uname from `uname -a`. TrimSpace result.
4. Record local time string (human-readable, localtime()).
5. Read /proc/meminfo; extract MemTotal line; parse integer KiB value.
6. Read /proc/cmdline verbatim; strip trailing newline.
7. Read /proc/loadavg verbatim; strip trailing newline.
8. Read /proc/uptime; split on space: uptime_sec, idletime_sec.
   Compute uptime_min = floor(uptime_sec / 60),
           idletime_min = floor(idletime_sec / 60).
9. Emit one GeneralInfoRecord per field, in this order:
     { key: "hostname",     value: <hostname> }
     { key: "os_release",   value: <release string> }
     { key: "uname",        value: <uname -a output> }
     { key: "collected_at", value: <local time string> }
     { key: "mem_total_kb", value: <integer as string> }
     { key: "cmdline",      value: <cmdline string> }
     { key: "loadavg",      value: <loadavg string> }
     { key: "uptime_min",   value: <integer as string> }
     { key: "idletime_min", value: <integer as string> }
10. Return GeneralInfoScope with _elements = above 9 records.

POSTCONDITIONS:
- result._elements has exactly 9 records
- record with key "hostname" MUST NOT have value "" or "localhost"
  unless that is the actual system hostname
- record with key "uname" MUST NOT have value ""

---

## BEHAVIOR: collect-environment
Constraint: required

Collect system environment metadata for the manifest environment scope,
following the Machinery schema exactly.

INPUTS: filesystem

OUTPUTS: EnvironmentScope

STEPS:
1. Detect locale: read LANG or LC_ALL from /etc/locale.conf,
   /etc/default/locale, or /etc/sysconfig/language (SUSE).
   First readable file wins. Parse the value of LANG or LC_ALL.
   If none found: use "C".
2. Detect system_type:
   If /.dockerenv exists or /proc/1/cgroup contains "docker": "docker".
   Else if /proc/1/environ contains container indicators: "remote".
   Else: "local".
3. Return EnvironmentScope{locale, system_type}.

POSTCONDITIONS:
- locale is non-empty
- system_type is one of "local", "remote", "docker"

ERRORS:
- All locale sources unreadable → locale = "C", log to stderr

---

## BEHAVIOR: collect-os
Constraint: required

Collect OS identification for the manifest os scope,
following the Machinery schema exactly.

INPUTS: filesystem, detect-distribution result

OUTPUTS: OsScope

STEPS:
1. Read /etc/os-release; parse NAME, VERSION, and VERSION_ID fields.
   Strip outer quotes from values.
2. If /etc/os-release absent: use the release string from
   detect-distribution as name; version = null.
3. Read architecture from `uname -m`. TrimSpace result.
4. Return OsScope{name, version, architecture}.
   name and version may be null if /etc/os-release is absent and
   detect-distribution returned an empty release string.

POSTCONDITIONS:
- architecture is non-empty on any supported Linux system

ERRORS:
- /etc/os-release unreadable → name from detect-distribution, log to stderr

---

## BEHAVIOR: collect-chkconfig
Constraint: supported

Collect service startup configuration from chkconfig (sysvinit systems)
or systemctl (systemd systems). Results populate manifest.services.

INPUTS: command_runner

OUTPUTS: ServicesScope

STEPS:
1. Detect init system:
   a. If /run/systemd/private exists or `systemctl` is executable
      and /run/systemd exists: init_system = "systemd".
   b. Else if `chkconfig` is executable: init_system = "sysvinit".
   c. Else: return null (cannot determine service configuration).
2. If systemd:
   Run `systemctl list-unit-files --type=service --no-legend`.
   For each output line: split on whitespace: name, state.
   Valid states: enabled, disabled, static, masked, generated,
   transient, indirect. Emit ServiceRecord per line.
   Set _attributes.init_system = "systemd".
3. If sysvinit:
   Run `chkconfig --list`.
   For each output line: parse service name and runlevel states.
   A service is "enabled" if it is on in runlevel 3 or 5.
   Emit ServiceRecord{name, state="enabled"|"disabled"}.
   Set _attributes.init_system = "sysvinit".
4. Return ServicesScope.

POSTCONDITIONS:
- services._elements is non-empty on any system with running services

ERRORS:
- systemctl/chkconfig unavailable → return null, log to stderr

---

## BEHAVIOR: collect-cpu
Constraint: required

Collect CPU information from /proc/cpuinfo.

INPUTS: filesystem

OUTPUTS: CpuScope with _attributes.architecture = uname -m

STEPS:
1. Detect architecture from `uname -m`.
2. Open /proc/cpuinfo.
3. If architecture = "alpha":
   a. Parse each "key : value" line.
   b. Emit one CpuRecord per "cpus detected" line encountered;
      include all subsequent key/value pairs until next processor block.
4. Else (x86 and others):
   a. On line matching `^processor`: record processor index; begin new CpuRecord.
   b. Collect lines matching:
        cpu MHz, model name, vendor_id, cache size, stepping,
        cpu family, model.
      Store as fields of the current CpuRecord.
   c. On next `^processor` or EOF: emit completed CpuRecord.
5. Return CpuScope.

POSTCONDITIONS:
- At least one CpuRecord present on any supported Linux system
- cpu._elements MUST NOT be empty when /proc/cpuinfo is readable

ERRORS:
- /proc/cpuinfo unreadable → empty _elements, log to stderr

---

## BEHAVIOR: collect-kernel-params
Constraint: required

Collect all readable sysctl values under /proc/sys/kernel/.

INPUTS: filesystem, command_runner

OUTPUTS: KernelParamsScope

STEPS:
1. Walk /proc/sys/kernel/ recursively; collect ALL regular files,
   not just /proc/sys/kernel/hostname. Every readable file under
   this directory tree is a kernel parameter to be collected.
2. For each file: read content up to SITAR_READFILE_LIMIT (32767) bytes.
   Strip trailing newline. Skip if content is empty.
3. Record KernelParamRecord with key = path relative to
   /proc/sys/kernel/, value = file content.
4. Return KernelParamsScope sorted by key.

POSTCONDITIONS:
- kernel_params._elements MUST NOT be empty when /proc/sys/kernel/
  is readable (a minimal Linux system has at least 20 readable entries)

ERRORS:
- Unreadable file -> skip silently

---

## BEHAVIOR: collect-net-params
Constraint: required

Collect all readable sysctl values under /proc/sys/net/.

INPUTS: filesystem

OUTPUTS: NetParamsScope

STEPS:
1. For each subtree in /proc/sys/net/ from this list:
     802, appletalk, ax25, bridge, core, decnet, ethernet,
     ipv4, ipv6, irda, ipx, netfilter, rose, unix, x25
   If the subdirectory exists: walk it recursively; collect all regular files.
2. For each file: read content up to SITAR_READFILE_LIMIT (32767) bytes.
   Strip trailing newline. Skip if content is empty.
3. Record NetParamRecord with key = path relative to /proc/sys/net/,
   value = file content.
4. Return NetParamsScope sorted by key.

POSTCONDITIONS:
- net_params._elements MUST NOT be empty on any running Linux system
  (ipv4 subtree always has readable entries)

ERRORS:
- Unreadable file -> skip silently
- Subtree absent -> skip that subtree silently

---

## BEHAVIOR: collect-devices
Constraint: required

Collect device DMA/IRQ/port assignments from /proc.

INPUTS: filesystem

OUTPUTS: DevicesScope

STEPS:
1. Open /proc/interrupts; skip header lines (starting with uppercase).
   For each data line: extract IRQ number and device name.
   If line contains "PIC": split on whitespace; else split on " [ +] ".
   Device name is the last field.
2. Open /proc/dma; for each line: split on ": "; extract device name
   (first word before "(").
3. Open /proc/ioports; for each line: split on " : "; extract device name
   (first word before "(").
4. Merge: for each unique device name, combine all IRQ, DMA, port values.
5. Return DevicesScope sorted by device name (case-insensitive).

POSTCONDITIONS:
- devices._elements MUST NOT be empty when /proc/interrupts is readable

ERRORS:
- Unreadable /proc/interrupts, /proc/dma, or /proc/ioports ->
  skip that source; partial results are acceptable

---

## BEHAVIOR: collect-pci
Constraint: required

Collect PCI device information.

INPUTS: command_runner, filesystem

OUTPUTS: PciScope

STEPS:
1. If `lspci` is executable:
   Run `lspci -vm`; parse blank-line-separated stanzas.
   First "Device:" key in each stanza begins a new PciRecord.
   Store all key/value pairs (Device, Class, Vendor, SVendor, SDevice,
   Rev) into the record.
   On blank line: emit PciRecord, begin new record.
   Return PciScope.
2. Else if /proc/bus/pci or /proc/pci is readable:
   Parse legacy /proc/pci format.
   Lines ending with ":": extract Bus, Device, Function numbers.
   Other "type: vendor" lines: emit one PciRecord per entry.
   Return PciScope.
3. Else if `lshal` is executable:
   Run `lshal --long`; collect raw output.
   Emit one RawControllerRecord with raw_output = full lshal output.
   Return PciScope with raw_output (degraded mode).
4. If no source available: return empty PciScope.

ERRORS:
- lspci parse error → skip malformed stanza, log to stderr

---

## BEHAVIOR: collect-storage
Constraint: required

Collect partition table, mount state, filesystem attributes, software
RAID, IDE, SCSI, and RAID controller information.

INPUTS: filesystem, command_runner

OUTPUTS: StorageScope

STEPS:
1. Collect block device and mount information (primary: lsblk -J):
   a. If `lsblk` is executable: run
        lsblk -J -o NAME,MAJ:MIN,TYPE,SIZE,FSTYPE,MOUNTPOINT,UUID,LABEL,RO
      Parse JSON output; for each item where type is "part", "disk", or "lvm":
      emit PartitionRecord with source="lsblk", populating:
        device = "/dev/" + NAME
        maj_min, type, size, fstype, mountpoint, uuid, label, ro
        begin_sector="", end_sector="", raw_size_kb=0
        type_id="", partition_type=""
   b. Else (lsblk unavailable): fall back to legacy:
      Run `mount`; for each line matching `^/dev`: parse device,
      mountpoint, fstype, options.
      Run `fdisk -l`; for each line matching `^/dev`: strip "*" (boot flag);
      split on whitespace (max 6 fields): device, begin_sector, end_sector,
      raw_size_kb, type_id, type_name.
      If type_id == "8e": type_name = "LVM-PV".
      If type_id == "fe": type_name = "old LVM".
      Emit PartitionRecord with source="fdisk".

2. Collect mount options:
   If `findmnt` is executable: run `findmnt -J`; parse JSON; for each
   filesystem entry matching a device in step 1: set mount_options from
   the "options" field.
   Else: run `mount`; parse "(options)" field for each matching device.

3. Collect df usage statistics:
   Run `df -PPk`; for each line matching `^/dev`:
   split: device, df_blocks_kb, df_used_kb, df_avail_kb, df_use_percent.
   Merge into matching PartitionRecord.

4. Collect ext2/3/4 attributes for each partition where fstype is
   ext2, ext3, or ext4:
   Run `tune2fs -l <device>` (suppress stderr). Extract:
     "Reserved block count:" -> reserved_blocks
     "Block size:"           -> block_size
     "Inode count:"          -> inode_count (internal)
     "Block count:"          -> block_count (internal)
     "Maximum mount count:"  -> max_mount_count
   Compute inode_density:
     If inode_count > 0 AND block_count > 0 AND block_size > 0:
       inode_density = 2^round(log2(block_count / inode_count)) * block_size
       where round() returns nearest integer, log2() is base-2 logarithm
     Else: inode_density = ""
   Non-ext filesystems: all tune2fs fields remain "".

5. Collect software RAID state (if /proc/mdstat readable):
   Parse /proc/mdstat line by line.
   Line matching `^md\d+\s:\sactive\s(raid\d)\s(.*)$`:
   Capture device, level, partition list.
   Next line: parse blocks, chunk_size, algorithm (level-specific).
   Emit SoftwareRaidRecord.

6. Collect IDE devices (if any /proc/ide/hd* readable):
   For each /dev/hda through /dev/hdi present:
   Read media, model, driver from /proc/ide/hd${x}/ pseudo-files.
   If media = "disk": read geometry from /proc/ide/hd${x}/geometry.
   Emit IdeRecord.

7. Collect SCSI devices (if /proc/scsi/scsi readable):
   Parse host/channel/id/lun, vendor/model/rev, type/ansi_rev triplets.
   Emit ScsiRecord per complete triplet.

8. Collect RAID controller data (each sub-step independent):
   a. CCISS/HP SmartArray: if hpacucli or cpqacucli executable.
   b. Areca: if /usr/lib/snmp/cli64 or cli32 executable.
   c. DAC960: if /proc/rd readable.
   d. GDTH: if /proc/scsi/gdth readable.
   e. IPS: if /proc/scsi/ips readable.
   f. Compaq Smart Array: if /proc/array readable.
   All store raw_output per controller.

9. EVMS: if evms_gather_info executable: run, capture stdout, store.
   Remove gather_info.qry if created.

10. Multipath: if multipath executable: include /etc/multipath.conf
    verbatim; run `multipath -ll`; store both.

11. Include /etc/fstab verbatim as StorageScope.fstab.
12. Include /etc/lvm/lvm.conf verbatim as StorageScope.lvm_conf.
    Include /etc/lvm/.cache if readable.
    Include all files in /etc/lvm/backup/ if directory exists.

POSTCONDITIONS:
- storage.partitions._elements MUST NOT be empty on any system with
  /dev block devices and a readable mount table
- PartitionRecord.source is always "lsblk" or "fdisk"
- begin_sector and end_sector are "" when source="lsblk"
- Each RAID controller sub-scope is absent if hardware not detected
- tune2fs fields are empty strings for non-ext filesystems

ERRORS:
- lsblk/fdisk/mount/tune2fs invocation failure -> log to stderr, continue
- Missing /proc/ide, /proc/scsi -> IDE/SCSI sub-scopes remain empty

---

## BEHAVIOR: collect-btrfs
Constraint: supported

Collect btrfs filesystem details: subvolumes, device members, and space
usage. Only runs when at least one btrfs filesystem is mounted.
Important for SLES 12+ where btrfs is the default root filesystem.

INPUTS: filesystem, command_runner

OUTPUTS: StorageScope.btrfs (ScopeWrapper<BtrfsFilesystemRecord>)

PRECONDITIONS:
- `btrfs` executable must be present (btrfs-progs package)
- At least one btrfs mountpoint must exist in the partition records

STEPS:
1. Identify all btrfs mountpoints from StorageScope.partitions._elements
   where fstype = "btrfs". Deduplicate by filesystem UUID where possible.
2. For each unique btrfs mountpoint:
   a. Run `btrfs filesystem show <mountpoint>`.
      Parse output:
        Line matching `Label:`: extract label (strip quotes; use
          "<unlabeled>" if "none") and uuid.
        Line matching `Total devices`: extract total_devices.
        Line matching `FS bytes used`: extract bytes_used.
        Lines matching `devid`: extract devid, size, used, path.
          Emit one BtrfsDeviceRecord per devid line.
   b. Run `btrfs subvolume list <mountpoint>`.
      Parse output; each line format:
        ID <id> gen <gen> top level <top_level> path <path>
      Emit one BtrfsSubvolumeRecord per line.
   c. Run `btrfs filesystem df <mountpoint>`.
      Parse output lines of format `<type>, <profile>: total=<n>, used=<n>`:
        Data    -> data_total, data_used
        Metadata -> metadata_total, metadata_used
        System  -> system_total, system_used
   d. Emit one BtrfsFilesystemRecord with mount_point, label, uuid,
      total_devices, bytes_used, devices[], subvolumes[],
      and all df space fields.
3. Return ScopeWrapper with _elements = all BtrfsFilesystemRecords.

POSTCONDITIONS:
- If btrfs mountpoints exist and btrfs is executable:
  storage.btrfs._elements MUST NOT be empty
- Each BtrfsFilesystemRecord has at least one BtrfsDeviceRecord

ERRORS:
- btrfs not executable -> skip module entirely, return null
- btrfs subcommand fails for one mountpoint -> log to stderr,
  emit partial record with available fields, continue with next

---

## BEHAVIOR: collect-network-interfaces
Constraint: required

Collect network interface configuration.

INPUTS: command_runner

OUTPUTS: NetworkScope.interfaces (ScopeWrapper<NetworkInterfaceRecord>)

STEPS:
1. Run `ip -j addr show`.
2. Parse JSON array; each top-level object is one interface.
   Map fields directly to NetworkInterfaceRecord:
     ifname    → ifname
     link_type → link_type
     address   → address (MAC; absent for loopback → "")
     flags     → flags (JSON array of strings)
     mtu       → mtu (integer)
     operstate → operstate
   From addr_info array: find first entry with family="inet":
     local      → ip
     prefixlen  → prefixlen (as string)
     broadcast  → broadcast (absent → "")
   From addr_info array: find first entry with family="inet6"
   and scope != "link":
     local      → ip6
     prefixlen  → ip6_prefixlen (as string)
3. Emit one NetworkInterfaceRecord per interface object.
4. Return ScopeWrapper with _attributes.command = "ip".

POSTCONDITIONS:
- network.interfaces._elements MUST NOT be empty on any running Linux
  system (at minimum loopback lo is always present)

ERRORS:
- ip not executable or fails → empty _elements, log to stderr

---

## BEHAVIOR: collect-network-routing
Constraint: required

Collect IP routing table.

INPUTS: command_runner

OUTPUTS: NetworkScope.routes (ScopeWrapper<RouteRecord>)

STEPS:
1. Run `ip -j route show`.
2. Parse JSON array; each object is one route entry.
   Map fields directly to RouteRecord:
     dst      → dst ("default" if absent or equals "0.0.0.0/0")
     gateway  → gateway (field name "gateway"; "" if absent)
     dev      → dev
     protocol → protocol (e.g. "kernel", "static", "dhcp")
     scope    → scope (e.g. "link", "global", "host"; "" if absent)
     type     → type (e.g. "unicast"; "" if absent)
     metric   → metric (as string; "" if absent)
     flags    → flags (JSON array; empty array if absent)
3. Emit one RouteRecord per array element.

POSTCONDITIONS:
- network.routes._elements MUST NOT be empty on any running Linux
  system (at minimum a loopback or default route is always present)

ERRORS:
- ip not executable or fails → empty _elements, log to stderr

---

## BEHAVIOR: collect-network-firewall
Constraint: required

Collect packet filter rules; detect engine automatically.

INPUTS: filesystem, command_runner

OUTPUTS: NetworkScope.packet_filter (ScopeWrapper<PacketFilterRecord>)

STEPS:
1. If /proc/net/ip_input readable: engine = "ipfwadm".
   Emit one PacketFilterRecord{engine="ipfwadm",
   raw_output="ipfwadm is not supported."}.
2. Else if /proc/net/ip_fwnames readable: engine = "ipchains".
   Parse /proc/net/ip_fwchains and /proc/net/ip_fwnames.
   Emit one PacketFilterRecord per chain.
3. Else if /proc/net/ip_tables_names readable AND iptables executable:
   engine = "iptables".
   For each table name in /proc/net/ip_tables_names:
   Run `iptables -v -L -n -t <table>`.
   Emit one PacketFilterRecord{engine="iptables", table=<name>,
   raw_output=<output>}.
4. Else: emit one PacketFilterRecord{engine="none",
   raw_output="No packet filter installed."}.

ERRORS:
- iptables invocation failure → log to stderr, continue

---

## BEHAVIOR: collect-security-apparmor
Constraint: supported

Collect AppArmor kernel state and profile list.

INPUTS: filesystem, command_runner, config

OUTPUTS: SecurityApparmorScope

STEPS:
1. Locate AppArmor kernel interface:
   Search paths (in order): /proc/sys/kernel/security/apparmor,
   /sys/kernel/security/apparmor.
   If neither found: skip this module entirely (return null).
2. Walk the found kernel path; collect all readable files.
   For each file: read content (up to SITAR_READFILE_LIMIT bytes).
   Skip empty content.
   Record key = path relative to kernel interface base, value = content.
   Special case: if key = "profiles":
   Parse value: split on ")" to extract per-profile name(enforce|complain) pairs.
   Emit ApparmorProfileRecord per pair.
   Other keys → emit ApparmorKernelRecord.
3. If AppArmor config log readable:
   Include its path in config_files list.
4. Include all files in /etc/apparmor/ and /etc/apparmor.d/
   (or /etc/subdomain/ and /etc/subdomain.d/) as config_files.
5. If config.allsubdomain = "On", OR
   config.allsubdomain = "Auto" AND neither consistency nor unpacked
   cache file exists:
   Scan all files in the profile directories (above); add paths to
   config_files list.
6. Return SecurityApparmorScope.

ERRORS:
- Unreadable kernel path file → skip that file

---

## BEHAVIOR: collect-processes
Constraint: required

Collect process list from /proc.

INPUTS: filesystem

OUTPUTS: ProcessesScope

STEPS:
1. Walk /proc/; collect all subdirectory names matching `^[0-9]+$`.
2. For each PID directory:
   a. Read /proc/<pid>/stat; parse:
      field 1: pid, field 2: comm (strip parentheses), field 3: state,
      field 4: ppid.
   b. Read /proc/<pid>/cmdline; replace null bytes with spaces;
      strip trailing whitespace. If empty: process is a kernel thread.
3. Emit one ProcessRecord per readable PID.
4. Return ProcessesScope sorted by pid (numeric).

POSTCONDITIONS:
- processes._elements MUST NOT be empty on any running Linux system
  (at minimum PID 1 is always present and readable)

ERRORS:
- Unreadable /proc/<pid>/* → skip that process (process may have exited)

---

## BEHAVIOR: collect-dmi
Constraint: supported

Collect Desktop Management Information (DMI/SMBIOS) via dmidecode.

INPUTS: command_runner

OUTPUTS: DmiScope

STEPS:
1. If dmidecode is not executable: skip this module (return null).
2. Run `dmidecode`; capture full stdout.
3. Store verbatim output in DmiScope.raw_output.

ERRORS:
- dmidecode fails → log to stderr, return null

---

## BEHAVIOR: collect-config-files
Constraint: supported

Collect configuration file contents according to config source policy.

INPUTS: filesystem, config, consistency_cache_exists: bool,
        unpacked_cache_exists: bool

OUTPUTS: collected config file data appended to manifest sections
         (rendered verbatim in human formats; not a distinct JSON scope)

STEPS:
1. Determine effective source mode for allconfigfiles:
   - "On"   → always use hardcoded list
   - "Off"  → never use hardcoded list
   - "Auto" → use hardcoded list only if neither cache file exists

2. If source mode active: iterate config file candidates derived from
   the pattern-based list below. For each candidate path or glob:
   a. Expand any glob patterns to concrete file paths.
   b. Skip if path contains /proc.
   c. Skip if file matches any path in config.exclude.
   d. Skip if file size > config.file_size_limit (and limit > 0).
   e. Skip if filename matches backup patterns:
      *.orig, *.org, *.ori, *.bak, *.bac, *~, #*.
   f. Skip if already included (deduplication).
   g. Special handling for files requiring password blanking:
      Paths matching: /etc/pppoed.conf, /etc/grub.conf,
      /boot/grub/menu.lst, /etc/lilo.conf, /boot/grub2/grub.cfg
      → blank values matching the regex `[Pp]assword\s*=\s*\S+`.
   h. Special handling for C-comment-style files:
      Paths matching: /etc/named.conf, /etc/bind/named.conf,
      /etc/named.conf.local, and zone files referenced within them
      → strip /* */ block comments and // line comments before inclusion.
   i. Include file stat (uid, gid, mode in octal, size, mtime) as
      metadata alongside the verbatim content.
   j. For DNS/Bind configs: after stripping comments, scan for
      `file "<path>"` directives; include each referenced zone file.

   CONFIG FILE PATTERN LIST (each entry is a path or glob):
   // Authentication and identity
   /etc/passwd, /etc/group
   /etc/nsswitch.conf, /etc/pam.d/*, /etc/security/*
   /etc/krb5.conf, /etc/krb5.keytab (metadata only; binary)
   // Network
   /etc/hosts, /etc/hostname, /etc/resolv.conf
   /etc/networks, /etc/protocols, /etc/services
   /etc/hosts.allow, /etc/hosts.deny
   /etc/ntp.conf, /etc/chrony.conf, /etc/chrony.d/*
   /etc/network/interfaces (Debian), /etc/network/interfaces.d/*
   /etc/netconfig, /etc/gai.conf
   // SSH
   /etc/ssh/sshd_config, /etc/ssh/ssh_config, /etc/sshd_config
   // DNS
   /etc/named.conf, /etc/bind/named.conf
   // Mail
   /etc/postfix/main.cf, /etc/postfix/master.cf, /etc/aliases
   /etc/sendmail.cf, /etc/mail/sendmail.cf
   // Web / proxy
   /etc/apache2/httpd.conf, /etc/apache2/apache2.conf
   /etc/httpd/conf/httpd.conf, /etc/nginx/nginx.conf
   /etc/squid/squid.conf, /etc/squid.conf
   // File sharing
   /etc/samba/smb.conf, /etc/smb.conf
   /etc/exports, /etc/exports.d/*
   // Directory services
   /etc/openldap/slapd.conf, /etc/ldap/slapd.conf, /etc/slapd.conf
   /etc/openldap/ldap.conf, /etc/ldap/ldap.conf, /etc/ldap.conf
   // Logging
   /etc/syslog.conf, /etc/syslog-ng/syslog-ng.conf
   /etc/rsyslog.conf, /etc/rsyslog.d/*
   // Init and boot
   /etc/inittab, /etc/grub.conf, /etc/lilo.conf
   /boot/grub/menu.lst, /boot/grub2/grub.cfg
   /etc/default/grub, /etc/grub2.cfg
   // Cron
   /etc/crontab, /etc/cron.d/*, /etc/cron.daily/*
   /etc/cron.weekly/*, /etc/cron.monthly/*
   // Printing
   /etc/cups/cupsd.conf, /etc/printcap
   // Firewall
   /etc/sysconfig/SuSEfirewall2, /etc/sysconfig/iptables
   // Misc system
   /etc/fstab, /etc/mtab, /etc/mdadm.conf, /etc/mdadm/mdadm.conf
   /etc/modules, /etc/modprobe.d/*, /etc/modprobe.conf
   /etc/sysctl.conf, /etc/sysctl.d/*
   /etc/securetty, /etc/shells, /etc/environment
   /etc/profile, /etc/profile.d/*.sh, /etc/bashrc, /etc/bash.bashrc
   /etc/login.defs, /etc/logrotate.conf, /etc/logrotate.d/*
   /etc/updatedb.conf, /etc/ld.so.conf, /etc/ld.so.conf.d/*

3. Always include (if readable, regardless of source mode):
   /etc/ssh/sshd_config or /etc/sshd_config
   /etc/named.conf or /etc/bind/named.conf (with referenced zones)
   /etc/samba/smb.conf or /etc/smb.conf
   /etc/openldap/slapd.conf (or /etc/ldap/slapd.conf or /etc/slapd.conf)
   /etc/openldap/ldap.conf (or /etc/ldap/ldap.conf or /etc/ldap.conf)
   /etc/aliases (if Postfix present)
   If postconf executable: run `postconf -n` and include output verbatim.

4. Process /var/lib/support/*.include files (drop-in extension):
   Each .include file contains a Perl `@files = (...)` array declaration.
   Parse the file using the pattern: extract quoted strings between
   the opening `(` and closing `)`.
   For each extracted path: include the file using the same rules as step 2.
   Do NOT execute Perl; use string parsing only.

5. If config.allsysconfig active:
   Walk /etc/sysconfig/ recursively; include all regular files whose
   content is detected as text (read first 512 bytes; if all bytes are
   printable UTF-8 or ASCII, treat as text). Skip binary files silently.

6. If SUSE-family and /etc/rc.config readable:
   Include /etc/rc.config.
   Include all files matching /etc/rc.config.d/*.config.

7. Crontab collection:
   Parse /etc/crontab; emit CrontabRecord per non-comment data line.
   Parse per-user crontabs:
     SUSE/UnitedLinux: /var/spool/cron/tabs/
     Red Hat / RHEL:   /var/spool/cron/
     Debian/Ubuntu:    /var/spool/cron/crontabs/

8. AppArmor config files (if apparmor active and allsubdomain active):
   Already handled in collect-security-apparmor.

9. GCONF exclusion: if config.gconf = false:
   Skip all files below /etc/opt/gnome.

10. LVM archive exclusion: if config.lvmarchive = false:
    Skip all files below /etc/lvm/archive.

POSTCONDITIONS:
- /etc/shadow is never read (invariant; enforced by exclude list)
- No file larger than file_size_limit is included verbatim

ERRORS:
- Unreadable file → skip silently

---

## BEHAVIOR: collect-installed-rpm
Constraint: supported

Collect installed RPM packages, grouped by packager/distribution.

INPUTS: command_runner

OUTPUTS: PackagesScope, PatternsScope (zypp only)

STEPS:
1. Run `rpm -qa --queryformat
   '%{NAME}::%{VERSION}-%{RELEASE}::%{SIZE}::%{SUMMARY}::%{DISTRIBUTION}::%{PACKAGER}::%{ARCH}::a\n'`.
2. Run `rpm -qa --queryformat '%{DISTRIBUTION}::%{PACKAGER}\n'` and deduplicate
   to get the packager list.
3. For each PackageRecord:
   name = NAME, version = VERSION-RELEASE, size = SIZE (integer bytes),
   summary = SUMMARY, distribution = DISTRIBUTION, packager = PACKAGER,
   arch = ARCH.
4. Retrieve checksum: run `rpm -q --queryformat '%{MD5SUM}\n' <name>`.
   Store as checksum field.
5. For SUSE/SLES: also query installation sources via
   `installation_sources -s` if available; store output as
   supplementary text in manifest (not a distinct JSON scope).
6. For zypp systems: run `zypper patterns` or query RPM pattern packages;
   emit PatternsScope elements with name, version, release.
7. Assemble PackagesScope with _attributes.package_system = "rpm".

POSTCONDITIONS:
- PackagesScope._elements is sorted by package name
- All required PackageRecord fields populated; missing fields = ""

ERRORS:
- rpm command unavailable or fails → empty PackagesScope, log to stderr

---

## BEHAVIOR: collect-installed-deb
Constraint: supported

Collect installed Debian/Ubuntu packages from dpkg status file.

INPUTS: filesystem

OUTPUTS: PackagesScope

STEPS:
1. Open /var/lib/dpkg/status; locate record boundaries (blank lines).
2. For each record whose "Status:" line ends with "installed":
   Extract: Package (→ name), Status, Installed-Size (KiB → size),
   Version (→ version + release = ""), Description (first line → summary),
   Architecture (→ arch).
   Vendor and distribution fields → "".
   Checksum: dpkg does not record an install checksum; set to "".
3. Assemble PackagesScope with _attributes.package_system = "dpkg".

POSTCONDITIONS:
- PackagesScope._elements is sorted by package name

ERRORS:
- /var/lib/dpkg/status unreadable → empty PackagesScope, log to stderr

---

## BEHAVIOR: collect-kernel-config
Constraint: supported

Collect active kernel configuration.

INPUTS: filesystem, command_runner

OUTPUTS: KernelConfigScope

STEPS:
1. If /proc/config.gz readable and gzip executable:
   Run `gzip -dc /proc/config.gz`.
2. Else if /boot/config-$(uname -r) readable:
   Read file directly.
3. Else: skip this module (return null).
4. Parse output: skip lines matching `^#` or empty lines.
   For each remaining line: split on first `=`.
   Emit KernelConfigRecord{key, value}.
5. Return KernelConfigScope sorted by key.

ERRORS:
- Both sources absent or unreadable → return null, log to stderr

---

## BEHAVIOR: collect-repositories
Constraint: supported

Collect configured package repositories.

INPUTS: filesystem, command_runner, backend: PackageVersioningBackend

OUTPUTS: RepositoriesScope

STEPS:
1. If backend = rpm AND installation_sources executable (older SUSE):
   Run `installation_sources -s`; parse output.
   Emit RepositoriesScope with _attributes.repository_system = "zypp".
2. Else if /etc/zypp/repos.d/ or /etc/zypper/repos.d/ readable:
   Parse each *.repo file; extract alias, name, url, type, enabled,
   gpgcheck, autorefresh, priority.
   Emit RepositoriesScope with _attributes.repository_system = "zypp".
3. Else if /etc/yum.repos.d/ readable:
   Parse each *.repo file (INI format).
   Emit RepositoriesScope with _attributes.repository_system = "yum".
4. Else if /etc/apt/sources.list or /etc/apt/sources.list.d/ readable:
   Parse each sources.list line (deb / deb-src).
   Emit RepositoriesScope with _attributes.repository_system = "apt".
5. If no source found: return null.

ERRORS:
- Parse error in repo file → log to stderr, skip that repo entry

---

## BEHAVIOR: collect-services
Constraint: supported

Collect service startup configuration.

INPUTS: filesystem, command_runner

OUTPUTS: ServicesScope

STEPS:
1. Detect init system:
   If /run/systemd/private or /sys/fs/cgroup/systemd readable:
     init_system = "systemd"
   Else if chkconfig executable:
     init_system = "sysvinit"
   Else if initctl executable:
     init_system = "upstart"
   Else: return null.
2. If systemd: run `systemctl list-unit-files --type=service`.
   Parse output: each line "name.service state".
   State values: enabled, disabled, static, masked.
   Emit ServiceRecord per line.
3. If sysvinit: run `chkconfig --list`.
   Parse output; emit ServiceRecord{name, state="enabled"|"disabled"}.
4. If upstart: run `initctl list`; detect legacy sysv flag via
   presence in /etc/init.d/.
   Emit ServiceRecord{name, state, legacy_sysv}.
5. Assemble ServicesScope with _attributes.init_system.

ERRORS:
- Command unavailable or fails → return null, log to stderr

---

## BEHAVIOR: collect-groups
Constraint: supported

Collect system groups from /etc/group.

INPUTS: filesystem

OUTPUTS: GroupsScope

STEPS:
1. Open /etc/group; for each non-comment line:
   Split on ":": name, password, gid, members_csv.
   Split members_csv on ",".
   Emit GroupRecord{name, password, gid (integer or null if empty),
   users: [members]}.
2. Return GroupsScope sorted by name.

ERRORS:
- /etc/group unreadable → empty _elements, log to stderr

---

## BEHAVIOR: collect-users
Constraint: supported

Collect system users from /etc/passwd and /etc/shadow (if readable).

INPUTS: filesystem

OUTPUTS: UsersScope

STEPS:
1. Open /etc/passwd; for each non-comment line:
   Split on ":": name, password_placeholder, uid, gid, comment, home, shell.
   Emit partial UserRecord.
2. If /etc/shadow readable AND path not in config.exclude:
   Open /etc/shadow; for each non-comment line:
   Split on ":": name, encrypted_password, last_changed, min, max, warn,
   disable_days, disabled_date, reserved.
   Merge into matching UserRecord.
   INVARIANT: /etc/shadow is excluded by default (in config.exclude).
3. Return UsersScope sorted by name.

POSTCONDITIONS:
- /etc/shadow is only read if explicitly removed from config.exclude
  (default exclude list always contains "/etc/shadow")

ERRORS:
- /etc/passwd unreadable → empty _elements, exit 1 (system is broken)
- /etc/shadow unreadable or excluded → shadow fields omitted

---

## BEHAVIOR: collect-changed-config-files
Constraint: supported

Identify RPM-owned configuration files that differ from their packaged state.
Populates manifest.changed_config_files aligned with Machinery schema.

INPUTS: command_runner

OUTPUTS: ChangedConfigFilesScope

STEPS:
1. Run `rpm -qca --queryformat '%{NAME}\n'` to get the list of all RPM
   config files with their owning package names.
2. For each unique package: run `rpm -V --nodeps --noscript <pkg>`.
   Parse output lines (format: SM5DLUGTP  c <path>):
   Extract change flags (S=size, M=mode, 5=md5, D=device, L=link,
   U=user, G=group, T=mtime, P=capabilities) and file path.
3. For each changed config file: run `stat` on the file to retrieve
   mode, user, group, type.
4. Emit ChangedConfigFileRecord{name, package_name, package_version,
   status="changed", changes=[], mode, user, group, type}.
   changes[] values: "size", "mode", "md5", "device_number", "link_path",
   "user", "group", "time", "capabilities", "replaced", "deleted".
5. On rpm -V read error for a specific file: emit
   ChangedConfigFileRecord{status="error", error_message=<message>}.
6. Return ChangedConfigFilesScope with _attributes.extracted = false.

ERRORS:
- rpm not available → return null, log to stderr

---

## BEHAVIOR: collect-changed-managed-files
Constraint: supported

Identify RPM-owned non-config files that differ from their packaged state.
Populates manifest.changed_managed_files aligned with Machinery schema.

INPUTS: command_runner

OUTPUTS: ChangedManagedFilesScope

STEPS:
1. Run `rpm -Va --nodeps --noscript` (verify all packages, all files).
2. Skip lines for files marked as config ("c" in flags column).
3. For each non-config changed file: parse change flags; retrieve
   mode, user, group, type via stat.
4. Emit ChangedManagedFileRecord per changed file.
5. Return ChangedManagedFilesScope with _attributes.extracted = false.

ERRORS:
- rpm not available → return null, log to stderr
- Verification errors on individual files → emit status="error" record

---

## BEHAVIOR: find-unpacked
Constraint: supported

Find files below /etc that do not belong to any installed RPM.
Write results to a cache file for use in subsequent runs.

INPUTS:
```
config_dir:  string   // default: /var/lib/support
cache_file:  string   // default: Find_Unpacked.include
search_root: string   // default: /etc
```

OUTPUTS:
```
cache_path: string
```

PRECONDITIONS:
- Requires rpm backend
- May take 5–20 minutes

STEPS:
1. Run `rpm -qla`; collect all files owned by any RPM
   that fall under search_root.
2. Walk search_root recursively; collect all regular files.
3. Filter: files not in RPM file list AND not ending with "~"
   AND readable AND (if ignore_binary=true) not binary/data type
   (detected via `file -p -b`; skip if output matches "Berkeley DB"
   or "data").
4. Sort result; write to cache_file as Perl array:
   `@files = ( "/path/a", "/path/b" );`
5. Create config_dir if it does not exist.
6. Return cache_path.

POSTCONDITIONS:
- cache_file is a valid Perl snippet loadable by collect-config-files
- cache_file is preserved between runs (not overwritten unless
  this BEHAVIOR is explicitly invoked)

ERRORS:
- rpm unavailable → log to stderr, return error
- config_dir not creatable → log to stderr, return error

---

## BEHAVIOR: check-consistency
Constraint: supported

Check that RPM-declared configuration files have not been modified
since installation. Write results to cache for subsequent runs.

INPUTS:
```
config_dir:  string   // default: /var/lib/support
cache_file:  string   // default: Configuration_Consistency.include
```

OUTPUTS:
```
cache_path: string
```

PRECONDITIONS:
- Requires rpm backend
- May take 5–20 minutes

STEPS:
1. Run `rpm -qca --queryformat '%{NAME}\n'` to get all config files
   with owning package names.
2. Build: configfiles[path] = package_name.
3. For each unique package_name: run `rpm -V --nodeps --noscript <pkg>`.
   Parse output; collect paths of changed config files
   (those whose path is confirmed in configfiles for this package;
   skip "missing" prefixed lines).
4. Sort and write cache_file as Perl array:
   `@files = ( "/path/a", "/path/b" );`
5. Create config_dir if it does not exist.
6. Return cache_path.

POSTCONDITIONS:
- cache_file is preserved between runs; overwritten only on explicit invocation
- Content of listed files is included verbatim in subsequent sitar runs

ERRORS:
- rpm unavailable → log to stderr, return error

---

## BEHAVIOR: render
Constraint: required

Render a SitarManifest to one or more output files in the requested format(s).

INPUTS:
```
manifest: SitarManifest
config:   Config
```

OUTPUTS:
```
files_written: []string   // absolute paths of files written
```

PRECONDITIONS:
- manifest is non-empty (at least general_info populated)
- If config.format = "all": outdir must exist or be creatable
- If config.format is single and config.outfile is set:
  parent directory of outfile must be writable

STEPS:
1. Resolve output paths:
   a. Determine effective format set:
      if config.format = "" or config.format = "all":
        active_formats = [html, tex, sdocbook, json, markdown]
      else:
        active_formats = [config.format]
   b. Determine outdir:
      if config.outdir = "" and len(active_formats) > 1:
        outdir = /tmp/sitar-{hostname}-{YYYYMMDDhh}
      else if config.outdir != "":
        outdir = config.outdir
      else:
        outdir = current working directory
      Create outdir recursively if it does not exist; on failure →
      stderr + exit 1.
   c. For each format in active_formats: determine outpath:
      if config.outfile != "" and len(active_formats) = 1:
        outpath = config.outfile
        if outpath has no directory component and outdir != ".":
          outpath = path_join(outdir, outpath)
      else:
        ext = format extension (html→.html, tex→.tex,
              sdocbook→.sdocbook.xml, json→.json, markdown→.md)
        outpath = path_join(outdir, "sitar-" + hostname + ext)
      Create parent directory of outpath recursively if needed.
2. For each format in active_formats:
   a. Open outpath for writing; on failure → stderr + skip this format.
   b. Dispatch to format renderer:
      html, tex, sdocbook, markdown: invoke render-human with the
        manifest and format; MUST produce non-empty output.
      json:     invoke render-json; serialise full manifest.
   c. Apply format-specific escaping to all string values before writing:
      html:     &amp; &lt; &gt;
      tex:      \_ \# \% \& $<$ $>$
      sdocbook: &amp; &lt; &gt;
      json:     JSON string escaping (standard)
      markdown: escape | in table cells
   d. Close file; append outpath to files_written.
3. Log to stderr: "Generating {outpath}...\n" per file written.
4. Return files_written.

POSTCONDITIONS:
- Each file in files_written exists and is non-empty (bytes_written > 0)
- Each human-format file contains all sections from the render-human
  SECTION-MAP for which manifest data is present
- Format-specific escaping applied to all rendered values
- No format failure aborts other formats

ERRORS:
- Output file not writable → stderr + skip that format (not fatal)

---

## BEHAVIOR: render-human
Constraint: required
// Sub-behavior of render; covers html, tex, sdocbook, and markdown formats.
// This BEHAVIOR defines the mandatory section mapping from SitarManifest to
// human-readable output. Every format listed here MUST produce a non-empty
// body. A placeholder comment or empty body is a correctness violation.

Render one human-readable output file from a SitarManifest, producing
all sections in the canonical order defined below.

INPUTS:
```
manifest: SitarManifest
format:   OutputFormat   // one of: html | tex | sdocbook | markdown
outfile:  string
```

OUTPUTS:
```
bytes_written: integer   // must be > 0
```

PRECONDITIONS:
- format is one of html, tex, sdocbook, markdown (not json, not all)
- manifest.general_info is populated

STEPS:
1. Open outfile for writing.
2. Write format header:
   html:     <!DOCTYPE html> + <html> + <head> with inline CSS +
             <body> + <h1>SITAR — System InformaTion At Runtime</h1>
   tex:      \documentclass{scrartcl} + preamble (longtable, verbatim,
             multicol packages) + \begin{document}
   sdocbook: <?xml ...?> + <article> root element
   markdown: # SITAR — System InformaTion At Runtime\n\nHostname: {hostname}
             Date: {collected_at}
3. Write table of contents placeholder (html only: numbered anchor list;
   tex: \tableofcontents; sdocbook: omit; markdown: omit).
4. Write sections in this exact order, using the heading and source
   mapping defined in the SECTION-MAP table below. For each section:
   a. If the manifest field is null or absent: skip the section entirely.
   b. Write the section heading at the level specified.
   c. Write the section body as a table (html/tex/sdocbook) or
      GFM table (markdown) with columns and rows from the source records.
      Column headers are the field names of the record type.
      Each record becomes one row. All values are string-escaped for format.
   d. For RawTextRecord fields: write content verbatim in a preformatted
      block (html: <pre>, tex: verbatim environment, sdocbook: <screen>,
      markdown: fenced code block).
5. Write format footer:
   html:     </body></html>
   tex:      \end{document}
   sdocbook: </article>
   markdown: (none)
6. Close file. Return bytes_written (must be > 0).

SECTION-MAP:
// Each section has a named render function that the translator MUST implement.
// The function takes the manifest and a Renderer instance and returns a string.
// If the manifest source is null or _elements is empty: return empty string.
// Sections appear in the output in the order listed here.
//
// Level 1 = top-level section (html h1, tex \section, sdocbook <section>)
// Level 2 = sub-section      (html h2, tex \subsection)
//
// TABLE RENDERING: for each _elements section, produce one table where:
//   - Column headers = the field names listed in "Columns"
//   - One row per element
//   - All values converted to string; empty string renders as ""
//   - Values escaped for the output format via Renderer.Escape()
//
// VERBATIM RENDERING: for RawTextRecord content, use:
//   html: <pre>...</pre>
//   tex:  \begin{verbatim}...\end{verbatim}
//   sdocbook: <screen>...</screen>
//   markdown: fenced code block

| Render Function              | Level | Manifest Source                         | Columns                                                              |
|------------------------------|-------|-----------------------------------------|----------------------------------------------------------------------|
| renderGeneralInfo            | 1     | manifest.general_info._elements         | key, value                                                           |
| renderCPU                    | 1     | manifest.cpu._elements                  | processor, vendor_id, model_name, cpu_family, model, stepping, cpu_mhz, cache_size |
| renderKernelParams           | 1     | manifest.kernel_params._elements        | key, value                                                           |
| renderNetParams              | 1     | manifest.net_params._elements           | key, value                                                           |
| renderDevices                | 1     | manifest.devices._elements              | name, dma, irq, ports                                                |
| renderPCI                    | 1     | manifest.pci._elements                  | pci, device, class, vendor, svendor, sdevice, rev                    |
| renderSoftwareRAID           | 1     | manifest.storage.software_raid._elements | device, level, partitions, blocks, chunk_size, algorithm            |
| renderPartitions             | 1     | manifest.storage.partitions._elements   | device, type, size, fstype, mountpoint, uuid, label, source, begin_sector, end_sector, mount_options, block_size, inode_density, max_mount_count, df_blocks_kb, df_used_kb, df_avail_kb, df_use_percent |
| renderBtrfs                  | 2     | manifest.storage.btrfs._elements        | for each element: heading level 2 "Btrfs: {label} ({uuid})"; sub-table of devices (devid, size, used, path); sub-table of subvolumes (id, gen, top_level, path); space usage table (data_total, data_used, metadata_total, metadata_used, system_total, system_used) |
| renderFstab                  | 2     | manifest.storage.fstab                  | verbatim preformatted block                                          |
| renderLvmConf                | 2     | manifest.storage.lvm_conf               | verbatim preformatted block                                          |
| renderEVMS                   | 2     | manifest.storage.evms                   | verbatim preformatted block; omit if null                            |
| renderMultipath              | 2     | manifest.storage.multipath              | verbatim preformatted block; omit if null                            |
| renderIDE                    | 1     | manifest.storage.ide._elements          | device, media, model, driver, geometry_phys, geometry_log, capacity_blocks |
| renderSCSI                   | 1     | manifest.storage.scsi._elements         | host, channel, id, lun, vendor, model, revision, type, ansi_rev     |
| renderCCISS                  | 1     | manifest.storage.cciss._elements        | controller_id; raw_output as verbatim block; omit if null            |
| renderAreca                  | 1     | manifest.storage.areca._elements        | controller_id; raw_output as verbatim block; omit if null            |
| renderDAC960                 | 1     | manifest.storage.dac960._elements       | controller_id; raw_output as verbatim block; omit if null            |
| renderGDTH                   | 1     | manifest.storage.gdth._elements         | controller_id; raw_output as verbatim block; omit if null            |
| renderIPS                    | 1     | manifest.storage.ips._elements          | controller_id; raw_output as verbatim block; omit if null            |
| renderCompaqSmart            | 1     | manifest.storage.compaq_smart._elements | controller_id; raw_output as verbatim block; omit if null            |
| renderNetworkInterfaces      | 1     | manifest.network.interfaces._elements   | ifname, link_type, address, flags (join with space), mtu, operstate, ip, prefixlen, broadcast, ip6, ip6_prefixlen |
| renderRouting                | 1     | manifest.network.routes._elements       | dst, gateway, dev, protocol, scope, type, metric, flags (join with space) |
| renderPacketFilter           | 1     | manifest.network.packet_filter._elements | engine, table; raw_output as verbatim block                         |
| renderAppArmor               | 1     | manifest.security_apparmor              | kernel_params sub-table (key, value); profiles sub-table (name, mode); config_files as list |
| renderDMI                    | 1     | manifest.dmi                            | raw_output as verbatim block; omit if null                           |
| renderServices               | 1     | manifest.services._elements             | name, state                                                          |
| renderChangedConfigFiles     | 1     | manifest.changed_config_files._elements | name, package_name, package_version, status, changes, mode, user, group, type |
| renderPackages               | 1     | manifest.packages._elements             | name, version, release, arch, size, summary                          |
| renderKernelConfig           | 1     | manifest.kernel_config._elements        | key, value                                                           |

POSTCONDITIONS:
- bytes_written > 0
- Every section listed above for which the manifest source is non-null
  and non-empty MUST appear in the output
- No section may be replaced by a placeholder comment or left empty
- Heading levels match the SECTION-MAP exactly

ERRORS:
- outfile not writable → stderr + return error (caller skips this format)
- Individual record field contains non-UTF-8 bytes → replace with
  Unicode replacement character U+FFFD; do not abort

---

## BEHAVIOR: render-json
Constraint: required
// Sub-behavior of render; detailed here for JSON schema precision.

Serialise the SitarManifest to a JSON file conforming to the sitar
JSON schema (format_version=1), aligned with Machinery system
description format version 10 for all shared scopes.

INPUTS:
```
manifest: SitarManifest
outfile:  string
```

STEPS:
1. Open outfile for writing.
2. Emit top-level JSON object.
3. Emit "meta" scope first (SitarMeta).
4. For each scope present in manifest (see TYPES: SitarManifest),
   in the order listed there:
   a. Skip if scope is null (data source was absent).
   b. For Machinery-shared scopes (environment, os, packages, patterns,
      repositories, services, groups, users, changed_config_files,
      changed_managed_files, unmanaged_files):
      Emit in Machinery schema v10 format (_attributes, _elements
      where applicable). Sitar extension fields (size, summary,
      distribution, packager in PackageRecord) are emitted
      as additional fields alongside the required Machinery fields;
      they do not conflict with the schema.
   c. For sitar-specific scopes (cpu, kernel_params, kernel_config,
      devices, pci, storage, network, security_apparmor, processes, dmi):
      Emit with ScopeWrapper pattern (_attributes, _elements) where
      appropriate.
5. Emit closing "}" and flush.

POSTCONDITIONS:
- Output is valid JSON (no trailing commas, all strings properly escaped)
- Machinery-compatible scope field names use underscore_style
- format_version is always 1

ERRORS:
- Write failure → stderr + exit 1

---

## INTERFACES

```
Filesystem {
  required-methods:
    ReadFile(path: string) -> (content: string, error)
    ReadFileLimited(path: string, limit: integer) -> (content: string, error)
    Glob(pattern: string)  -> ([]string, error)
    Exists(path: string)   -> bool
    IsExecutable(path: string) -> bool
    Stat(path: string)     -> (FileInfo, error)
  implementations-required:
    production:  OSFilesystem
    test-double: FakeFilesystem {
      configurable: file map (path -> content), directory list,
                    executable list, stat info map
    }
}

FileInfo := {
  uid:   integer
  gid:   integer
  mode:  string    // octal string, e.g. "0644"
  size:  integer   // bytes
  mtime: string    // local time string
}

CommandRunner {
  required-methods:
    Run(cmd: string, args: []string) -> (stdout: string, stderr: string, error)
    // Production implementation MUST:
    // 1. Set PATH=/sbin:/bin:/usr/bin:/usr/sbin in the child process environment.
    // 2. Execute cmd with args using the platform subprocess API.
    // 3. Capture and return stdout and stderr as strings.
    // 4. Restore the original PATH in the parent after invocation.
    // 5. Return the process error (non-zero exit) as the error value.
    // A stub returning ("", "", nil) for all inputs is NOT a valid
    // production implementation and will cause all command-dependent
    // collection modules to silently produce empty output.
    // getHostname MUST call Run("hostname", ["-f"]) and return TrimSpace(stdout).
    // getUname    MUST call Run("uname", ["-a"])    and return TrimSpace(stdout).
  implementations-required:
    production:  OSCommandRunner
    test-double: FakeCommandRunner {
      configurable: response map (command -> CommandResponse)
    }
}

CommandResponse := {
  stdout: string
  stderr: string
  error:  error | null
}

Renderer {
  required-methods:
    Header(manifest: SitarManifest) -> string
    TOC(sections: []string)         -> string
    Section(title: string, level: integer, content: string) -> string
    Footer()                        -> string
    Escape(raw: string)             -> string
  implementations-required:
    production-html:      HTMLRenderer
    production-tex:       TeXRenderer
    production-sdocbook:  DocBookRenderer
    production-json:      JSONRenderer
    production-markdown:  MarkdownRenderer
    test-double:          FakeRenderer {
      configurable: recorded sections list
    }
  // ALL five production implementations are required.
  // A translator MUST NOT omit or stub any of the five.
  // For each implementation, Header() MUST return a non-empty format-valid
  // header string and Footer() MUST return a non-empty format-valid footer
  // string. Section() MUST return a non-empty string when content is non-empty.
  // Specifically:
  //   HTMLRenderer.Header()    returns valid HTML up to and including <body>
  //   HTMLRenderer.Footer()    returns </body></html>
  //   TeXRenderer.Header()     returns \documentclass{scrartcl} preamble
  //                            through \begin{document}
  //   TeXRenderer.Footer()     returns \end{document}
  //   DocBookRenderer.Header() returns <?xml ...?><article ...>
  //   DocBookRenderer.Footer() returns </article>
  //   MarkdownRenderer.Header() returns # SITAR... title block
  //   MarkdownRenderer.Footer() returns empty string (acceptable)
  //   JSONRenderer is handled entirely by render-json, not this interface
}

PackageBackend {
  required-methods:
    ListInstalled()         -> ([]PackageRecord, error)
    QueryFile(path: string) -> (package_name: string, error)
    VerifyAll()             -> ([]ChangedFileRecord, error)
    VerifyPackage(name: string) -> ([]ChangedFileRecord, error)
  implementations-required:
    production-rpm:   RPMBackend
    production-dpkg:  DpkgBackend
    production-none:  NullBackend
    test-double:      FakePackageBackend {
      configurable: package list, file-owner map, verify result list
    }
}
```

---

## PRECONDITIONS

- Must run as root (uid=0)
- /proc filesystem must be mounted and readable
- At least one output format must be valid or format="" (all)
- config file (/etc/sysconfig/sitar) need not exist
- If format=json: output is machine-readable; yast2 format is not
  supported (removed from scope)

---

## POSTCONDITIONS

- All requested output files are written and non-empty
- manifest.general_info, manifest.cpu, manifest.kernel_params, and
  manifest.os are populated on every supported Linux system
- Each output file has a valid header, body, and footer for its format
- JSON output conforms to sitar JSON schema format_version=1
- stderr contains one "Generating {path}..." line per file written
- Exit code is 0 on success, 1 on fatal error, 2 on invocation error

---

## INVARIANTS

- [observable]      uid=0 check happens before any filesystem or
                    command runner invocation
- [observable]      Format-specific escaping is applied to all string
                    values before writing (HTML entities, TeX specials,
                    XML entities, JSON string escaping as appropriate)
- [observable]      A missing or non-executable external tool causes the
                    corresponding collection module to be skipped, not aborted
- [observable]      /etc/shadow and any path in config.exclude are
                    never read or included in any output format
- [observable]      cache files in /var/lib/support/ are preserved
                    between runs; only overwritten on explicit invocation
                    of check-consistency or find-unpacked
- [observable]      All RAID controller sub-scopes are absent from JSON
                    output when the corresponding hardware is not detected
- [observable]      JSON output format_version is always 1; Machinery-shared
                    scope names and field names are identical to Machinery
                    schema v10; sitar extension fields are additive only
- [observable]      Every BEHAVIOR with Constraint: required MUST produce
                    non-empty, non-placeholder output when its declared data
                    source is present on the running system. Returning an
                    empty _elements array when /proc/cpuinfo, /proc/interrupts,
                    mount output, ifconfig output, or the package database is
                    readable and non-empty is a correctness violation, not
                    graceful degradation. A translator MUST NOT stub a
                    required BEHAVIOR and defer its implementation.
- [observable]      Every human-readable output format (html, tex, sdocbook,
                    markdown) MUST produce a non-empty body containing at
                    least the sections listed in BEHAVIOR: render-human.
                    A placeholder comment or empty file is not acceptable
                    output for any format declared in OutputFormat.
- [implementation]  OSCommandRunner.Run MUST execute real system commands
                    via the platform subprocess API. A stub implementation
                    that returns ("", "", nil) for all inputs is not a valid
                    production implementation and will cause all
                    command-dependent collection modules to silently
                    produce empty output, violating the observable
                    non-empty invariant above
- [implementation]  All external tool invocations go through CommandRunner;
                    no direct shell calls in collection modules
- [implementation]  PATH is set to /sbin:/bin:/usr/bin:/usr/sbin for
                    every CommandRunner.Run call and cleared afterwards
- [implementation]  Each Renderer implementation is independent; adding
                    a new format requires only a new Renderer and a new
                    OutputFormat value — no changes to collection modules
- [implementation]  Each collection module is independently skippable;
                    no module has a hard dependency on another module's output
- [implementation]  Distribution-specific collection (installed packages,
                    chkconfig, changed-config-files, kernel config) is
                    gated on DistributionFamily; unknown family silently
                    skips those modules

---

## EXAMPLES

EXAMPLE: no_arguments_shows_help
GIVEN:
  argv = []   // sitar called with no arguments
WHEN:
  prepare-config is called
THEN:
  help text printed to stdout
  exit_code = 0
  collection does NOT run

EXAMPLE: all_formats_with_outdir
GIVEN:
  uid = 0
  /proc is mounted
  argv = ["all", "outdir=/tmp/myreport"]
WHEN:
  collect and render are called
THEN:
  /tmp/myreport directory created if it did not exist
  files_written contains paths:
    /tmp/myreport/sitar-{hostname}.html
    /tmp/myreport/sitar-{hostname}.tex
    /tmp/myreport/sitar-{hostname}.sdocbook.xml
    /tmp/myreport/sitar-{hostname}.json
    /tmp/myreport/sitar-{hostname}.md
  no file is written to the current working directory
  each file is non-empty (bytes_written > 0)
  each human-format file contains at minimum the "General Information",
    "CPU", "Partitions, Mounts, LVM", "Networking Interfaces", and
    "Installed Packages" sections
  exit_code = 0

EXAMPLE: single_format_with_outdir
GIVEN:
  uid = 0
  argv = ["format=html", "outdir=/var/tmp/sitar-out"]
WHEN:
  collect and render are called
THEN:
  /var/tmp/sitar-out directory created if it did not exist
  file written to /var/tmp/sitar-out/sitar-{hostname}.html
  no file written to current working directory
  bytes_written > 0
  exit_code = 0

EXAMPLE: single_json_output
GIVEN:
  uid = 0
  argv = ["format=json", "outfile=/tmp/snapshot.json"]
WHEN:
  prepare-config parses argv; collect and render-json are called
THEN:
  /tmp/snapshot.json is written with bytes_written > 0
  JSON top-level keys include: meta, environment, os, packages,
    cpu, kernel_params, storage, network
  meta.format_version = 1
  cpu._elements is non-empty
  kernel_params._elements is non-empty
  packages._attributes.package_system present
  packages._elements is non-empty (at least one package installed)
  exit_code = 0

EXAMPLE: not_root
GIVEN:
  uid = 1000
WHEN:
  collect is called
THEN:
  stderr contains "Please run sitar as user root."
  exit_code = 1

EXAMPLE: unknown_format
GIVEN:
  argv = ["format=pdf"]
WHEN:
  prepare-config parses argv
THEN:
  stderr contains error referencing "pdf"
  exit_code = 2

EXAMPLE: missing_dmidecode
GIVEN:
  uid = 0
  dmidecode is not installed (IsExecutable returns false)
WHEN:
  collect runs collect-dmi
THEN:
  manifest.dmi is null
  dmi scope absent from JSON output
  no error raised; collection continues
  exit_code = 0

EXAMPLE: check_consistency_writes_cache
GIVEN:
  uid = 0
  config.check_consistency = true
  rpm backend available
WHEN:
  check-consistency is called
THEN:
  /var/lib/support/Configuration_Consistency.include is written
  file contains valid Perl "@files = (...);" declaration
  file is preserved on next run without: sitar check-consistency
  exit_code = 0

EXAMPLE: find_unpacked_skips_binary
GIVEN:
  uid = 0
  config.find_unpacked = true
  /etc/foo.conf not owned by any RPM
  /etc/foo.db is a Berkeley DB binary file
WHEN:
  find-unpacked is called with ignore_binary = true
THEN:
  /var/lib/support/Find_Unpacked.include contains "/etc/foo.conf"
  /etc/foo.conf is NOT present (it is a binary; skipped)
  exit_code = 0

EXAMPLE: shadow_excluded_by_default
GIVEN:
  uid = 0
  config.exclude = ["/etc/shadow"]   // default
WHEN:
  collect-users runs
THEN:
  UserRecords have password = "x" or "!"
  encrypted_password field absent (shadow not read)
  /etc/shadow not opened at any point
  exit_code = 0

EXAMPLE: ext4_filesystem_attributes
GIVEN:
  uid = 0
  /dev/sda1 is an ext4 filesystem, mounted at /
  tune2fs -l /dev/sda1 is readable
WHEN:
  collect-storage runs
THEN:
  PartitionRecord for /dev/sda1 includes:
    filesystem = "ext4"
    mount_point = "/"
    block_size = "4096"
    max_mount_count populated
    inode_density computed and non-empty
  exit_code = 0

EXAMPLE: rpm_package_with_extensions
GIVEN:
  uid = 0
  rpm backend available
  package "zypper" is installed
WHEN:
  collect-installed-rpm runs
THEN:
  PackageRecord for "zypper" contains:
    name, version, release, arch, vendor, checksum (required Machinery fields)
    size (bytes), summary, distribution, packager (sitar extension fields)
  packages._attributes.package_system = "rpm"
  exit_code = 0

EXAMPLE: html_output_non_empty
GIVEN:
  uid = 0
  argv = ["format=html", "outfile=/tmp/sitar.html"]
  /proc/cpuinfo readable with at least one processor entry
  mount table non-empty
WHEN:
  collect and render-human are called
THEN:
  /tmp/sitar.html is written with bytes_written > 0
  file contains literal text "General Information"
  file contains literal text "CPU"
  file contains literal text "Partitions, Mounts, LVM"
  file contains literal text "Networking Interfaces"
  file does NOT contain "placeholder"
  file does NOT contain "<!-- placeholder"
  file does NOT contain empty <table></table> for sections whose
    manifest source is non-empty
  exit_code = 0


EXAMPLE: tex_output_non_empty
GIVEN:
  uid = 0
  argv = ["format=tex", "outfile=/tmp/sitar.tex"]
  /proc/cpuinfo readable
  mount table non-empty
WHEN:
  collect and render-human are called with format=tex
THEN:
  /tmp/sitar.tex is written with bytes_written > 0
  file begins with \documentclass
  file contains \section{General Information}
  file contains \section{CPU}
  file contains \section{Partitions, Mounts, LVM}
  file ends with \end{document}
  file does NOT contain "placeholder"
  exit_code = 0

EXAMPLE: sdocbook_output_non_empty
GIVEN:
  uid = 0
  argv = ["format=sdocbook", "outfile=/tmp/sitar.sdocbook.xml"]
  /proc/cpuinfo readable
  mount table non-empty
WHEN:
  collect and render-human are called with format=sdocbook
THEN:
  /tmp/sitar.sdocbook.xml is written with bytes_written > 0
  file begins with <?xml
  file contains <article
  file contains <title>General Information</title>
  file contains <title>CPU</title>
  file ends with </article>
  file does NOT contain "placeholder"
  exit_code = 0

EXAMPLE: markdown_output_non_empty
GIVEN:
  uid = 0
  argv = ["format=markdown", "outfile=/tmp/sitar.md"]
  /proc/cpuinfo readable
  mount table non-empty
WHEN:
  collect and render-human are called with format=markdown
THEN:
  /tmp/sitar.md is written with bytes_written > 0
  file begins with # SITAR
  file contains ## General Information or # General Information
  file contains ## CPU or # CPU
  file contains ## Partitions, Mounts, LVM
  file contains GFM table rows (lines starting with |)
  file does NOT contain "placeholder"
  exit_code = 0

---

## DEPENDENCIES

```
// Required tools (modules fail gracefully if absent):
  ip           iproute2 >= 4.12  // network interfaces (ip -j addr show)
                                  // routing table     (ip -j route show)
  lsblk        util-linux >= 2.27 // block device enumeration (lsblk -J)
  findmnt      util-linux >= 2.27 // mount options (findmnt -J)
  df           any                // disk space statistics (df -PPk)
  tune2fs      any                // ext2/3/4 filesystem attributes (e2fsprogs)
  hostname     any                // FQDN detection
  uname        any                // kernel/architecture info
  rpm          >= 4.0             // package queries (rpm-family distros)
  dpkg         >= 1.10            // package queries (deb-family distros)
  btrfs        btrfs-progs >= 4.5 // btrfs subvolume and filesystem info (optional)
  lspci        any                // PCI device enumeration (optional)
  dmidecode    any                // DMI/BIOS information (optional)
  chkconfig    any                // sysvinit service config (optional; legacy)
  systemctl    systemd >= 210     // systemd service config (optional)
  gzip         any                // /proc/config.gz decompression (optional)
  iptables     >= 1.4             // firewall rules (optional)
  hpacucli     any                // HP SmartArray controllers (optional)
  cpqacucli    any                // Compaq SmartArray controllers (optional)
  cli64/cli32  any                // Areca RAID controllers (optional)
  evms_gather_info any            // EVMS storage (optional; legacy)
  multipath    any                // DM multipath (optional)
  postconf     any                // Postfix configuration dump (optional)

// Tools removed from scope (replaced by modern alternatives):
//   ifconfig  → replaced by ip -j addr show
//   route     → replaced by ip -j route show
//   mount     → replaced by findmnt -J (with mount as fallback)
//   fdisk     → replaced by lsblk -J  (with fdisk as fallback)
//   lshal     → removed; HAL is not present on any supported target

No runtime library dependencies beyond the standard library of the
implementation language.
All external tool invocations go through the CommandRunner interface.
```

Language-specific implementation hints:

  cli-tool.go.milestones.hints.md:
    hints-file: cli-tool.go.milestones.hints.md
    // Go scaffold-first patterns applicable to any cli-tool.
    // Read before writing any code.

  sitar.implementation.hints.md:
    hints-file: sitar.implementation.hints.md
    // Sitar-specific Go implementation details: file layout, render
    // function names, milestone verification commands, known failure modes.
    // Advisory only — a correct M0 is achievable from the spec and generic
    // hints alone. This file improves quality but is not required.

---

## DEPLOYMENT

Runtime: command-line tool, single static binary, no runtime
         dependencies beyond the external tools listed in DEPENDENCIES.

Invocation:
```
sitar                             no arguments: print help and exit 0
sitar all                         produce all formats, auto outdir,
                                  plus consistency + find-unpacked cache
sitar format=html outfile=...     single format, explicit outfile
sitar format=html outdir=...      single format, output to named directory
sitar format=json outfile=...     machine-readable JSON output
sitar check-consistency           pre-run cache generation only
sitar find-unpacked               pre-run cache generation only
sitar version                     print version and exit 0
sitar help                        print usage and exit 0
```

CLI style: bare-word commands and key=value options.
POSIX --flag style is forbidden (template constraint CLI-ARG-STYLE).

Config file: /etc/sysconfig/sitar
  SITAR_OPT_FORMAT, SITAR_OPT_OUTDIR, SITAR_OPT_OUTFILE,
  SITAR_OPT_LIMIT, SITAR_OPT_GCONF, SITAR_OPT_ALLCONFIGFILES,
  SITAR_OPT_ALLSUBDOMAIN, SITAR_OPT_ALLSYSCONFIG,
  SITAR_OPT_EXCLUDE, SITAR_OPT_LVMARCHIVE.

Extension mechanism:
  Drop *.include files into /var/lib/support/ to add config files
  to the collection. Each file must contain exactly:
    @files = ( "/path/one", "/path/two" );
  This is the PaDS-inherited extension point; format unchanged.

JSON output compatibility:
  The JSON output follows the Machinery system description format
  (manifest.json structure) for all shared scopes. Applications that
  can consume Machinery manifests can read sitar JSON output for
  the shared scopes. Sitar-specific scopes (cpu, kernel_params,
  kernel_config, devices, pci, storage, network, security_apparmor,
  processes, dmi) are additional scopes not present in Machinery
  and will be ignored by Machinery-based tooling.

Install locations:
  Binary:      /usr/bin/sitar
  Man page:    /usr/share/man/man1/sitar.1
  Config:      /etc/sysconfig/sitar
  Cache dir:   /var/lib/support/
  Data dir:    /usr/share/sitar/   // proc.txt and other static data

Packaging:
  RPM and DEB required. OBS build target (build.opensuse.org).
  curl-based installation is forbidden (supply chain security).

Platform: Linux only. Root required.

Target distributions (minimum supported versions):
  RHEL / CentOS / AlmaLinux / Rocky:  8, 9, 10
  SUSE Linux Enterprise Server:       12 SP5, 15 SP1+, 16
  openSUSE Leap:                      15.5+
  Ubuntu:                             23.04+
  Debian:                             12+

Minimum tool versions enforced by target floor (SLES 12 SP5 / RHEL 8):
  iproute2 >= 4.12  (ip -j support)
  util-linux >= 2.27 (lsblk -J, findmnt -J support)
  btrfs-progs >= 4.5 (text parsing; no --format json required)

---

## MILESTONE: 0.9.0-M0
Status: active
Scaffold: true
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Full scaffold pass. Every source file, every type definition,
// every function signature, every stub body. No real collection logic.
// Compile gate is the only acceptance criterion.
// All subsequent milestones fill in stubs — they never create new files
// or new types.

Included BEHAVIORs:
  prepare-config, detect-distribution, collect, collect-general-info,
  collect-environment, collect-os, collect-chkconfig, collect-cpu,
  collect-kernel-params, collect-net-params, collect-devices, collect-pci,
  collect-storage, collect-btrfs, collect-network-interfaces,
  collect-network-routing, collect-network-firewall,
  collect-security-apparmor, collect-processes, collect-dmi,
  collect-config-files, collect-installed-rpm, collect-installed-deb,
  collect-kernel-config, collect-repositories, collect-services,
  collect-groups, collect-users, collect-changed-config-files,
  collect-changed-managed-files, find-unpacked, check-consistency,
  render, render-human, render-json

Deferred BEHAVIORs:
  // None — all BEHAVIORs are included but implemented as stubs.
  // See Hints-file for language-specific stub body conventions.

Acceptance criteria:
  binary compiles with no errors
  binary is statically linked (no runtime shared library dependencies)
  sitar version                     prints "sitar 0.9.0" and exits 0
  sitar help                        prints usage text and exits 0
  sitar format=unknown_format       exits 2 (invocation error)
  sitar (as non-root)               exits 1 with "Please run sitar as user root."
                                    when any collection is attempted

---

## MILESTONE: 0.9.0-M1
Status: pending
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Identity pass. Real implementations for config parsing, distribution
// detection, OS/environment/general-info collection, and JSON rendering.
// After this milestone: running sitar as root with format=json produces
// a valid, non-empty JSON file with meta, os, environment, and
// general_info scopes populated. All other scopes are empty arrays.

Included BEHAVIORs:
  prepare-config, detect-distribution, collect-general-info,
  collect-environment, collect-os, render-json

Deferred BEHAVIORs:
  collect, collect-chkconfig, collect-cpu, collect-kernel-params,
  collect-net-params, collect-devices, collect-pci, collect-storage,
  collect-btrfs, collect-network-interfaces, collect-network-routing,
  collect-network-firewall, collect-security-apparmor, collect-processes,
  collect-dmi, collect-config-files, collect-installed-rpm,
  collect-installed-deb, collect-kernel-config, collect-repositories,
  collect-services, collect-groups, collect-users,
  collect-changed-config-files, collect-changed-managed-files,
  find-unpacked, check-consistency, render, render-human

Acceptance criteria:
  binary compiles with no errors
  sitar version                     prints "sitar 0.9.0" and exits 0
  sitar format=pdf                  exits 2, stderr references "pdf"
  sitar format=json outfile=...     (run as root) exits 0
  output JSON: meta.format_version  equals 1
  output JSON: meta.sitar_version   equals "0.9.0"
  output JSON: meta.hostname        is non-empty string
  output JSON: meta.uname           is non-empty string
  output JSON: os.architecture      is non-null non-empty string
  output JSON: general_info._elements  contains exactly 9 records
  output JSON: general_info._elements  record with key "hostname" is non-empty

---

## MILESTONE: 0.9.0-M2
Status: pending
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Hardware and kernel pass. Real implementations for all /proc-based
// collection modules.
// After this milestone: JSON output contains populated cpu, kernel_params,
// net_params, devices, pci, processes, and dmi scopes.

Included BEHAVIORs:
  collect-cpu, collect-kernel-params, collect-net-params,
  collect-devices, collect-pci, collect-processes, collect-dmi

Deferred BEHAVIORs:
  collect, collect-chkconfig, collect-storage, collect-btrfs,
  collect-network-interfaces, collect-network-routing,
  collect-network-firewall, collect-security-apparmor,
  collect-config-files, collect-installed-rpm, collect-installed-deb,
  collect-kernel-config, collect-repositories, collect-services,
  collect-groups, collect-users, collect-changed-config-files,
  collect-changed-managed-files, find-unpacked, check-consistency,
  render, render-human

Acceptance criteria:
  binary compiles with no errors
  sitar format=json outfile=...     (run as root) exits 0
  output JSON: cpu._elements        is non-empty array
  output JSON: cpu._elements[0].vendor_id  is non-empty string
  output JSON: kernel_params._elements     contains at least 20 records
  output JSON: net_params._elements        is non-empty array
  output JSON: devices._elements           is non-empty array
  output JSON: processes._elements         is non-empty array
  output JSON: processes._elements[0].pid  is non-empty string

---

## MILESTONE: 0.9.0-M3
Status: pending
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Storage pass. Real implementation of collect-storage and collect-btrfs.
// Uses lsblk -J as primary source with fdisk fallback; findmnt -J for
// mount options; tune2fs for ext attributes; btrfs tools for btrfs data.
// After this milestone: JSON output contains populated storage scope.

Included BEHAVIORs:
  collect-storage, collect-btrfs

Deferred BEHAVIORs:
  collect, collect-chkconfig, collect-network-interfaces,
  collect-network-routing, collect-network-firewall,
  collect-security-apparmor, collect-config-files,
  collect-installed-rpm, collect-installed-deb, collect-kernel-config,
  collect-repositories, collect-services, collect-groups, collect-users,
  collect-changed-config-files, collect-changed-managed-files,
  find-unpacked, check-consistency, render, render-human

Acceptance criteria:
  binary compiles with no errors
  sitar format=json outfile=...     (run as root) exits 0
  output JSON: storage.partitions._elements   is non-empty array
  output JSON: storage.partitions._elements[0].device   is non-empty string
  output JSON: storage.partitions._elements[0].source   is "lsblk" or "fdisk"

---

## MILESTONE: 0.9.0-M4
Status: pending
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Networking pass. Real implementations for network interfaces, routing,
// and firewall collection using ip -j as primary source.
// After this milestone: JSON output contains populated network scope.

Included BEHAVIORs:
  collect-network-interfaces, collect-network-routing,
  collect-network-firewall

Deferred BEHAVIORs:
  collect, collect-chkconfig, collect-security-apparmor,
  collect-config-files, collect-installed-rpm, collect-installed-deb,
  collect-kernel-config, collect-repositories, collect-services,
  collect-groups, collect-users, collect-changed-config-files,
  collect-changed-managed-files, find-unpacked, check-consistency,
  render, render-human

Acceptance criteria:
  binary compiles with no errors
  sitar format=json outfile=...     (run as root) exits 0
  output JSON: network.interfaces._elements         is non-empty array
  output JSON: network.interfaces._elements         contains entry with ifname "lo"
  output JSON: network.routes._elements             is non-empty array
  output JSON: network.routes._elements[0].dev      is non-empty string
  output JSON: network.packet_filter._elements      is non-empty array
  output JSON: network.packet_filter._elements[0].engine  is non-empty string

---

## MILESTONE: 0.9.0-M5
Status: pending
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Packages and services pass. Real implementations for all package
// management, repository, service, user, group, and changed-file modules.
// After this milestone: JSON output is nearly complete.

Included BEHAVIORs:
  collect-installed-rpm, collect-installed-deb, collect-repositories,
  collect-services, collect-chkconfig, collect-groups, collect-users,
  collect-changed-config-files, collect-changed-managed-files,
  collect-kernel-config

Deferred BEHAVIORs:
  collect, collect-security-apparmor, collect-config-files,
  find-unpacked, check-consistency, render, render-human

Acceptance criteria:
  binary compiles with no errors
  sitar format=json outfile=...     (run as root) exits 0
  output JSON: packages._elements                      is non-empty array
  output JSON: packages._elements[0].name              is non-empty string
  output JSON: packages._elements[0].version           is non-empty string
  output JSON: packages._attributes.package_system     is "rpm" or "dpkg"
  output JSON: services._elements                      is non-empty array
  output JSON: services._elements[0].name              is non-empty string
  output JSON: groups._elements                        is non-empty array
  output JSON: users._elements                         is non-empty array

---

## MILESTONE: 0.9.0-M6
Status: pending
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Human renderers pass. Real implementation of render-human with all
// 30 named render functions and the render BEHAVIOR dispatch logic.
// After this milestone: all five output formats produce non-empty,
// correctly structured files. This is purely a rendering pass —
// no new collection logic.

Included BEHAVIORs:
  render, render-human

Deferred BEHAVIORs:
  collect, collect-security-apparmor, collect-config-files,
  find-unpacked, check-consistency

Acceptance criteria:
  binary compiles with no errors
  sitar all outdir=...              (run as root) exits 0
  output directory contains exactly one .html file, size > 0
  output directory contains exactly one .tex file, size > 0
  output directory contains exactly one .json file, size > 0
  output directory contains exactly one .md file, size > 0
  output directory contains exactly one .sdocbook.xml file, size > 0
  html file contains the text "General Information"
  html file contains the text "CPU"
  html file contains the text "Partitions"
  html file contains the text "Networking"
  html file contains the text "Installed Packages"
  tex file contains the text "\documentclass"
  xml file contains the text "<?xml"
  md file contains the text "# SITAR"
  none of the output files contain the word "placeholder"

---

## MILESTONE: 0.9.0-M7
Status: pending
Hints-file: cli-tool.go.milestones.hints.md, sitar.implementation.hints.md

// Config files and cache pass. Real implementations for the slow,
// optional modules. Also: the collect BEHAVIOR orchestration is
// finalised here — it previously called stubs; now it wires the
// real module sequence.
// After this milestone: sitar is feature-complete at v0.9.0.

Included BEHAVIORs:
  collect, collect-security-apparmor, collect-config-files,
  find-unpacked, check-consistency

Deferred BEHAVIORs:
  // None — this milestone completes the implementation.

Acceptance criteria:
  binary compiles with no errors
  sitar all outdir=...              (run as root) exits 0
  html output file size             greater than 50000 bytes
  json output file size             greater than 100000 bytes
  sitar check-consistency           (run as root) exits 0
  /var/lib/support/Configuration_Consistency.include  exists after above
  sitar find-unpacked               (run as root) exits 0
  /var/lib/support/Find_Unpacked.include              exists after above
