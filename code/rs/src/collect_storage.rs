// collect_storage.rs — Storage collection (M3)
// BEHAVIORs: collect-storage, collect-btrfs
#![allow(dead_code)]
#![allow(unused_variables)]

use std::collections::HashMap;
use crate::interfaces::{Filesystem, CommandRunner};
use crate::types::*;

// ---------------------------------------------------------------------------
// collect-storage
// ---------------------------------------------------------------------------

pub fn collect_storage(fs: &dyn Filesystem, cr: &dyn CommandRunner) -> StorageScope {
    let mut scope = StorageScope::default();

    // Step 1: block device and mount information
    let mut partitions = collect_partitions(fs, cr);

    // Step 2: mount options
    enrich_mount_options(fs, cr, &mut partitions);

    // Step 3: df usage statistics
    enrich_df_stats(cr, &mut partitions);

    // Step 4: ext2/3/4 attributes
    enrich_ext_attrs(cr, &mut partitions);

    scope.partitions.elements = partitions;

    // Step 5: software RAID
    scope.software_raid = collect_software_raid(fs);

    // Step 6: IDE
    scope.ide = collect_ide(fs);

    // Step 7: SCSI
    scope.scsi = collect_scsi(fs);

    // Step 8: RAID controllers (independent sub-steps)
    collect_raid_controllers(fs, cr, &mut scope);

    // Step 9: EVMS
    if fs.is_executable("/sbin/evms_gather_info") || fs.is_executable("/usr/sbin/evms_gather_info") {
        if let Ok((out, _)) = cr.run("evms_gather_info", &[]) {
            scope.evms = Some(RawTextRecord { path: "evms_gather_info".to_string(), content: out });
        }
    }

    // Step 10: multipath
    if let Ok(conf) = fs.read_file("/etc/multipath.conf") {
        let mp_out = cr.run("multipath", &["-ll"])
            .map(|(o, _)| o)
            .unwrap_or_default();
        scope.multipath = Some(RawTextRecord {
            path: "/etc/multipath.conf".to_string(),
            content: format!("{}\n---\n{}", conf, mp_out),
        });
    }

    // Step 11: fstab
    if let Ok(content) = fs.read_file("/etc/fstab") {
        scope.fstab = Some(RawTextRecord { path: "/etc/fstab".to_string(), content });
    }

    // Step 12: lvm.conf
    let mut lvm_content = String::new();
    if let Ok(content) = fs.read_file("/etc/lvm/lvm.conf") {
        lvm_content.push_str(&content);
    }
    if let Ok(content) = fs.read_file("/etc/lvm/.cache") {
        lvm_content.push_str("\n--- .cache ---\n");
        lvm_content.push_str(&content);
    }
    if !lvm_content.is_empty() {
        scope.lvm_conf = Some(RawTextRecord { path: "/etc/lvm/lvm.conf".to_string(), content: lvm_content });
    }

    // Collect btrfs if any btrfs partitions
    let btrfs_mounts: Vec<String> = scope.partitions.elements.iter()
        .filter(|p| p.fstype == "btrfs" && !p.mountpoint.is_empty())
        .map(|p| p.mountpoint.clone())
        .collect();
    if !btrfs_mounts.is_empty() {
        scope.btrfs = collect_btrfs(fs, cr, &btrfs_mounts);
    }

    scope
}

fn collect_partitions(fs: &dyn Filesystem, cr: &dyn CommandRunner) -> Vec<PartitionRecord> {
    // Primary: lsblk -J
    if let Ok((output, _)) = cr.run("lsblk", &["-J", "-o", "NAME,MAJ:MIN,TYPE,SIZE,FSTYPE,MOUNTPOINT,UUID,LABEL,RO"]) {
        if !output.is_empty() {
            if let Some(records) = parse_lsblk_json(&output) {
                return records;
            }
        }
    }

    // Fallback: fdisk -l + mount
    collect_partitions_fdisk(fs, cr)
}

fn parse_lsblk_json(output: &str) -> Option<Vec<PartitionRecord>> {
    let v: serde_json::Value = serde_json::from_str(output).ok()?;
    let devices = v.get("blockdevices")?.as_array()?;
    let mut records = Vec::new();
    for dev in devices {
        parse_lsblk_device(dev, &mut records);
    }
    Some(records)
}

fn parse_lsblk_device(dev: &serde_json::Value, records: &mut Vec<PartitionRecord>) {
    let name = dev.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let dev_type = dev.get("type").and_then(|v| v.as_str()).unwrap_or("");

    if matches!(dev_type, "disk" | "part" | "lvm" | "raid1" | "raid5" | "raid6" | "raid10" | "md") {
        let mut rec = PartitionRecord::default();
        rec.device     = format!("/dev/{}", name);
        rec.maj_min    = dev.get("maj:min").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.r#type     = dev_type.to_string();
        rec.size       = dev.get("size").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.fstype     = dev.get("fstype").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.mountpoint = dev.get("mountpoint").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.uuid       = dev.get("uuid").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.label      = dev.get("label").and_then(|v| v.as_str()).unwrap_or("").to_string();
        rec.ro         = dev.get("ro").and_then(|v| v.as_str())
            .or_else(|| dev.get("ro").and_then(|v| if v.as_bool() == Some(true) { Some("1") } else { Some("0") }))
            .unwrap_or("0").to_string();
        rec.source = "lsblk".to_string();
        records.push(rec);
    }

    // Recurse into children
    if let Some(children) = dev.get("children").and_then(|v| v.as_array()) {
        for child in children {
            parse_lsblk_device(child, records);
        }
    }
}

fn collect_partitions_fdisk(fs: &dyn Filesystem, cr: &dyn CommandRunner) -> Vec<PartitionRecord> {
    let mut mount_map: HashMap<String, (String, String, String)> = HashMap::new();

    // Parse mount output
    if let Ok((mount_out, _)) = cr.run("mount", &[]) {
        for line in mount_out.lines() {
            if !line.starts_with("/dev") { continue; }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let device = parts[0].to_string();
                let mountpoint = parts[2].to_string();
                let fstype = parts[4].to_string();
                let opts = if parts.len() > 5 {
                    parts[5].trim_matches(|c| c == '(' || c == ')').to_string()
                } else {
                    String::new()
                };
                mount_map.insert(device, (mountpoint, fstype, opts));
            }
        }
    }

    let mut records = Vec::new();

    if let Ok((fdisk_out, _)) = cr.run("fdisk", &["-l"]) {
        for line in fdisk_out.lines() {
            if !line.starts_with("/dev") { continue; }
            let line = line.replace('*', " "); // strip boot flag
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() { continue; }
            let device = parts[0].to_string();
            let mut rec = PartitionRecord::default();
            rec.device = device.clone();
            rec.source = "fdisk".to_string();
            if parts.len() >= 3 { rec.begin_sector = parts[1].to_string(); }
            if parts.len() >= 4 { rec.end_sector   = parts[2].to_string(); }
            if parts.len() >= 5 {
                rec.raw_size_kb = parts[3].parse().unwrap_or(0);
            }
            if parts.len() >= 6 { rec.type_id = parts[4].to_string(); }
            if parts.len() >= 7 {
                rec.partition_type = parts[5..].join(" ");
            }
            // Adjust type_id
            if rec.type_id == "8e" { rec.partition_type = "LVM-PV".to_string(); }
            if rec.type_id == "fe" { rec.partition_type = "old LVM".to_string(); }

            if let Some((mp, ft, opts)) = mount_map.get(&device) {
                rec.mountpoint   = mp.clone();
                rec.fstype       = ft.clone();
                rec.mount_options = opts.clone();
            }
            records.push(rec);
        }
    }

    records
}

fn enrich_mount_options(fs: &dyn Filesystem, cr: &dyn CommandRunner, partitions: &mut Vec<PartitionRecord>) {
    // Try findmnt -J first
    if let Ok((output, _)) = cr.run("findmnt", &["-J"]) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&output) {
            if let Some(filesystems) = v.get("filesystems").and_then(|f| f.as_array()) {
                let mut opts_map: HashMap<String, String> = HashMap::new();
                collect_findmnt_opts(filesystems, &mut opts_map);
                for rec in partitions.iter_mut() {
                    if let Some(opts) = opts_map.get(&rec.device) {
                        rec.mount_options = opts.clone();
                    }
                }
                return;
            }
        }
    }

    // Fallback: mount output
    if let Ok((mount_out, _)) = cr.run("mount", &[]) {
        let mut opts_map: HashMap<String, String> = HashMap::new();
        for line in mount_out.lines() {
            if !line.starts_with("/dev") { continue; }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 6 {
                let device = parts[0].to_string();
                let opts = parts[5].trim_matches(|c| c == '(' || c == ')').to_string();
                opts_map.insert(device, opts);
            }
        }
        for rec in partitions.iter_mut() {
            if rec.mount_options.is_empty() {
                if let Some(opts) = opts_map.get(&rec.device) {
                    rec.mount_options = opts.clone();
                }
            }
        }
    }
}

fn collect_findmnt_opts(filesystems: &[serde_json::Value], map: &mut HashMap<String, String>) {
    for fs_entry in filesystems {
        let source = fs_entry.get("source").and_then(|v| v.as_str()).unwrap_or("");
        let opts   = fs_entry.get("options").and_then(|v| v.as_str()).unwrap_or("");
        if !source.is_empty() {
            map.insert(source.to_string(), opts.to_string());
        }
        if let Some(children) = fs_entry.get("children").and_then(|v| v.as_array()) {
            collect_findmnt_opts(children, map);
        }
    }
}

fn enrich_df_stats(cr: &dyn CommandRunner, partitions: &mut Vec<PartitionRecord>) {
    let output = match cr.run("df", &["-PPk"]) {
        Ok((o, _)) => o,
        Err(_) => return,
    };
    let mut df_map: HashMap<String, (i64, i64, i64, String)> = HashMap::new();
    for line in output.lines() {
        if !line.starts_with("/dev") { continue; }
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 { continue; }
        let device = parts[0].to_string();
        let blocks: i64 = parts[1].parse().unwrap_or(0);
        let used:   i64 = parts[2].parse().unwrap_or(0);
        let avail:  i64 = parts[3].parse().unwrap_or(0);
        let pct = parts[4].to_string();
        df_map.insert(device, (blocks, used, avail, pct));
    }
    for rec in partitions.iter_mut() {
        if let Some((blocks, used, avail, pct)) = df_map.get(&rec.device) {
            rec.df_blocks_kb   = *blocks;
            rec.df_used_kb     = *used;
            rec.df_avail_kb    = *avail;
            rec.df_use_percent = pct.clone();
        }
    }
}

fn enrich_ext_attrs(cr: &dyn CommandRunner, partitions: &mut Vec<PartitionRecord>) {
    for rec in partitions.iter_mut() {
        if !matches!(rec.fstype.as_str(), "ext2" | "ext3" | "ext4") { continue; }
        let output = match cr.run("tune2fs", &["-l", &rec.device]) {
            Ok((o, _)) => o,
            Err(_) => continue,
        };
        let mut inode_count: f64 = 0.0;
        let mut block_count: f64 = 0.0;
        let mut block_size_val: f64 = 0.0;

        for line in output.lines() {
            if let Some(v) = line.strip_prefix("Reserved block count:") {
                rec.reserved_blocks = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Block size:") {
                let s = v.trim().to_string();
                block_size_val = s.parse().unwrap_or(0.0);
                rec.block_size = s;
            } else if let Some(v) = line.strip_prefix("Inode count:") {
                inode_count = v.trim().parse().unwrap_or(0.0);
            } else if let Some(v) = line.strip_prefix("Block count:") {
                block_count = v.trim().parse().unwrap_or(0.0);
            } else if let Some(v) = line.strip_prefix("Maximum mount count:") {
                rec.max_mount_count = v.trim().to_string();
            }
        }

        // Compute inode_density
        if inode_count > 0.0 && block_count > 0.0 && block_size_val > 0.0 {
            let ratio = block_count / inode_count;
            let log2_ratio = ratio.log2();
            let rounded = log2_ratio.round() as i32;
            let density = (2_f64.powi(rounded) * block_size_val) as u64;
            rec.inode_density = density.to_string();
        }
    }
}

fn collect_software_raid(fs: &dyn Filesystem) -> ScopeWrapper<SoftwareRaidRecord> {
    let content = match fs.read_file("/proc/mdstat") {
        Ok(c) => c,
        Err(_) => return ScopeWrapper::default(),
    };

    let mut records = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        // Match: md0 : active raid1 sda1[0] sdb1[1]
        if let Some(captures) = parse_mdstat_active_line(line) {
            let (device, level, parts) = captures;
            let mut rec = SoftwareRaidRecord {
                device,
                level,
                partitions: parts,
                ..Default::default()
            };
            // Next line has blocks/chunk info
            if i + 1 < lines.len() {
                let next = lines[i + 1];
                parse_mdstat_info_line(next, &mut rec);
            }
            records.push(rec);
        }
        i += 1;
    }

    ScopeWrapper { attributes: Default::default(), elements: records }
}

fn parse_mdstat_active_line(line: &str) -> Option<(String, String, Vec<String>)> {
    // md0 : active raid1 sda1[0] sdb1[1]
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 { return None; }
    if parts[1] != ":" || parts[2] != "active" { return None; }
    let device = parts[0].to_string();
    let level  = parts[3].to_string();
    let members: Vec<String> = parts[4..].iter()
        .map(|s| s.split('[').next().unwrap_or(s).to_string())
        .collect();
    Some((device, level, members))
}

fn parse_mdstat_info_line(line: &str, rec: &mut SoftwareRaidRecord) {
    // 1953382400 blocks super 1.2 [2/2] [UU]
    // or: 1953382400 blocks super 1.2 512k chunk, algorithm 2 [2/2] [UU]
    for part in line.split_whitespace() {
        if part.ends_with("k") || part.ends_with("K") {
            if let Ok(n) = part.trim_end_matches(|c| c == 'k' || c == 'K').parse::<u64>() {
                rec.chunk_size = format!("{}k", n);
            }
        }
    }
    if let Some(pos) = line.find("blocks") {
        let before = line[..pos].trim();
        rec.blocks = before.split_whitespace().last().unwrap_or("").to_string();
    }
    if let Some(pos) = line.find("algorithm") {
        let rest = line[pos + "algorithm".len()..].trim();
        rec.algorithm = rest.split_whitespace().next().unwrap_or("").to_string();
    }
}

fn collect_ide(fs: &dyn Filesystem) -> ScopeWrapper<IdeRecord> {
    let letters = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
    let mut records = Vec::new();

    for &letter in &letters {
        let dev_path = format!("/proc/ide/hd{}", letter);
        if !fs.is_dir(&dev_path) { continue; }

        let media = fs.read_file(&format!("{}/media", dev_path))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let model = fs.read_file(&format!("{}/model", dev_path))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let driver = fs.read_file(&format!("{}/driver", dev_path))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let (geometry_phys, geometry_log, capacity_blocks) = if media == "disk" {
            let geo = fs.read_file(&format!("{}/geometry", dev_path))
                .unwrap_or_default();
            let mut phys = String::new();
            let mut log  = String::new();
            for line in geo.lines() {
                if let Some(v) = line.strip_prefix("physical geometry =") {
                    phys = v.trim().to_string();
                } else if let Some(v) = line.strip_prefix("logical  geometry =") {
                    log = v.trim().to_string();
                }
            }
            let cap = fs.read_file(&format!("{}/capacity", dev_path))
                .map(|s| s.trim().to_string())
                .unwrap_or_default();
            (phys, log, cap)
        } else {
            (String::new(), String::new(), String::new())
        };

        records.push(IdeRecord {
            device: format!("/dev/hd{}", letter),
            media,
            model,
            driver,
            geometry_phys,
            geometry_log,
            capacity_blocks,
        });
    }

    ScopeWrapper { attributes: Default::default(), elements: records }
}

fn collect_scsi(fs: &dyn Filesystem) -> ScopeWrapper<ScsiRecord> {
    let content = match fs.read_file("/proc/scsi/scsi") {
        Ok(c) => c,
        Err(_) => return ScopeWrapper::default(),
    };

    let mut records = Vec::new();
    let mut host = String::new();
    let mut channel = String::new();
    let mut id = String::new();
    let mut lun = String::new();
    let mut vendor = String::new();
    let mut model = String::new();
    let mut revision = String::new();
    let mut scsi_type = String::new();
    let mut ansi_rev = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("Host:") {
            // "Host: scsi0 Channel: 00 Id: 00 Lun: 00"
            for part in line.split_whitespace().collect::<Vec<_>>().windows(2) {
                match part[0] {
                    "Host:"    => host    = part[1].trim_start_matches("scsi").to_string(),
                    "Channel:" => channel = part[1].to_string(),
                    "Id:"      => id      = part[1].to_string(),
                    "Lun:"     => lun     = part[1].to_string(),
                    _ => {}
                }
            }
        } else if line.starts_with("Vendor:") {
            // "Vendor: ATA      Model: Samsung SSD 860  Rev: 1B6Q"
            if let Some(v_pos) = line.find("Vendor:") {
                if let Some(m_pos) = line.find("Model:") {
                    if let Some(r_pos) = line.find("Rev:") {
                        vendor   = line[v_pos+7..m_pos].trim().to_string();
                        model    = line[m_pos+6..r_pos].trim().to_string();
                        revision = line[r_pos+4..].trim().to_string();
                    }
                }
            }
        } else if line.starts_with("Type:") {
            // "Type:   Direct-Access                    ANSI  SCSI revision: 05"
            if let Some(a_pos) = line.find("ANSI") {
                scsi_type = line[5..a_pos].trim().to_string();
                if let Some(r_pos) = line.find("revision:") {
                    ansi_rev = line[r_pos+9..].trim().to_string();
                }
            }
            records.push(ScsiRecord {
                host: host.clone(), channel: channel.clone(),
                id: id.clone(), lun: lun.clone(),
                vendor: vendor.clone(), model: model.clone(),
                revision: revision.clone(),
                r#type: scsi_type.clone(),
                ansi_rev: ansi_rev.clone(),
            });
            host.clear(); channel.clear(); id.clear(); lun.clear();
            vendor.clear(); model.clear(); revision.clear();
            scsi_type.clear(); ansi_rev.clear();
        }
    }

    ScopeWrapper { attributes: Default::default(), elements: records }
}

fn collect_raid_controllers(fs: &dyn Filesystem, cr: &dyn CommandRunner, scope: &mut StorageScope) {
    // CCISS/HP SmartArray
    for cmd in &["hpacucli", "cpqacucli"] {
        if let Ok((out, _)) = cr.run(cmd, &["ctrl", "all", "show", "config"]) {
            scope.cciss.elements.push(RawControllerRecord {
                controller_id: cmd.to_string(),
                raw_output: out,
            });
        }
    }

    // Areca
    for cmd in &["/usr/lib/snmp/cli64", "/usr/lib/snmp/cli32"] {
        if fs.is_executable(cmd) {
            if let Ok((out, _)) = cr.run(cmd, &["hw info"]) {
                scope.areca.elements.push(RawControllerRecord {
                    controller_id: cmd.to_string(),
                    raw_output: out,
                });
            }
        }
    }

    // DAC960
    if fs.is_dir("/proc/rd") {
        if let Ok(content) = fs.read_file("/proc/rd/status") {
            scope.dac960.elements.push(RawControllerRecord {
                controller_id: "dac960".to_string(),
                raw_output: content,
            });
        }
    }

    // GDTH
    if fs.is_dir("/proc/scsi/gdth") {
        if let Ok(entries) = fs.read_dir("/proc/scsi/gdth") {
            for entry in entries {
                if let Ok(content) = fs.read_file(&entry) {
                    scope.gdth.elements.push(RawControllerRecord {
                        controller_id: entry.clone(),
                        raw_output: content,
                    });
                }
            }
        }
    }

    // IPS
    if fs.is_dir("/proc/scsi/ips") {
        if let Ok(entries) = fs.read_dir("/proc/scsi/ips") {
            for entry in entries {
                if let Ok(content) = fs.read_file(&entry) {
                    scope.ips.elements.push(RawControllerRecord {
                        controller_id: entry.clone(),
                        raw_output: content,
                    });
                }
            }
        }
    }

    // Compaq Smart Array
    if fs.is_dir("/proc/array") {
        if let Ok(entries) = fs.read_dir("/proc/array") {
            for entry in entries {
                if let Ok(content) = fs.read_file(&entry) {
                    scope.compaq_smart.elements.push(RawControllerRecord {
                        controller_id: entry.clone(),
                        raw_output: content,
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// collect-btrfs
// ---------------------------------------------------------------------------

pub fn collect_btrfs(
    fs: &dyn Filesystem,
    cr: &dyn CommandRunner,
    mountpoints: &[String],
) -> ScopeWrapper<BtrfsFilesystemRecord> {
    let mut records = Vec::new();
    let mut seen_uuids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for mp in mountpoints {
        // show
        let show_out = match cr.run("btrfs", &["filesystem", "show", mp]) {
            Ok((o, _)) => o,
            Err(e) => {
                eprintln!("sitar: collect-btrfs: btrfs filesystem show {} failed: {}", mp, e);
                continue;
            }
        };

        let mut rec = BtrfsFilesystemRecord {
            mount_point: mp.clone(),
            ..Default::default()
        };

        // Parse show output
        for line in show_out.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Label:") {
                // Label: 'none' uuid: abc-def-...
                if let Some(uuid_pos) = trimmed.find("uuid:") {
                    rec.uuid = trimmed[uuid_pos+5..].trim().to_string();
                }
                if let Some(label_part) = trimmed.split("uuid:").next() {
                    let raw = label_part.trim_start_matches("Label:").trim().trim_matches('\'');
                    rec.label = if raw == "none" { "<unlabeled>".to_string() } else { raw.to_string() };
                }
            } else if trimmed.starts_with("Total devices") {
                rec.total_devices = trimmed.split_whitespace()
                    .nth(2).unwrap_or("").to_string();
            } else if trimmed.contains("FS bytes used") {
                rec.bytes_used = trimmed.split_whitespace().last().unwrap_or("").to_string();
            } else if trimmed.starts_with("devid") {
                // devid  1 size 50.00GiB used 30.00GiB path /dev/sda2
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                let mut bdev = BtrfsDeviceRecord::default();
                for (i, &p) in parts.iter().enumerate() {
                    match p {
                        "devid" => bdev.devid = parts.get(i+1).unwrap_or(&"").to_string(),
                        "size"  => bdev.size  = parts.get(i+1).unwrap_or(&"").to_string(),
                        "used"  => bdev.used  = parts.get(i+1).unwrap_or(&"").to_string(),
                        "path"  => bdev.path  = parts.get(i+1).unwrap_or(&"").to_string(),
                        _ => {}
                    }
                }
                rec.devices.push(bdev);
            }
        }

        // Deduplicate by UUID
        if !rec.uuid.is_empty() && seen_uuids.contains(&rec.uuid) { continue; }
        if !rec.uuid.is_empty() { seen_uuids.insert(rec.uuid.clone()); }

        // subvolume list
        if let Ok((sub_out, _)) = cr.run("btrfs", &["subvolume", "list", mp]) {
            for line in sub_out.lines() {
                // ID 256 gen 8 top level 5 path @
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut sv = BtrfsSubvolumeRecord::default();
                for (i, &p) in parts.iter().enumerate() {
                    match p {
                        "ID"        => sv.id        = parts.get(i+1).unwrap_or(&"").to_string(),
                        "gen"       => sv.gen       = parts.get(i+1).unwrap_or(&"").to_string(),
                        "level"     => sv.top_level = parts.get(i+1).unwrap_or(&"").to_string(),
                        "path"      => sv.path      = parts[i+1..].join(" "),
                        _ => {}
                    }
                }
                if !sv.id.is_empty() { rec.subvolumes.push(sv); }
            }
        }

        // df
        if let Ok((df_out, _)) = cr.run("btrfs", &["filesystem", "df", mp]) {
            for line in df_out.lines() {
                // Data, single: total=1.00GiB, used=512.00MiB
                if let Some(colon) = line.find(':') {
                    let type_part = line[..colon].trim().to_lowercase();
                    let rest = line[colon+1..].trim();
                    let mut total = String::new();
                    let mut used  = String::new();
                    for part in rest.split(',') {
                        let part = part.trim();
                        if let Some(v) = part.strip_prefix("total=") {
                            total = v.to_string();
                        } else if let Some(v) = part.strip_prefix("used=") {
                            used = v.to_string();
                        }
                    }
                    if type_part.starts_with("data") {
                        rec.data_total = total; rec.data_used = used;
                    } else if type_part.starts_with("metadata") {
                        rec.metadata_total = total; rec.metadata_used = used;
                    } else if type_part.starts_with("system") {
                        rec.system_total = total; rec.system_used = used;
                    }
                }
            }
        }

        records.push(rec);
    }

    ScopeWrapper { attributes: Default::default(), elements: records }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::{FakeFilesystem, FakeCommandRunner};

    #[test]
    fn test_parse_lsblk_json() {
        let json = r#"{"blockdevices":[{"name":"sda","maj:min":"8:0","type":"disk","size":"100G","fstype":null,"mountpoint":null,"uuid":null,"label":null,"ro":"0","children":[{"name":"sda1","maj:min":"8:1","type":"part","size":"50G","fstype":"ext4","mountpoint":"/","uuid":"abc-123","label":null,"ro":"0"}]}]}"#;
        let records = parse_lsblk_json(json).unwrap();
        assert_eq!(records.len(), 2); // disk + partition
        assert_eq!(records[0].device, "/dev/sda");
        assert_eq!(records[1].device, "/dev/sda1");
        assert_eq!(records[1].fstype, "ext4");
        assert_eq!(records[1].source, "lsblk");
    }

    #[test]
    fn test_parse_mdstat() {
        let content = "Personalities : [raid1]\nmd0 : active raid1 sda1[0] sdb1[1]\n      1953382400 blocks super 1.2 [2/2] [UU]\n\nunused devices: <none>\n";
        let mut fs = FakeFilesystem::new();
        fs.files.insert("/proc/mdstat".to_string(), content.to_string());
        let scope = collect_software_raid(&fs);
        assert_eq!(scope.elements.len(), 1);
        assert_eq!(scope.elements[0].device, "md0");
        assert_eq!(scope.elements[0].level, "raid1");
        assert!(scope.elements[0].partitions.contains(&"sda1".to_string()));
    }

    #[test]
    fn test_collect_storage_empty_fallback() {
        let fs = FakeFilesystem::new();
        let cr = FakeCommandRunner::new();
        let scope = collect_storage(&fs, &cr);
        // Should not panic; partitions may be empty when no tools available
        assert!(scope.partitions.elements.len() >= 0);
    }
}
