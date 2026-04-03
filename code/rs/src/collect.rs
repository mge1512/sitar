// collect.rs — collect orchestrator + collect-general-info, collect-environment, collect-os
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::interfaces::{Filesystem, CommandRunner};
use crate::types::*;
use crate::detect::parse_os_release_field;
use crate::collect_hw;
use crate::collect_storage;
use crate::collect_network;
use crate::collect_pkg;
use crate::collect_config;

const SITAR_VERSION: &str = "0.9.0";

macro_rules! sitar_debug {
    ($($arg:tt)*) => {
        if std::env::var("SITAR_DEBUG").as_deref() == Ok("1") {
            eprintln!("DEBUG: {}", format!($($arg)*));
        }
    };
}

/// collect BEHAVIOR — orchestrate all enabled collection modules
pub fn collect(
    config: &Config,
    dist: &DistributionInfo,
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
) -> SitarManifest {
    // Step 1: verify uid=0
    let uid = unsafe { libc_geteuid() };
    if uid != 0 {
        eprintln!("Please run sitar as user root.");
        std::process::exit(1);
    }

    let mut manifest = SitarManifest::default();

    // Step 4: initialise meta
    manifest.meta.format_version = 1;
    manifest.meta.sitar_version  = SITAR_VERSION.to_string();
    manifest.meta.collected_at   = utc_now();
    manifest.meta.hostname       = get_hostname(cr);
    manifest.meta.uname          = get_uname(cr);

    // Step 5: general-info
    manifest.general_info = collect_general_info(fs, cr, dist);

    // Step 6: environment
    manifest.environment = collect_environment(fs);

    // Step 7: OS
    manifest.os = collect_os(fs, dist, cr);

    // Step 8: CPU
    manifest.cpu = collect_hw::collect_cpu(fs, cr);

    // Step 9: kernel params
    manifest.kernel_params = collect_hw::collect_kernel_params(fs);

    // Step 10: net params
    manifest.net_params = collect_hw::collect_net_params(fs);

    // Step 11: devices
    manifest.devices = collect_hw::collect_devices(fs);

    // Step 12: PCI
    manifest.pci = collect_hw::collect_pci(fs, cr);

    // Step 13+14: storage
    manifest.storage = collect_storage::collect_storage(fs, cr);

    // Step 15-17: network
    manifest.network.interfaces    = collect_network::collect_network_interfaces(cr);
    manifest.network.routes        = collect_network::collect_network_routing(cr);
    manifest.network.packet_filter = collect_network::collect_network_firewall(fs, cr);

    // Step 18: AppArmor (supported; skip if not found)
    manifest.security_apparmor = collect_config::collect_security_apparmor(fs, cr, config);

    // Step 19: processes
    manifest.processes = collect_hw::collect_processes(fs);

    // Step 20: DMI
    manifest.dmi = collect_hw::collect_dmi(cr);

    // Step 21: distribution-specific
    match dist.family {
        DistributionFamily::Rpm => {
            manifest.services = collect_pkg::collect_chkconfig(cr)
                .unwrap_or_default();
            let (pkgs, pats) = collect_pkg::collect_installed_rpm(cr);
            manifest.packages  = pkgs;
            manifest.patterns  = pats;
            manifest.repositories = collect_pkg::collect_repositories(fs, cr, &dist.backend)
                .unwrap_or_default();
            manifest.groups = collect_pkg::collect_groups(fs);
            manifest.users  = collect_pkg::collect_users(fs, config);
            manifest.changed_config_files  = collect_pkg::collect_changed_config_files(cr)
                .unwrap_or_default();
            manifest.changed_managed_files = collect_pkg::collect_changed_managed_files(cr)
                .unwrap_or_default();
            manifest.kernel_config = collect_config::collect_kernel_config(fs, cr)
                .unwrap_or_default();
        }
        DistributionFamily::Deb => {
            manifest.packages = collect_pkg::collect_installed_deb(fs, &dist.dpkg_status);
            manifest.repositories = collect_pkg::collect_repositories(fs, cr, &dist.backend)
                .unwrap_or_default();
            manifest.groups = collect_pkg::collect_groups(fs);
            manifest.users  = collect_pkg::collect_users(fs, config);
            manifest.kernel_config = collect_config::collect_kernel_config(fs, cr)
                .unwrap_or_default();
        }
        DistributionFamily::Unknown => {
            sitar_debug!("collect: unknown distribution family; skipping distro-specific modules");
        }
    }

    // Step 2-3: find-unpacked / check-consistency (if requested)
    if config.find_unpacked {
        let _ = collect_config::find_unpacked(cr, "/var/lib/support", "Find_Unpacked.json", "/etc");
    }
    if config.check_consistency {
        let _ = collect_config::check_consistency(cr, "/var/lib/support", "Configuration_Consistency.json");
    }

    manifest
}

/// collect-general-info BEHAVIOR
pub fn collect_general_info(
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
    dist: &DistributionInfo,
) -> ScopeWrapper<GeneralInfoRecord> {
    let mut elements = Vec::new();

    // Step 1: hostname
    let hostname = get_hostname(cr);

    // Step 2: OS release from detect-distribution
    let os_release = dist.release.clone();

    // Step 3: uname
    let uname = get_uname(cr);

    // Step 4: local time
    let collected_at = local_time_string();

    // Step 5: meminfo
    let mem_total_kb = read_meminfo_total(fs);

    // Step 6: cmdline
    let cmdline = fs.read_file("/proc/cmdline")
        .unwrap_or_default()
        .trim_end_matches('\n')
        .to_string();

    // Step 7: loadavg
    let loadavg = fs.read_file("/proc/loadavg")
        .unwrap_or_default()
        .trim_end_matches('\n')
        .to_string();

    // Step 8: uptime
    let (uptime_min, idletime_min) = read_uptime(fs);

    // Step 9: emit records in order
    elements.push(GeneralInfoRecord { key: "hostname".to_string(),     value: hostname });
    elements.push(GeneralInfoRecord { key: "os_release".to_string(),   value: os_release });
    elements.push(GeneralInfoRecord { key: "uname".to_string(),        value: uname });
    elements.push(GeneralInfoRecord { key: "collected_at".to_string(), value: collected_at });
    elements.push(GeneralInfoRecord { key: "mem_total_kb".to_string(), value: mem_total_kb.to_string() });
    elements.push(GeneralInfoRecord { key: "cmdline".to_string(),      value: cmdline });
    elements.push(GeneralInfoRecord { key: "loadavg".to_string(),      value: loadavg });
    elements.push(GeneralInfoRecord { key: "uptime_min".to_string(),   value: uptime_min.to_string() });
    elements.push(GeneralInfoRecord { key: "idletime_min".to_string(), value: idletime_min.to_string() });

    ScopeWrapper { attributes: Default::default(), elements }
}

/// collect-environment BEHAVIOR
pub fn collect_environment(fs: &dyn Filesystem) -> EnvironmentScope {
    // Step 1: detect locale
    let locale = detect_locale(fs);

    // Step 2: detect system_type
    let system_type = detect_system_type(fs);

    EnvironmentScope { locale, system_type }
}

/// collect-os BEHAVIOR
pub fn collect_os(
    fs: &dyn Filesystem,
    dist: &DistributionInfo,
    cr: &dyn CommandRunner,
) -> OsScope {
    // Step 1: read /etc/os-release
    let (name, version) = if fs.exists("/etc/os-release") {
        let content = fs.read_file("/etc/os-release").unwrap_or_default();
        let name = parse_os_release_field(&content, "NAME")
            .or_else(|| parse_os_release_field(&content, "PRETTY_NAME"));
        let version = parse_os_release_field(&content, "VERSION")
            .or_else(|| parse_os_release_field(&content, "VERSION_ID"));
        (name, version)
    } else {
        // Step 2: fallback to detect-distribution result
        if !dist.release.is_empty() {
            (Some(dist.release.clone()), None)
        } else {
            (None, None)
        }
    };

    // Step 3: architecture from uname -m
    let architecture = cr.run("uname", &["-m"])
        .ok()
        .map(|(out, _)| out.trim().to_string())
        .filter(|s| !s.is_empty());

    OsScope { name, version, architecture }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn get_hostname(cr: &dyn CommandRunner) -> String {
    cr.run("hostname", &["-f"])
        .map(|(out, _)| out.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

pub fn get_uname(cr: &dyn CommandRunner) -> String {
    cr.run("uname", &["-a"])
        .map(|(out, _)| out.trim().to_string())
        .unwrap_or_else(|_| String::new())
}

fn utc_now() -> String {
    // RFC3339 / ISO 8601 UTC timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_utc_timestamp(secs)
}

pub fn format_utc_timestamp(secs: u64) -> String {
    // Simple UTC formatter — no external dependency
    let s = secs as i64;
    let (mut year, mut month, mut day, mut hour, mut min, mut sec_r) = (1970i32, 1u32, 1u32, 0u32, 0u32, 0u32);
    let mut rem = s;
    // days since epoch
    let days = rem / 86400;
    rem %= 86400;
    hour = (rem / 3600) as u32;
    rem %= 3600;
    min = (rem / 60) as u32;
    sec_r = (rem % 60) as u32;

    // Convert days to Y-M-D (Gregorian calendar)
    let mut d = days as i32;
    year = 1970;
    loop {
        let leap = is_leap(year);
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year { break; }
        d -= days_in_year;
        year += 1;
    }
    let months = [31, if is_leap(year) { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    month = 1;
    for &m in &months {
        if d < m { break; }
        d -= m;
        month += 1;
    }
    day = (d + 1) as u32;

    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, min, sec_r)
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

fn local_time_string() -> String {
    // Return UTC time as local time string (no timezone library)
    utc_now()
}

fn read_meminfo_total(fs: &dyn Filesystem) -> u64 {
    let content = fs.read_file("/proc/meminfo").unwrap_or_default();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            return kb;
        }
    }
    0
}

fn read_uptime(fs: &dyn Filesystem) -> (u64, u64) {
    let content = fs.read_file("/proc/uptime").unwrap_or_default();
    let mut parts = content.split_whitespace();
    let uptime_sec: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let idle_sec:   f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    ((uptime_sec / 60.0).floor() as u64, (idle_sec / 60.0).floor() as u64)
}

fn detect_locale(fs: &dyn Filesystem) -> String {
    let sources = [
        "/etc/locale.conf",
        "/etc/default/locale",
        "/etc/sysconfig/language",
    ];
    for src in &sources {
        if let Ok(content) = fs.read_file(src) {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("LANG=")
                    .or_else(|| line.strip_prefix("LC_ALL="))
                    .or_else(|| line.strip_prefix("RC_LANG="))  // SUSE
                {
                    let v = rest.trim().trim_matches('"').to_string();
                    if !v.is_empty() {
                        return v;
                    }
                }
            }
        }
    }
    eprintln!("sitar: all locale sources unreadable; defaulting to C");
    "C".to_string()
}

fn detect_system_type(fs: &dyn Filesystem) -> String {
    if fs.exists("/.dockerenv") {
        return "docker".to_string();
    }
    if let Ok(cgroup) = fs.read_file("/proc/1/cgroup") {
        if cgroup.contains("docker") {
            return "docker".to_string();
        }
    }
    if let Ok(environ) = fs.read_file("/proc/1/environ") {
        if environ.contains("container") {
            return "remote".to_string();
        }
    }
    "local".to_string()
}

fn libc_geteuid() -> u32 {
    unsafe {
        extern "C" { fn geteuid() -> u32; }
        geteuid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::{FakeFilesystem, FakeCommandRunner};
    use crate::types::DistributionFamily;

    fn make_dist() -> DistributionInfo {
        DistributionInfo {
            family:      DistributionFamily::Unknown,
            release:     "Test 1.0".to_string(),
            backend:     crate::types::PackageVersioningBackend::None,
            rpm_cmd:     String::new(),
            dpkg_status: String::new(),
        }
    }

    #[test]
    fn test_collect_general_info_nine_records() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/proc/meminfo".to_string(), "MemTotal:       8000000 kB\n".to_string());
        fs.files.insert("/proc/cmdline".to_string(), "BOOT_IMAGE=/vmlinuz\n".to_string());
        fs.files.insert("/proc/loadavg".to_string(), "0.10 0.05 0.01 1/100 1234\n".to_string());
        fs.files.insert("/proc/uptime".to_string(), "3600.00 7200.00\n".to_string());

        let mut cr = FakeCommandRunner::new();
        cr.responses.insert("hostname".to_string(), ("myhost.example.com\n".to_string(), String::new()));
        cr.responses.insert("uname".to_string(), ("Linux myhost 5.15.0\n".to_string(), String::new()));

        let dist = make_dist();
        let scope = collect_general_info(&fs, &cr, &dist);
        assert_eq!(scope.elements.len(), 9);
        assert_eq!(scope.elements[0].key, "hostname");
        assert_eq!(scope.elements[0].value, "myhost.example.com");
        assert_eq!(scope.elements[2].key, "uname");
        assert!(!scope.elements[2].value.is_empty());
    }

    #[test]
    fn test_collect_environment_locale() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/locale.conf".to_string(), "LANG=en_US.UTF-8\n".to_string());
        let env = collect_environment(&fs);
        assert_eq!(env.locale, "en_US.UTF-8");
        assert_eq!(env.system_type, "local");
    }

    #[test]
    fn test_collect_environment_docker() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/.dockerenv".to_string(), String::new());
        let env = collect_environment(&fs);
        assert_eq!(env.system_type, "docker");
    }

    #[test]
    fn test_collect_os_from_os_release() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/os-release".to_string(),
            "NAME=\"SUSE Linux Enterprise Server 15 SP6\"\nVERSION=\"15-SP6\"\n".to_string());
        let mut cr = FakeCommandRunner::new();
        cr.responses.insert("uname".to_string(), ("x86_64\n".to_string(), String::new()));
        let dist = make_dist();
        let os = collect_os(&fs, &dist, &cr);
        assert_eq!(os.name, Some("SUSE Linux Enterprise Server 15 SP6".to_string()));
        assert_eq!(os.version, Some("15-SP6".to_string()));
        assert_eq!(os.architecture, Some("x86_64".to_string()));
    }

    #[test]
    fn test_format_utc_timestamp() {
        // Unix epoch
        let s = format_utc_timestamp(0);
        assert_eq!(s, "1970-01-01T00:00:00Z");
        // 2026-04-03T22:56:38Z ≈ 1775254598
        let s2 = format_utc_timestamp(86400);
        assert_eq!(s2, "1970-01-02T00:00:00Z");
    }
}
