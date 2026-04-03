// collect_hw.rs — Hardware and kernel collection modules (M2)
// BEHAVIORs: collect-cpu, collect-kernel-params, collect-net-params,
//            collect-devices, collect-pci, collect-processes, collect-dmi
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashMap;
use crate::interfaces::{Filesystem, CommandRunner};
use crate::types::*;

const SITAR_READFILE_LIMIT: usize = 32767;

// ---------------------------------------------------------------------------
// collect-cpu
// ---------------------------------------------------------------------------

pub fn collect_cpu(fs: &dyn Filesystem, cr: &dyn CommandRunner) -> ScopeWrapper<CpuRecord> {
    let arch = cr.run("uname", &["-m"])
        .map(|(o, _)| o.trim().to_string())
        .unwrap_or_default();

    let content = match fs.read_file("/proc/cpuinfo") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("sitar: collect-cpu: /proc/cpuinfo unreadable: {}", e);
            return ScopeWrapper::default();
        }
    };

    let mut records = Vec::new();
    let mut attributes = HashMap::new();
    attributes.insert("architecture".to_string(),
        serde_json::Value::String(arch.clone()));

    if arch == "alpha" {
        let mut current = CpuRecord::default();
        let mut in_block = false;
        for line in content.lines() {
            if line.trim().is_empty() {
                if in_block {
                    records.push(current.clone());
                    current = CpuRecord::default();
                    in_block = false;
                }
                continue;
            }
            if let Some((k, v)) = split_cpuinfo_line(line) {
                if k == "cpus detected" {
                    in_block = true;
                    current.processor = v.to_string();
                } else if in_block {
                    apply_cpu_field(&mut current, &k, &v);
                }
            }
        }
        if in_block { records.push(current); }
    } else {
        let mut current: Option<CpuRecord> = None;
        for line in content.lines() {
            if line.trim().is_empty() {
                if let Some(rec) = current.take() {
                    records.push(rec);
                }
                continue;
            }
            if let Some((k, v)) = split_cpuinfo_line(line) {
                if k == "processor" {
                    if let Some(rec) = current.take() {
                        records.push(rec);
                    }
                    let mut rec = CpuRecord::default();
                    rec.processor = v.to_string();
                    current = Some(rec);
                } else if let Some(ref mut rec) = current {
                    apply_cpu_field(rec, &k, &v);
                }
            }
        }
        if let Some(rec) = current { records.push(rec); }
    }

    ScopeWrapper { attributes, elements: records }
}

fn split_cpuinfo_line(line: &str) -> Option<(String, String)> {
    let pos = line.find(':')?;
    let k = line[..pos].trim().to_lowercase();
    let v = line[pos+1..].trim().to_string();
    Some((k, v))
}

fn apply_cpu_field(rec: &mut CpuRecord, key: &str, val: &str) {
    match key {
        "vendor_id"  => rec.vendor_id  = val.to_string(),
        "model name" => rec.model_name = val.to_string(),
        "cpu family" => rec.cpu_family = val.to_string(),
        "model"      => rec.model      = val.to_string(),
        "stepping"   => rec.stepping   = val.to_string(),
        "cpu mhz"    => rec.cpu_mhz    = val.to_string(),
        "cache size" => rec.cache_size = val.to_string(),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// collect-kernel-params
// ---------------------------------------------------------------------------

pub fn collect_kernel_params(fs: &dyn Filesystem) -> ScopeWrapper<KernelParamRecord> {
    let mut records = Vec::new();
    walk_proc_sys_kernel(fs, "/proc/sys/kernel", "/proc/sys/kernel", &mut records);
    records.sort_by(|a, b| a.key.cmp(&b.key));
    ScopeWrapper { attributes: Default::default(), elements: records }
}

fn walk_proc_sys_kernel(
    fs: &dyn Filesystem,
    base: &str,
    dir: &str,
    records: &mut Vec<KernelParamRecord>,
) {
    let entries = match fs.read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries {
        if fs.is_dir(&entry) {
            walk_proc_sys_kernel(fs, base, &entry, records);
        } else {
            let content = match fs.read_file_limited(&entry, SITAR_READFILE_LIMIT) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let value = content.trim_end_matches('\n').to_string();
            if value.is_empty() { continue; }
            let key = entry[base.len()..].trim_start_matches('/').to_string();
            records.push(KernelParamRecord { key, value });
        }
    }
}

// ---------------------------------------------------------------------------
// collect-net-params
// ---------------------------------------------------------------------------

pub fn collect_net_params(fs: &dyn Filesystem) -> ScopeWrapper<NetParamRecord> {
    let subtrees = [
        "802", "appletalk", "ax25", "bridge", "core", "decnet",
        "ethernet", "ipv4", "ipv6", "irda", "ipx", "netfilter",
        "rose", "unix", "x25",
    ];
    let mut records = Vec::new();
    for sub in &subtrees {
        let dir = format!("/proc/sys/net/{}", sub);
        if fs.is_dir(&dir) {
            walk_proc_sys_net(fs, "/proc/sys/net", &dir, &mut records);
        }
    }
    records.sort_by(|a, b| a.key.cmp(&b.key));
    ScopeWrapper { attributes: Default::default(), elements: records }
}

fn walk_proc_sys_net(
    fs: &dyn Filesystem,
    base: &str,
    dir: &str,
    records: &mut Vec<NetParamRecord>,
) {
    let entries = match fs.read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries {
        if fs.is_dir(&entry) {
            walk_proc_sys_net(fs, base, &entry, records);
        } else {
            let content = match fs.read_file_limited(&entry, SITAR_READFILE_LIMIT) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let value = content.trim_end_matches('\n').to_string();
            if value.is_empty() { continue; }
            let key = entry[base.len()..].trim_start_matches('/').to_string();
            records.push(NetParamRecord { key, value });
        }
    }
}

// ---------------------------------------------------------------------------
// collect-devices
// ---------------------------------------------------------------------------

pub fn collect_devices(fs: &dyn Filesystem) -> ScopeWrapper<DeviceRecord> {
    let mut device_map: HashMap<String, DeviceRecord> = HashMap::new();

    // /proc/interrupts
    if let Ok(content) = fs.read_file("/proc/interrupts") {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() { continue; }
            if trimmed.starts_with(|c: char| c.is_ascii_alphabetic()) { continue; }
            let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
            if parts.len() < 2 { continue; }
            let irq_num = parts[0].trim().to_string();
            let rest = parts[1].trim();
            let dev_name = rest.split_whitespace().last().unwrap_or("").to_string();
            if dev_name.is_empty() { continue; }
            let entry = device_map.entry(dev_name.clone()).or_insert_with(|| DeviceRecord {
                name: dev_name,
                ..Default::default()
            });
            if entry.irq.is_empty() {
                entry.irq = irq_num;
            } else {
                entry.irq = format!("{},{}", entry.irq, irq_num);
            }
        }
    }

    // /proc/dma
    if let Ok(content) = fs.read_file("/proc/dma") {
        for line in content.lines() {
            if let Some(pos) = line.find(':') {
                let dma_num = line[..pos].trim().to_string();
                let rest = line[pos+1..].trim();
                let dev_name = rest.split('(').next().unwrap_or(rest).trim().to_string();
                if dev_name.is_empty() { continue; }
                let entry = device_map.entry(dev_name.clone()).or_insert_with(|| DeviceRecord {
                    name: dev_name,
                    ..Default::default()
                });
                if entry.dma.is_empty() { entry.dma = dma_num; }
                else { entry.dma = format!("{},{}", entry.dma, dma_num); }
            }
        }
    }

    // /proc/ioports
    if let Ok(content) = fs.read_file("/proc/ioports") {
        for line in content.lines() {
            if let Some(pos) = line.find(':') {
                let port_range = line[..pos].trim().to_string();
                let rest = line[pos+1..].trim();
                let dev_name = rest.split('(').next().unwrap_or(rest).trim().to_string();
                if dev_name.is_empty() { continue; }
                let entry = device_map.entry(dev_name.clone()).or_insert_with(|| DeviceRecord {
                    name: dev_name,
                    ..Default::default()
                });
                if entry.ports.is_empty() { entry.ports = port_range; }
                else { entry.ports = format!("{},{}", entry.ports, port_range); }
            }
        }
    }

    let mut records: Vec<DeviceRecord> = device_map.into_values().collect();
    records.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    ScopeWrapper { attributes: Default::default(), elements: records }
}

// ---------------------------------------------------------------------------
// collect-pci
// ---------------------------------------------------------------------------

pub fn collect_pci(fs: &dyn Filesystem, cr: &dyn CommandRunner) -> ScopeWrapper<PciRecord> {
    if let Ok((output, _)) = cr.run("lspci", &["-vm"]) {
        if !output.is_empty() {
            return parse_lspci_vm(&output);
        }
    }

    if fs.exists("/proc/pci") {
        if let Ok(content) = fs.read_file("/proc/pci") {
            return parse_proc_pci(&content);
        }
    }

    ScopeWrapper::default()
}

fn parse_lspci_vm(output: &str) -> ScopeWrapper<PciRecord> {
    let mut records = Vec::new();
    let mut current = PciRecord::default();
    let mut in_record = false;

    for line in output.lines() {
        if line.trim().is_empty() {
            if in_record {
                records.push(current.clone());
                current = PciRecord::default();
                in_record = false;
            }
            continue;
        }
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim().to_lowercase();
            let val = line[pos+1..].trim().to_string();
            match key.as_str() {
                "slot"    => { current.pci = val; in_record = true; }
                "device"  => { if current.device.is_empty() { current.device = val; } in_record = true; }
                "class"   => current.class   = val,
                "vendor"  => current.vendor  = val,
                "svendor" => current.svendor = val,
                "sdevice" => current.sdevice = val,
                "rev"     => current.rev     = val,
                _ => {}
            }
        }
    }
    if in_record { records.push(current); }

    ScopeWrapper { attributes: Default::default(), elements: records }
}

fn parse_proc_pci(content: &str) -> ScopeWrapper<PciRecord> {
    let mut records = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() || line.trim_start().starts_with("PCI") { continue; }
        let mut rec = PciRecord::default();
        rec.device = line.trim().to_string();
        records.push(rec);
    }
    ScopeWrapper { attributes: Default::default(), elements: records }
}

// ---------------------------------------------------------------------------
// collect-processes
// ---------------------------------------------------------------------------

pub fn collect_processes(fs: &dyn Filesystem) -> ScopeWrapper<ProcessRecord> {
    let entries = match fs.read_dir("/proc") {
        Ok(e) => e,
        Err(e) => {
            eprintln!("sitar: collect-processes: /proc unreadable: {}", e);
            return ScopeWrapper::default();
        }
    };

    let mut records = Vec::new();

    for entry in entries {
        let basename = entry.split('/').last().unwrap_or("").to_string();
        if !basename.chars().all(|c| c.is_ascii_digit()) || basename.is_empty() { continue; }
        let pid = basename.clone();

        let stat_path = format!("{}/stat", entry);
        let stat_content = match fs.read_file(&stat_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let (comm, state, ppid) = parse_proc_stat(&stat_content);

        let cmdline_path = format!("{}/cmdline", entry);
        let cmdline = fs.read_file(&cmdline_path)
            .map(|c| c.replace('\0', " ").trim().to_string())
            .unwrap_or_default();

        records.push(ProcessRecord { pid, ppid, comm, state, cmdline });
    }

    records.sort_by(|a, b| {
        let pa: u64 = a.pid.parse().unwrap_or(0);
        let pb: u64 = b.pid.parse().unwrap_or(0);
        pa.cmp(&pb)
    });

    ScopeWrapper { attributes: Default::default(), elements: records }
}

fn parse_proc_stat(content: &str) -> (String, String, String) {
    let start = content.find('(');
    let end   = content.rfind(')');
    if let (Some(s), Some(e)) = (start, end) {
        let comm  = content[s+1..e].to_string();
        let rest  = content[e+1..].trim();
        let mut parts = rest.splitn(3, ' ');
        let state = parts.next().unwrap_or("").to_string();
        let ppid  = parts.next().unwrap_or("").to_string();
        return (comm, state, ppid);
    }
    (String::new(), String::new(), String::new())
}

// ---------------------------------------------------------------------------
// collect-dmi
// ---------------------------------------------------------------------------

pub fn collect_dmi(cr: &dyn CommandRunner) -> Option<DmiScope> {
    match cr.run("dmidecode", &[]) {
        Ok((stdout, _)) if !stdout.is_empty() => Some(DmiScope { raw_output: stdout }),
        Ok(_) => None,
        Err(e) => {
            eprintln!("sitar: collect-dmi: dmidecode not available: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::{FakeFilesystem, FakeCommandRunner};

    #[test]
    fn test_collect_cpu_basic() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/proc/cpuinfo".to_string(), "processor\t: 0\nvendor_id\t: GenuineIntel\nmodel name\t: Intel Core i7\ncpu family\t: 6\nmodel\t\t: 142\nstepping\t: 10\ncpu MHz\t\t: 1992.000\ncache size\t: 8192 KB\n\nprocessor\t: 1\nvendor_id\t: GenuineIntel\nmodel name\t: Intel Core i7\ncpu family\t: 6\nmodel\t\t: 142\nstepping\t: 10\ncpu MHz\t\t: 1992.000\ncache size\t: 8192 KB\n\n".to_string());
        let mut cr = FakeCommandRunner::new();
        cr.responses.insert("uname".to_string(), ("x86_64\n".to_string(), String::new()));

        let scope = collect_cpu(&fs, &cr);
        assert_eq!(scope.elements.len(), 2);
        assert_eq!(scope.elements[0].vendor_id, "GenuineIntel");
        assert_eq!(scope.elements[0].processor, "0");
    }

    #[test]
    fn test_parse_proc_stat() {
        let (comm, state, ppid) = parse_proc_stat("1 (systemd) S 0 1 1 0 -1\n");
        assert_eq!(comm, "systemd");
        assert_eq!(state, "S");
        assert_eq!(ppid, "0");
    }

    #[test]
    fn test_collect_dmi_absent() {
        let cr = FakeCommandRunner::new();
        let result = collect_dmi(&cr);
        assert!(result.is_none());
    }

    #[test]
    fn test_collect_devices_from_interrupts() {
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/proc/interrupts".to_string(),
            "           CPU0       CPU1\n  0:         46          0   IO-APIC   2-edge      timer\n  1:          0          0   IO-APIC   1-edge      i8042\n".to_string());
        let scope = collect_devices(&fs);
        assert!(!scope.elements.is_empty());
        let timer = scope.elements.iter().find(|r| r.name == "timer");
        assert!(timer.is_some());
    }

    #[test]
    fn test_parse_lspci_vm() {
        let output = "Slot:\t00:00.0\nClass:\tHost bridge\nVendor:\tIntel Corporation\nDevice:\t8th Gen Core Processor Host Bridge\nSVendor:\tLenovo\nSDevice:\tThinkPad T480s\nRev:\t07\n\nSlot:\t00:02.0\nClass:\tVGA compatible controller\nVendor:\tIntel Corporation\nDevice:\tUHD Graphics 620\n\n";
        let scope = parse_lspci_vm(output);
        assert_eq!(scope.elements.len(), 2);
        assert_eq!(scope.elements[0].pci, "00:00.0");
        assert_eq!(scope.elements[0].class, "Host bridge");
    }
}
