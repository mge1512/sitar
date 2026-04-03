// detect.rs — detect-distribution BEHAVIOR
#![allow(dead_code)]

use crate::interfaces::Filesystem;
use crate::types::{DistributionFamily, DistributionInfo, PackageVersioningBackend};

/// Determine the Linux distribution family and select the appropriate
/// package versioning backend. STEPS order is the precedence order.
pub fn detect_distribution(fs: &dyn Filesystem) -> DistributionInfo {
    // Step 1: Debian family
    if fs.exists("/etc/debian_version") {
        let release = fs.read_file("/etc/debian_version")
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        return DistributionInfo {
            family:      DistributionFamily::Deb,
            release,
            backend:     PackageVersioningBackend::Dpkg,
            rpm_cmd:     String::new(),
            dpkg_status: "/var/lib/dpkg/status".to_string(),
        };
    }

    // Step 2: Red Hat family
    if fs.exists("/etc/redhat-release") {
        let release = fs.read_file("/etc/redhat-release")
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        return DistributionInfo {
            family:      DistributionFamily::Rpm,
            release,
            backend:     PackageVersioningBackend::Rpm,
            rpm_cmd:     "/bin/rpm".to_string(),
            dpkg_status: String::new(),
        };
    }

    // Step 3: UnitedLinux AND SuSE-release
    if fs.exists("/etc/UnitedLinux-release") && fs.exists("/etc/SuSE-release") {
        let ul = fs.read_file("/etc/UnitedLinux-release")
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let suse = fs.read_file("/etc/SuSE-release")
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let release = format!("{}, {}", ul, suse);
        return DistributionInfo {
            family:      DistributionFamily::Rpm,
            release,
            backend:     PackageVersioningBackend::Rpm,
            rpm_cmd:     "/bin/rpm".to_string(),
            dpkg_status: String::new(),
        };
    }

    // Step 4: UnitedLinux only
    if fs.exists("/etc/UnitedLinux-release") {
        let release = fs.read_file("/etc/UnitedLinux-release")
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        return DistributionInfo {
            family:      DistributionFamily::Rpm,
            release,
            backend:     PackageVersioningBackend::Rpm,
            rpm_cmd:     "/bin/rpm".to_string(),
            dpkg_status: String::new(),
        };
    }

    // Step 5: SLOX
    if fs.exists("/etc/SLOX-release") {
        let release = fs.read_file("/etc/SLOX-release")
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        return DistributionInfo {
            family:      DistributionFamily::Rpm,
            release,
            backend:     PackageVersioningBackend::Rpm,
            rpm_cmd:     "/bin/rpm".to_string(),
            dpkg_status: String::new(),
        };
    }

    // Step 6: SuSE-release
    if fs.exists("/etc/SuSE-release") {
        let release = fs.read_file("/etc/SuSE-release")
            .unwrap_or_default()
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        return DistributionInfo {
            family:      DistributionFamily::Rpm,
            release,
            backend:     PackageVersioningBackend::Rpm,
            rpm_cmd:     "/bin/rpm".to_string(),
            dpkg_status: String::new(),
        };
    }

    // Step 7: /etc/os-release
    if fs.exists("/etc/os-release") {
        let content = fs.read_file("/etc/os-release").unwrap_or_default();
        let release = parse_os_release_field(&content, "PRETTY_NAME")
            .unwrap_or_else(|| "Linux".to_string());
        return DistributionInfo {
            family:      DistributionFamily::Rpm,
            release,
            backend:     PackageVersioningBackend::Rpm,
            rpm_cmd:     "/usr/bin/rpm".to_string(),
            dpkg_status: String::new(),
        };
    }

    // Step 8: unknown
    eprintln!("sitar: distribution not supported");
    DistributionInfo {
        family:      DistributionFamily::Unknown,
        release:     String::new(),
        backend:     PackageVersioningBackend::None,
        rpm_cmd:     String::new(),
        dpkg_status: String::new(),
    }
}

/// Parse a KEY=VALUE or KEY="VALUE" line from /etc/os-release content.
pub fn parse_os_release_field(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(key) {
            if let Some(val) = rest.strip_prefix('=') {
                let v = val.trim().trim_matches('"').to_string();
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::FakeFilesystem;

    #[test]
    fn test_detect_debian() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/debian_version".to_string(), "12.0\n".to_string());
        let info = detect_distribution(&fs);
        assert_eq!(info.family, DistributionFamily::Deb);
        assert_eq!(info.backend, PackageVersioningBackend::Dpkg);
        assert_eq!(info.release, "12.0");
    }

    #[test]
    fn test_detect_redhat() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/redhat-release".to_string(), "Red Hat Enterprise Linux 9\n".to_string());
        let info = detect_distribution(&fs);
        assert_eq!(info.family, DistributionFamily::Rpm);
        assert_eq!(info.backend, PackageVersioningBackend::Rpm);
    }

    #[test]
    fn test_detect_os_release() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/etc/os-release".to_string(),
            "NAME=\"openSUSE Leap\"\nPRETTY_NAME=\"openSUSE Leap 15.5\"\n".to_string());
        let info = detect_distribution(&fs);
        assert_eq!(info.family, DistributionFamily::Rpm);
        assert_eq!(info.release, "openSUSE Leap 15.5");
    }

    #[test]
    fn test_detect_unknown() {
        let fs = FakeFilesystem::new();
        let info = detect_distribution(&fs);
        assert_eq!(info.family, DistributionFamily::Unknown);
    }
}
