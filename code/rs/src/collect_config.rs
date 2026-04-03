// collect_config.rs — Config files, AppArmor, kernel config, find-unpacked, check-consistency (M7)
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::interfaces::{Filesystem, CommandRunner};
use crate::types::*;

const SITAR_READFILE_LIMIT: usize = 32767;

// ---------------------------------------------------------------------------
// collect-security-apparmor
// ---------------------------------------------------------------------------

pub fn collect_security_apparmor(
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
    config: &Config,
) -> Option<SecurityApparmorScope> {
    // Step 1: locate AppArmor kernel interface
    let apparmor_paths = [
        "/proc/sys/kernel/security/apparmor",
        "/sys/kernel/security/apparmor",
    ];
    let base_path = apparmor_paths.iter().find(|p| fs.is_dir(p))?;

    let mut scope = SecurityApparmorScope::default();
    scope.kernel_params.attributes.insert(
        "apparmor_path".to_string(),
        serde_json::Value::String(base_path.to_string()),
    );

    // Step 2: walk the kernel path
    let entries = fs.read_dir(base_path).unwrap_or_default();
    for entry in entries {
        let content = match fs.read_file_limited(&entry, SITAR_READFILE_LIMIT) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let content = content.trim_end_matches('\n').to_string();
        if content.is_empty() { continue; }

        let key = entry[base_path.len()..].trim_start_matches('/').to_string();

        if key == "profiles" {
            // Parse profiles: name (mode)\n
            for line in content.lines() {
                if let Some(paren) = line.rfind('(') {
                    let name = line[..paren].trim().to_string();
                    let mode = line[paren+1..].trim_end_matches(')').trim().to_string();
                    scope.profiles.elements.push(ApparmorProfileRecord { name, mode });
                }
            }
        } else {
            scope.kernel_params.elements.push(ApparmorKernelRecord { key, value: content });
        }
    }

    // Step 3-5: config files
    for dir in &["/etc/apparmor", "/etc/apparmor.d", "/etc/subdomain", "/etc/subdomain.d"] {
        if fs.is_dir(dir) {
            if let Ok(entries) = fs.read_dir(dir) {
                for entry in entries {
                    scope.config_files.push(entry);
                }
            }
        }
    }

    if config.allsubdomain == "On"
        || (config.allsubdomain == "Auto"
            && !fs.exists("/var/lib/support/Configuration_Consistency.json")
            && !fs.exists("/var/lib/support/Find_Unpacked.json"))
    {
        // Already added above
    }

    Some(scope)
}

// ---------------------------------------------------------------------------
// collect-kernel-config
// ---------------------------------------------------------------------------

pub fn collect_kernel_config(
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
) -> Option<ScopeWrapper<KernelConfigRecord>> {
    let content = if fs.exists("/proc/config.gz") {
        // Try gzip -dc
        match cr.run("gzip", &["-dc", "/proc/config.gz"]) {
            Ok((out, _)) => out,
            Err(_) => return None,
        }
    } else {
        // Try /boot/config-$(uname -r)
        let uname = cr.run("uname", &["-r"])
            .map(|(o, _)| o.trim().to_string())
            .unwrap_or_default();
        if uname.is_empty() { return None; }
        let path = format!("/boot/config-{}", uname);
        match fs.read_file(&path) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("sitar: collect-kernel-config: neither /proc/config.gz nor {} readable", path);
                return None;
            }
        }
    };

    let mut records = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() { continue; }
        if let Some(pos) = line.find('=') {
            let key   = line[..pos].trim().to_string();
            let value = line[pos+1..].trim().to_string();
            records.push(KernelConfigRecord { key, value });
        }
    }

    records.sort_by(|a, b| a.key.cmp(&b.key));
    Some(ScopeWrapper { attributes: Default::default(), elements: records })
}

// ---------------------------------------------------------------------------
// collect-config-files
// ---------------------------------------------------------------------------

pub fn collect_config_files(
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
    config: &Config,
    consistency_cache_exists: bool,
    unpacked_cache_exists: bool,
) -> Vec<(String, String)> {
    // Returns list of (path, content) pairs
    let mut result = Vec::new();
    let mut included: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Step 1: determine effective source mode
    let use_hardcoded = match config.allconfigfiles.as_str() {
        "On"   => true,
        "Off"  => false,
        "Auto" | _ => !consistency_cache_exists && !unpacked_cache_exists,
    };

    if use_hardcoded {
        for path in CONFIG_FILE_PATTERNS {
            collect_config_file_path(fs, config, path, &mut result, &mut included);
        }
    }

    // Step 3: always include
    for path in ALWAYS_INCLUDE {
        collect_config_file_path(fs, config, path, &mut result, &mut included);
    }

    // Step 4: /var/lib/support/*.json
    if let Ok(entries) = fs.glob("/var/lib/support/*.json") {
        for entry in entries {
            if let Ok(content) = fs.read_file(&entry) {
                if let Ok(paths) = serde_json::from_str::<Vec<String>>(&content) {
                    for path in paths {
                        collect_config_file_path(fs, config, &path, &mut result, &mut included);
                    }
                }
            }
        }
    }

    // Step 5: allsysconfig
    let use_sysconfig = match config.allsysconfig.as_str() {
        "On"   => true,
        "Off"  => false,
        "Auto" | _ => !consistency_cache_exists && !unpacked_cache_exists,
    };
    if use_sysconfig {
        collect_sysconfig_files(fs, config, &mut result, &mut included);
    }

    // Step 7: crontabs
    collect_crontabs(fs, &mut result, &mut included);

    result
}

fn collect_config_file_path(
    fs: &dyn Filesystem,
    config: &Config,
    path: &str,
    result: &mut Vec<(String, String)>,
    included: &mut std::collections::HashSet<String>,
) {
    // Handle glob patterns
    let paths = if path.contains('*') {
        fs.glob(path).unwrap_or_default()
    } else {
        vec![path.to_string()]
    };

    for p in paths {
        if included.contains(&p) { continue; }
        if p.contains("/proc") { continue; }
        if config.exclude.iter().any(|e| e == &p) { continue; }
        if is_backup_file(&p) { continue; }
        if !config.gconf && p.starts_with("/etc/opt/gnome") { continue; }
        if !config.lvmarchive && p.starts_with("/etc/lvm/archive") { continue; }

        // Check file size
        if config.file_size_limit > 0 {
            if let Ok(info) = fs.stat(&p) {
                if info.size > config.file_size_limit { continue; }
            }
        }

        let content = match fs.read_file(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };

        included.insert(p.clone());

        // Password blanking
        let content = if needs_password_blanking(&p) {
            blank_passwords(&content)
        } else {
            content
        };

        result.push((p, content));
    }
}

fn is_backup_file(path: &str) -> bool {
    let name = path.split('/').last().unwrap_or(path);
    name.ends_with(".orig") || name.ends_with(".org") || name.ends_with(".ori")
        || name.ends_with(".bak") || name.ends_with(".bac")
        || name.ends_with('~') || name.starts_with('#')
}

fn needs_password_blanking(path: &str) -> bool {
    matches!(path,
        "/etc/pppoed.conf" | "/etc/grub.conf" | "/boot/grub/menu.lst"
        | "/etc/lilo.conf" | "/boot/grub2/grub.cfg"
    )
}

fn blank_passwords(content: &str) -> String {
    // Blank values matching [Pp]assword\s*=\s*\S+
    let mut result = String::new();
    for line in content.lines() {
        let lower = line.to_lowercase();
        if lower.contains("password") {
            if let Some(eq_pos) = line.find('=') {
                let before = &line[..eq_pos+1];
                result.push_str(before);
                result.push_str("***BLANKED***\n");
                continue;
            }
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

fn collect_sysconfig_files(
    fs: &dyn Filesystem,
    config: &Config,
    result: &mut Vec<(String, String)>,
    included: &mut std::collections::HashSet<String>,
) {
    walk_sysconfig(fs, config, "/etc/sysconfig", result, included);
}

fn walk_sysconfig(
    fs: &dyn Filesystem,
    config: &Config,
    dir: &str,
    result: &mut Vec<(String, String)>,
    included: &mut std::collections::HashSet<String>,
) {
    let entries = match fs.read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries {
        if fs.is_dir(&entry) {
            walk_sysconfig(fs, config, &entry, result, included);
        } else {
            // Check if text file (read first 512 bytes)
            if let Ok(preview) = fs.read_file_limited(&entry, 512) {
                if preview.bytes().all(|b| b.is_ascii() || b >= 0x80) {
                    collect_config_file_path(fs, config, &entry, result, included);
                }
            }
        }
    }
}

fn collect_crontabs(
    fs: &dyn Filesystem,
    result: &mut Vec<(String, String)>,
    included: &mut std::collections::HashSet<String>,
) {
    // /etc/crontab
    if !included.contains("/etc/crontab") {
        if let Ok(content) = fs.read_file("/etc/crontab") {
            included.insert("/etc/crontab".to_string());
            result.push(("/etc/crontab".to_string(), content));
        }
    }

    // Per-distribution user crontab dirs
    for dir in &[
        "/var/spool/cron/tabs",     // SUSE/UnitedLinux
        "/var/spool/cron",          // Red Hat
        "/var/spool/cron/crontabs", // Debian
    ] {
        if fs.is_dir(dir) {
            if let Ok(entries) = fs.read_dir(dir) {
                for entry in entries {
                    if included.contains(&entry) { continue; }
                    if let Ok(content) = fs.read_file(&entry) {
                        included.insert(entry.clone());
                        result.push((entry, content));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// find-unpacked
// ---------------------------------------------------------------------------

pub fn find_unpacked(
    cr: &dyn CommandRunner,
    config_dir: &str,
    cache_file: &str,
    search_root: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Step 1: get all RPM-owned files under search_root
    let (rpm_files_out, _) = cr.run("rpm", &["-qla"])?;
    let rpm_files: std::collections::HashSet<String> = rpm_files_out
        .lines()
        .filter(|l| l.starts_with(search_root))
        .map(|l| l.trim().to_string())
        .collect();

    // Step 2: walk search_root
    let all_files = walk_dir_recursive(cr, search_root);

    // Step 3: filter
    let mut unpacked: Vec<String> = all_files.into_iter()
        .filter(|f| !f.ends_with('~'))
        .filter(|f| !rpm_files.contains(f))
        .filter(|f| {
            // Check if binary via `file -p -b`
            if let Ok((file_out, _)) = cr.run("file", &["-p", "-b", f]) {
                !file_out.contains("Berkeley DB") && !file_out.contains("data")
            } else {
                true
            }
        })
        .collect();

    unpacked.sort();

    // Step 4-5: write cache
    let cache_path = write_json_cache(config_dir, cache_file, &unpacked)?;
    Ok(cache_path)
}

fn walk_dir_recursive(cr: &dyn CommandRunner, dir: &str) -> Vec<String> {
    // Use find command to walk directory
    cr.run("find", &[dir, "-type", "f"])
        .map(|(out, _)| out.lines().map(|l| l.trim().to_string()).collect())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// check-consistency
// ---------------------------------------------------------------------------

pub fn check_consistency(
    cr: &dyn CommandRunner,
    config_dir: &str,
    cache_file: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // Step 1: get all config files with owning packages
    let (output, _) = cr.run("rpm", &["-qca", "--queryformat", "%{NAME}\n"])?;

    let mut configfiles: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let packages: std::collections::HashSet<String> = output.lines()
        .map(|l| l.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Build configfiles map
    for pkg in &packages {
        if let Ok((files_out, _)) = cr.run("rpm", &["-qc", pkg]) {
            for file in files_out.lines() {
                let file = file.trim().to_string();
                if !file.is_empty() {
                    configfiles.insert(file, pkg.clone());
                }
            }
        }
    }

    // Step 3: verify each package
    let mut changed: Vec<String> = Vec::new();
    for pkg in &packages {
        if let Ok((verify_out, _)) = cr.run("rpm", &["-V", "--nodeps", "--noscript", pkg]) {
            for line in verify_out.lines() {
                if line.len() < 12 { continue; }
                let file_type = line.chars().nth(10).unwrap_or(' ');
                if file_type != 'c' { continue; }
                let path = line[12..].trim().to_string();
                if configfiles.contains_key(&path) && !path.starts_with("missing") {
                    changed.push(path);
                }
            }
        }
    }

    changed.sort();
    changed.dedup();

    let cache_path = write_json_cache(config_dir, cache_file, &changed)?;
    Ok(cache_path)
}

fn write_json_cache(
    config_dir: &str,
    cache_file: &str,
    items: &[String],
) -> Result<String, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(config_dir)?;
    let cache_path = format!("{}/{}", config_dir, cache_file);
    let json = serde_json::to_string_pretty(items)?;
    std::fs::write(&cache_path, json)?;
    Ok(cache_path)
}

// ---------------------------------------------------------------------------
// Config file pattern lists
// ---------------------------------------------------------------------------

const CONFIG_FILE_PATTERNS: &[&str] = &[
    // Authentication and identity
    "/etc/passwd", "/etc/group",
    "/etc/nsswitch.conf", "/etc/pam.d/*", "/etc/security/*",
    "/etc/krb5.conf",
    // Network
    "/etc/hosts", "/etc/hostname", "/etc/resolv.conf",
    "/etc/networks", "/etc/protocols", "/etc/services",
    "/etc/hosts.allow", "/etc/hosts.deny",
    "/etc/ntp.conf", "/etc/chrony.conf", "/etc/chrony.d/*",
    "/etc/network/interfaces", "/etc/network/interfaces.d/*",
    "/etc/netconfig", "/etc/gai.conf",
    // SSH
    "/etc/ssh/sshd_config", "/etc/ssh/ssh_config", "/etc/sshd_config",
    // DNS
    "/etc/named.conf", "/etc/bind/named.conf",
    // Mail
    "/etc/postfix/main.cf", "/etc/postfix/master.cf", "/etc/aliases",
    "/etc/sendmail.cf", "/etc/mail/sendmail.cf",
    // Web / proxy
    "/etc/apache2/httpd.conf", "/etc/apache2/apache2.conf",
    "/etc/httpd/conf/httpd.conf", "/etc/nginx/nginx.conf",
    "/etc/squid/squid.conf", "/etc/squid.conf",
    // File sharing
    "/etc/samba/smb.conf", "/etc/smb.conf",
    "/etc/exports", "/etc/exports.d/*",
    // Directory services
    "/etc/openldap/slapd.conf", "/etc/ldap/slapd.conf", "/etc/slapd.conf",
    "/etc/openldap/ldap.conf", "/etc/ldap/ldap.conf", "/etc/ldap.conf",
    // Logging
    "/etc/syslog.conf", "/etc/syslog-ng/syslog-ng.conf",
    "/etc/rsyslog.conf", "/etc/rsyslog.d/*",
    // Init and boot
    "/etc/inittab", "/etc/grub.conf", "/etc/lilo.conf",
    "/boot/grub/menu.lst", "/boot/grub2/grub.cfg",
    "/etc/default/grub", "/etc/grub2.cfg",
    // Cron
    "/etc/crontab", "/etc/cron.d/*", "/etc/cron.daily/*",
    "/etc/cron.weekly/*", "/etc/cron.monthly/*",
    // Printing
    "/etc/cups/cupsd.conf", "/etc/printcap",
    // Firewall
    "/etc/sysconfig/SuSEfirewall2", "/etc/sysconfig/iptables",
    // Misc system
    "/etc/fstab", "/etc/mtab", "/etc/mdadm.conf", "/etc/mdadm/mdadm.conf",
    "/etc/modules", "/etc/modprobe.d/*", "/etc/modprobe.conf",
    "/etc/sysctl.conf", "/etc/sysctl.d/*",
    "/etc/securetty", "/etc/shells", "/etc/environment",
    "/etc/profile", "/etc/profile.d/*.sh", "/etc/bashrc", "/etc/bash.bashrc",
    "/etc/login.defs", "/etc/logrotate.conf", "/etc/logrotate.d/*",
    "/etc/updatedb.conf", "/etc/ld.so.conf", "/etc/ld.so.conf.d/*",
];

const ALWAYS_INCLUDE: &[&str] = &[
    "/etc/ssh/sshd_config",
    "/etc/sshd_config",
    "/etc/named.conf",
    "/etc/bind/named.conf",
    "/etc/samba/smb.conf",
    "/etc/smb.conf",
    "/etc/openldap/slapd.conf",
    "/etc/ldap/slapd.conf",
    "/etc/slapd.conf",
    "/etc/openldap/ldap.conf",
    "/etc/ldap/ldap.conf",
    "/etc/ldap.conf",
    "/etc/aliases",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::{FakeFilesystem, FakeCommandRunner};

    #[test]
    fn test_collect_kernel_config_from_boot() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/boot/config-5.15.0".to_string(),
            "# Auto-generated config\nCONFIG_SMP=y\nCONFIG_X86=y\n# comment\nCONFIG_MODULES=y\n".to_string());
        let mut cr = FakeCommandRunner::new();
        cr.responses.insert("uname".to_string(), ("5.15.0\n".to_string(), String::new()));
        let scope = collect_kernel_config(&fs, &cr).unwrap();
        assert_eq!(scope.elements.len(), 3);
        // sorted
        assert_eq!(scope.elements[0].key, "CONFIG_MODULES");
        assert_eq!(scope.elements[0].value, "y");
    }

    #[test]
    fn test_collect_config_files_shadow_excluded() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/shadow".to_string(), "root:$6$...\n".to_string());
        let config = Config::default();
        let result = collect_config_files(&fs, &FakeCommandRunner::new(), &config, false, false);
        assert!(!result.iter().any(|(p, _)| p == "/etc/shadow"));
    }

    #[test]
    fn test_is_backup_file() {
        assert!(is_backup_file("/etc/foo.bak"));
        assert!(is_backup_file("/etc/foo~"));
        assert!(is_backup_file("/etc/foo.orig"));
        assert!(!is_backup_file("/etc/foo.conf"));
    }
}
