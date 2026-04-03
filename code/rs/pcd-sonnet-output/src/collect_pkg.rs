// collect_pkg.rs — Package, service, user, group collection (M5)
// BEHAVIORs: collect-installed-rpm, collect-installed-deb, collect-repositories,
//            collect-services, collect-chkconfig, collect-groups, collect-users,
//            collect-changed-config-files, collect-changed-managed-files, collect-kernel-config
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashMap;
use crate::interfaces::{Filesystem, CommandRunner};
use crate::types::*;

// ---------------------------------------------------------------------------
// collect-installed-rpm
// ---------------------------------------------------------------------------

pub fn collect_installed_rpm(cr: &dyn CommandRunner) -> (ScopeWrapper<PackageRecord>, ScopeWrapper<PatternRecord>) {
    let fmt = "%{NAME}::%{VERSION}-%{RELEASE}::%{SIZE}::%{SUMMARY}::%{DISTRIBUTION}::%{PACKAGER}::%{ARCH}::%{VENDOR}::%{MD5SUM}\\n";
    let output = match cr.run("rpm", &["-qa", "--queryformat", fmt]) {
        Ok((o, _)) => o,
        Err(e) => {
            eprintln!("sitar: collect-installed-rpm: rpm -qa failed: {}", e);
            let mut scope = ScopeWrapper::<PackageRecord>::default();
            scope.attributes.insert("package_system".to_string(),
                serde_json::Value::String("rpm".to_string()));
            return (scope, ScopeWrapper::default());
        }
    };

    let mut records = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.splitn(9, "::").collect();
        if parts.len() < 9 { continue; }
        let name    = parts[0].to_string();
        let ver_rel = parts[1].to_string();
        let (version, release) = split_version_release(&ver_rel);
        let size: i64 = parts[2].parse().unwrap_or(0);
        let summary  = parts[3].to_string();
        let dist     = parts[4].to_string();
        let packager = parts[5].to_string();
        let arch     = parts[6].to_string();
        let vendor   = parts[7].to_string();
        let checksum = parts[8].trim().to_string();

        records.push(PackageRecord {
            name, version, release, arch, vendor, checksum,
            size, summary, distribution: dist, packager,
        });
    }

    records.sort_by(|a, b| a.name.cmp(&b.name));

    let mut pkg_scope = ScopeWrapper { attributes: Default::default(), elements: records };
    pkg_scope.attributes.insert("package_system".to_string(),
        serde_json::Value::String("rpm".to_string()));

    // Patterns (zypp)
    let pat_scope = collect_patterns_rpm(cr);

    (pkg_scope, pat_scope)
}

fn split_version_release(ver_rel: &str) -> (String, String) {
    if let Some(pos) = ver_rel.rfind('-') {
        (ver_rel[..pos].to_string(), ver_rel[pos+1..].to_string())
    } else {
        (ver_rel.to_string(), String::new())
    }
}

fn collect_patterns_rpm(cr: &dyn CommandRunner) -> ScopeWrapper<PatternRecord> {
    // Try zypper patterns first
    let output = cr.run("zypper", &["patterns", "--installed-only"])
        .map(|(o, _)| o)
        .unwrap_or_default();

    let mut records = Vec::new();
    // Parse zypper patterns output: lines with | separator
    for line in output.lines() {
        if !line.contains('|') { continue; }
        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if parts.len() < 4 { continue; }
        // Skip header/separator lines
        if parts[0] == "S" || parts[0].starts_with('-') { continue; }
        let name    = parts[2].to_string();
        let version = parts[3].to_string();
        if name.is_empty() { continue; }
        records.push(PatternRecord { name, version, release: String::new() });
    }

    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("patterns_system".to_string(),
        serde_json::Value::String("zypper".to_string()));
    scope
}

// ---------------------------------------------------------------------------
// collect-installed-deb
// ---------------------------------------------------------------------------

pub fn collect_installed_deb(fs: &dyn Filesystem, status_path: &str) -> ScopeWrapper<PackageRecord> {
    let content = match fs.read_file(status_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sitar: collect-installed-deb: {} unreadable: {}", status_path, e);
            let mut scope = ScopeWrapper::<PackageRecord>::default();
            scope.attributes.insert("package_system".to_string(),
                serde_json::Value::String("dpkg".to_string()));
            return scope;
        }
    };

    let mut records = Vec::new();
    let mut current: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        if line.is_empty() {
            // End of record
            if let Some(status) = current.get("Status") {
                if status.ends_with("installed") {
                    let name    = current.get("Package").cloned().unwrap_or_default();
                    let version = current.get("Version").cloned().unwrap_or_default();
                    let arch    = current.get("Architecture").cloned().unwrap_or_default();
                    let summary = current.get("Description").cloned().unwrap_or_default();
                    let size_kb: i64 = current.get("Installed-Size")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let size = size_kb * 1024;

                    if !name.is_empty() {
                        records.push(PackageRecord {
                            name, version, release: String::new(),
                            arch, vendor: String::new(), checksum: String::new(),
                            size, summary,
                            distribution: String::new(), packager: String::new(),
                        });
                    }
                }
            }
            current.clear();
        } else if let Some(pos) = line.find(": ") {
            let key = line[..pos].to_string();
            let val = line[pos+2..].to_string();
            // Only take first line of multi-line fields
            current.entry(key).or_insert(val);
        }
    }

    // Handle last record
    if let Some(status) = current.get("Status") {
        if status.ends_with("installed") {
            let name    = current.get("Package").cloned().unwrap_or_default();
            let version = current.get("Version").cloned().unwrap_or_default();
            let arch    = current.get("Architecture").cloned().unwrap_or_default();
            let summary = current.get("Description").cloned().unwrap_or_default();
            let size_kb: i64 = current.get("Installed-Size")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let size = size_kb * 1024;
            if !name.is_empty() {
                records.push(PackageRecord {
                    name, version, release: String::new(),
                    arch, vendor: String::new(), checksum: String::new(),
                    size, summary,
                    distribution: String::new(), packager: String::new(),
                });
            }
        }
    }

    records.sort_by(|a, b| a.name.cmp(&b.name));

    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("package_system".to_string(),
        serde_json::Value::String("dpkg".to_string()));
    scope
}

// ---------------------------------------------------------------------------
// collect-repositories
// ---------------------------------------------------------------------------

pub fn collect_repositories(
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
    backend: &PackageVersioningBackend,
) -> Option<ScopeWrapper<RepositoryRecord>> {
    // zypp repos.d
    for repos_dir in &["/etc/zypp/repos.d", "/etc/zypper/repos.d"] {
        if fs.is_dir(repos_dir) {
            return Some(collect_zypp_repos(fs, repos_dir));
        }
    }

    // yum repos.d
    if fs.is_dir("/etc/yum.repos.d") {
        return Some(collect_yum_repos(fs, "/etc/yum.repos.d"));
    }

    // apt sources.list
    if fs.exists("/etc/apt/sources.list") || fs.is_dir("/etc/apt/sources.list.d") {
        return Some(collect_apt_repos(fs));
    }

    None
}

fn collect_zypp_repos(fs: &dyn Filesystem, dir: &str) -> ScopeWrapper<RepositoryRecord> {
    let mut records = Vec::new();
    let entries = fs.read_dir(dir).unwrap_or_default();
    for entry in entries {
        if !entry.ends_with(".repo") { continue; }
        let content = match fs.read_file(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let rec = parse_ini_repo(&content, "zypp");
        if !rec.name.is_empty() || !rec.alias.is_empty() {
            records.push(rec);
        }
    }
    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("repository_system".to_string(),
        serde_json::Value::String("zypp".to_string()));
    scope
}

fn collect_yum_repos(fs: &dyn Filesystem, dir: &str) -> ScopeWrapper<RepositoryRecord> {
    let mut records = Vec::new();
    let entries = fs.read_dir(dir).unwrap_or_default();
    for entry in entries {
        if !entry.ends_with(".repo") { continue; }
        let content = match fs.read_file(&entry) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let rec = parse_ini_repo(&content, "yum");
        if !rec.name.is_empty() || !rec.alias.is_empty() {
            records.push(rec);
        }
    }
    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("repository_system".to_string(),
        serde_json::Value::String("yum".to_string()));
    scope
}

fn parse_ini_repo(content: &str, _system: &str) -> RepositoryRecord {
    let mut rec = RepositoryRecord::default();
    let mut section = String::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len()-1].to_string();
            if rec.alias.is_empty() { rec.alias = section.clone(); }
            continue;
        }
        if line.starts_with('#') || line.is_empty() { continue; }
        if let Some(pos) = line.find('=') {
            let k = line[..pos].trim().to_lowercase();
            let v = line[pos+1..].trim().to_string();
            match k.as_str() {
                "name"        => rec.name        = v,
                "baseurl" | "url" => rec.url     = v,
                "type"        => rec.r#type       = v,
                "enabled"     => rec.enabled      = v == "1" || v == "true",
                "gpgcheck"    => rec.gpgcheck     = v == "1" || v == "true",
                "autorefresh" => rec.autorefresh  = v == "1" || v == "true",
                "priority"    => rec.priority     = v.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    rec
}

fn collect_apt_repos(fs: &dyn Filesystem) -> ScopeWrapper<RepositoryRecord> {
    let mut records = Vec::new();
    let mut sources = Vec::new();

    if let Ok(content) = fs.read_file("/etc/apt/sources.list") {
        sources.push(content);
    }
    if let Ok(entries) = fs.read_dir("/etc/apt/sources.list.d") {
        for entry in entries {
            if let Ok(content) = fs.read_file(&entry) {
                sources.push(content);
            }
        }
    }

    for content in &sources {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() { continue; }
            // deb http://... distribution components...
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 3 { continue; }
            let repo_type = parts[0].to_string();
            let url = parts[1].to_string();
            let distribution = parts[2].to_string();
            let components: Vec<String> = parts[3..].iter().map(|s| s.to_string()).collect();
            records.push(RepositoryRecord {
                r#type: repo_type,
                url,
                distribution,
                components,
                ..Default::default()
            });
        }
    }

    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("repository_system".to_string(),
        serde_json::Value::String("apt".to_string()));
    scope
}

// ---------------------------------------------------------------------------
// collect-services / collect-chkconfig
// ---------------------------------------------------------------------------

pub fn collect_chkconfig(cr: &dyn CommandRunner) -> Option<ScopeWrapper<ServiceRecord>> {
    // Detect init system
    let is_systemd = std::path::Path::new("/run/systemd/private").exists()
        || std::path::Path::new("/run/systemd").exists();

    if is_systemd {
        return collect_systemd_services(cr);
    }

    // Try systemctl anyway
    if let Ok((output, _)) = cr.run("systemctl", &["list-unit-files", "--type=service", "--no-legend"]) {
        if !output.is_empty() {
            return Some(parse_systemctl_output(&output, "systemd"));
        }
    }

    // chkconfig fallback
    if let Ok((output, _)) = cr.run("chkconfig", &["--list"]) {
        return Some(parse_chkconfig_output(&output));
    }

    None
}

fn collect_systemd_services(cr: &dyn CommandRunner) -> Option<ScopeWrapper<ServiceRecord>> {
    let output = cr.run("systemctl", &["list-unit-files", "--type=service", "--no-legend"])
        .map(|(o, _)| o)
        .ok()?;
    Some(parse_systemctl_output(&output, "systemd"))
}

fn parse_systemctl_output(output: &str, init_system: &str) -> ScopeWrapper<ServiceRecord> {
    let mut records = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 { continue; }
        let name  = parts[0].trim_end_matches(".service").to_string();
        let state = parts[1].to_string();
        if name.is_empty() { continue; }
        records.push(ServiceRecord { name, state, legacy_sysv: false });
    }
    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("init_system".to_string(),
        serde_json::Value::String(init_system.to_string()));
    scope
}

fn parse_chkconfig_output(output: &str) -> ScopeWrapper<ServiceRecord> {
    let mut records = Vec::new();
    for line in output.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() { continue; }
        let name = parts[0].to_string();
        // Check runlevel 3 or 5 for enabled
        let state = parts.iter().skip(1).any(|p| {
            (p.starts_with("3:on") || p.starts_with("5:on"))
        });
        records.push(ServiceRecord {
            name,
            state: if state { "enabled".to_string() } else { "disabled".to_string() },
            legacy_sysv: false,
        });
    }
    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("init_system".to_string(),
        serde_json::Value::String("sysvinit".to_string()));
    scope
}

// ---------------------------------------------------------------------------
// collect-groups
// ---------------------------------------------------------------------------

pub fn collect_groups(fs: &dyn Filesystem) -> ScopeWrapper<GroupRecord> {
    let content = match fs.read_file("/etc/group") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sitar: collect-groups: /etc/group unreadable: {}", e);
            return ScopeWrapper::default();
        }
    };

    let mut records = Vec::new();
    for line in content.lines() {
        if line.starts_with('#') || line.is_empty() { continue; }
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 3 { continue; }
        let name     = parts[0].to_string();
        let password = parts[1].to_string();
        let gid: Option<i64> = parts[2].parse().ok();
        let users: Vec<String> = if parts.len() >= 4 && !parts[3].is_empty() {
            parts[3].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        } else {
            Vec::new()
        };
        records.push(GroupRecord { name, password, gid, users });
    }

    records.sort_by(|a, b| a.name.cmp(&b.name));
    ScopeWrapper { attributes: Default::default(), elements: records }
}

// ---------------------------------------------------------------------------
// collect-users
// ---------------------------------------------------------------------------

pub fn collect_users(fs: &dyn Filesystem, config: &Config) -> ScopeWrapper<UserRecord> {
    let content = match fs.read_file("/etc/passwd") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sitar: collect-users: /etc/passwd unreadable: {}", e);
            std::process::exit(1);
        }
    };

    let mut user_map: HashMap<String, UserRecord> = HashMap::new();

    for line in content.lines() {
        if line.starts_with('#') || line.is_empty() { continue; }
        let parts: Vec<&str> = line.splitn(7, ':').collect();
        if parts.len() < 7 { continue; }
        let name    = parts[0].to_string();
        let password = parts[1].to_string();
        let uid: Option<i64> = parts[2].parse().ok();
        let gid: Option<i64> = parts[3].parse().ok();
        let comment = parts[4].to_string();
        let home    = parts[5].to_string();
        let shell   = parts[6].to_string();
        user_map.insert(name.clone(), UserRecord {
            name, password, uid, gid, comment, home, shell,
            ..Default::default()
        });
    }

    // Shadow file — only if not in config.exclude
    let shadow_path = "/etc/shadow";
    if !config.exclude.iter().any(|e| e == shadow_path) {
        if let Ok(shadow) = fs.read_file(shadow_path) {
            for line in shadow.lines() {
                if line.starts_with('#') || line.is_empty() { continue; }
                let parts: Vec<&str> = line.splitn(9, ':').collect();
                if parts.len() < 2 { continue; }
                let name = parts[0].to_string();
                if let Some(user) = user_map.get_mut(&name) {
                    user.encrypted_password = parts[1].to_string();
                    if parts.len() > 2 { user.last_changed_date = parts[2].parse().unwrap_or(0); }
                    if parts.len() > 3 { user.min_days          = parts[3].parse().unwrap_or(0); }
                    if parts.len() > 4 { user.max_days          = parts[4].parse().unwrap_or(0); }
                    if parts.len() > 5 { user.warn_days         = parts[5].parse().unwrap_or(0); }
                    if parts.len() > 6 { user.disable_days      = parts[6].parse().unwrap_or(0); }
                    if parts.len() > 7 { user.disabled_date     = parts[7].parse().unwrap_or(0); }
                }
            }
        }
    }

    let mut records: Vec<UserRecord> = user_map.into_values().collect();
    records.sort_by(|a, b| a.name.cmp(&b.name));
    ScopeWrapper { attributes: Default::default(), elements: records }
}

// ---------------------------------------------------------------------------
// collect-changed-config-files
// ---------------------------------------------------------------------------

pub fn collect_changed_config_files(
    cr: &dyn CommandRunner,
) -> Option<ScopeWrapper<ChangedConfigFileRecord>> {
    // Get all config files with owning packages
    let output = cr.run("rpm", &["-qca", "--queryformat", "%{NAME}\n"])
        .map(|(o, _)| o)
        .ok()?;

    let packages: std::collections::HashSet<String> = output.lines()
        .map(|l| l.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let mut records = Vec::new();

    for pkg in &packages {
        let verify_out = match cr.run("rpm", &["-V", "--nodeps", "--noscript", pkg]) {
            Ok((o, _)) => o,
            Err(e) => {
                records.push(ChangedConfigFileRecord {
                    package_name: pkg.clone(),
                    status: "error".to_string(),
                    error_message: e.to_string(),
                    ..Default::default()
                });
                continue;
            }
        };

        for line in verify_out.lines() {
            // Format: SM5DLUGTP  c /path
            if line.len() < 12 { continue; }
            let flags = &line[..9];
            let file_type = line.chars().nth(10).unwrap_or(' ');
            if file_type != 'c' { continue; } // only config files
            let path = line[12..].trim().to_string();

            let changes = parse_rpm_verify_flags(flags);
            records.push(ChangedConfigFileRecord {
                name: path,
                package_name: pkg.clone(),
                status: "changed".to_string(),
                changes,
                ..Default::default()
            });
        }
    }

    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("extracted".to_string(), serde_json::Value::Bool(false));
    Some(scope)
}

fn parse_rpm_verify_flags(flags: &str) -> Vec<String> {
    let flag_map = [
        ('S', "size"), ('M', "mode"), ('5', "md5"), ('D', "device_number"),
        ('L', "link_path"), ('U', "user"), ('G', "group"), ('T', "time"),
        ('P', "capabilities"),
    ];
    let mut changes = Vec::new();
    for (i, (ch, name)) in flag_map.iter().enumerate() {
        if let Some(c) = flags.chars().nth(i) {
            if c != '.' && c != '?' {
                changes.push(name.to_string());
            }
        }
    }
    changes
}

// ---------------------------------------------------------------------------
// collect-changed-managed-files
// ---------------------------------------------------------------------------

pub fn collect_changed_managed_files(
    cr: &dyn CommandRunner,
) -> Option<ScopeWrapper<ChangedManagedFileRecord>> {
    let output = cr.run("rpm", &["-Va", "--nodeps", "--noscript"])
        .map(|(o, _)| o)
        .ok()?;

    let mut records = Vec::new();
    for line in output.lines() {
        if line.len() < 12 { continue; }
        let flags = &line[..9];
        let file_type = line.chars().nth(10).unwrap_or(' ');
        if file_type == 'c' { continue; } // skip config files
        let path = line[12..].trim().to_string();
        let changes = parse_rpm_verify_flags(flags);
        records.push(ChangedManagedFileRecord {
            name: path,
            status: "changed".to_string(),
            changes,
            ..Default::default()
        });
    }

    let mut scope = ScopeWrapper { attributes: Default::default(), elements: records };
    scope.attributes.insert("extracted".to_string(), serde_json::Value::Bool(false));
    Some(scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::{FakeFilesystem, FakeCommandRunner};

    #[test]
    fn test_collect_installed_deb() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/var/lib/dpkg/status".to_string(), "\
Package: bash
Status: install ok installed
Architecture: amd64
Version: 5.1-6
Installed-Size: 1234
Description: GNU Bourne Again SHell

Package: curl
Status: install ok installed
Architecture: amd64
Version: 7.88.1
Installed-Size: 567
Description: command line tool for transferring data

Package: removed-pkg
Status: deinstall ok config-files
Architecture: amd64
Version: 1.0
Description: Should not appear

".to_string());
        let scope = collect_installed_deb(&fs, "/var/lib/dpkg/status");
        assert_eq!(scope.elements.len(), 2);
        assert_eq!(scope.elements[0].name, "bash");
        assert_eq!(scope.elements[1].name, "curl");
        assert_eq!(scope.attributes.get("package_system").and_then(|v| v.as_str()), Some("dpkg"));
    }

    #[test]
    fn test_collect_groups() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/group".to_string(), "\
root:x:0:
daemon:x:1:
sudo:x:27:alice,bob
".to_string());
        let scope = collect_groups(&fs);
        assert_eq!(scope.elements.len(), 3);
        let sudo = scope.elements.iter().find(|g| g.name == "sudo").unwrap();
        assert_eq!(sudo.gid, Some(27));
        assert!(sudo.users.contains(&"alice".to_string()));
    }

    #[test]
    fn test_collect_users_shadow_excluded() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/passwd".to_string(), "root:x:0:0:root:/root:/bin/bash\nalice:x:1000:1000:Alice:/home/alice:/bin/bash\n".to_string());
        let config = Config::default(); // /etc/shadow excluded by default
        let scope = collect_users(&fs, &config);
        assert_eq!(scope.elements.len(), 2);
        // Shadow not read
        for user in &scope.elements {
            assert!(user.encrypted_password.is_empty());
        }
    }

    #[test]
    fn test_parse_systemctl_output() {
        let output = "sshd.service    enabled\nnginx.service   disabled\ncron.service    static\n";
        let scope = parse_systemctl_output(output, "systemd");
        assert_eq!(scope.elements.len(), 3);
        assert_eq!(scope.elements[0].name, "sshd");
        assert_eq!(scope.elements[0].state, "enabled");
    }

    #[test]
    fn test_split_version_release() {
        let (v, r) = split_version_release("1.2.3-4.el9");
        assert_eq!(v, "1.2.3");
        assert_eq!(r, "4.el9");
        let (v2, r2) = split_version_release("1.2.3");
        assert_eq!(v2, "1.2.3");
        assert_eq!(r2, "");
    }
}
