// render_human.rs — Human-readable renderer (M6)
// All 30 named render functions for html, tex, sdocbook, markdown formats
#![allow(dead_code)]
#![allow(unused_variables)]

use crate::interfaces::Renderer;
use crate::types::*;

// ---------------------------------------------------------------------------
// render_human — top-level dispatch
// ---------------------------------------------------------------------------

pub fn render_human(
    manifest: &SitarManifest,
    renderer: &dyn Renderer,
    outfile: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut output = String::new();

    // Step 2: header
    output.push_str(&renderer.header(manifest));

    // Collect section titles for TOC
    let section_titles: Vec<String> = vec![
        "General Information", "CPU", "Kernel Parameters", "Network Parameters",
        "Devices", "PCI Devices", "Software RAID", "Partitions, Mounts, LVM",
        "Btrfs Filesystems", "fstab", "LVM Configuration", "EVMS",
        "Multipath", "IDE Devices", "SCSI Devices", "CCISS Controllers",
        "Areca Controllers", "DAC960 Controllers", "GDTH Controllers",
        "IPS Controllers", "Compaq Smart Array", "Networking Interfaces",
        "Routing Table", "Packet Filter", "AppArmor", "DMI Information",
        "Services", "Changed Configuration Files", "Installed Packages",
        "Kernel Configuration",
    ].into_iter().map(|s| s.to_string()).collect();

    // Step 3: TOC
    output.push_str(&renderer.toc(&section_titles));

    // Step 4: sections in canonical SECTION-MAP order
    output.push_str(&render_general_info(manifest, renderer));
    output.push_str(&render_cpu(manifest, renderer));
    output.push_str(&render_kernel_params(manifest, renderer));
    output.push_str(&render_net_params(manifest, renderer));
    output.push_str(&render_devices(manifest, renderer));
    output.push_str(&render_pci(manifest, renderer));
    output.push_str(&render_software_raid(manifest, renderer));
    output.push_str(&render_partitions(manifest, renderer));
    output.push_str(&render_btrfs(manifest, renderer));
    output.push_str(&render_fstab(manifest, renderer));
    output.push_str(&render_lvm_conf(manifest, renderer));
    output.push_str(&render_evms(manifest, renderer));
    output.push_str(&render_multipath(manifest, renderer));
    output.push_str(&render_ide(manifest, renderer));
    output.push_str(&render_scsi(manifest, renderer));
    output.push_str(&render_cciss(manifest, renderer));
    output.push_str(&render_areca(manifest, renderer));
    output.push_str(&render_dac960(manifest, renderer));
    output.push_str(&render_gdth(manifest, renderer));
    output.push_str(&render_ips(manifest, renderer));
    output.push_str(&render_compaq_smart(manifest, renderer));
    output.push_str(&render_network_interfaces(manifest, renderer));
    output.push_str(&render_routing(manifest, renderer));
    output.push_str(&render_packet_filter(manifest, renderer));
    output.push_str(&render_apparmor(manifest, renderer));
    output.push_str(&render_dmi(manifest, renderer));
    output.push_str(&render_services(manifest, renderer));
    output.push_str(&render_changed_config_files(manifest, renderer));
    output.push_str(&render_packages(manifest, renderer));
    output.push_str(&render_kernel_config(manifest, renderer));

    // Step 5: footer
    output.push_str(&renderer.footer());

    // Write to file
    std::fs::write(outfile, &output)?;
    Ok(output.len())
}

// ---------------------------------------------------------------------------
// Helper: build a table from headers and rows
// ---------------------------------------------------------------------------

fn make_table(renderer: &dyn Renderer, headers: &[&str], rows: Vec<Vec<String>>) -> String {
    if rows.is_empty() { return String::new(); }

    // Detect format by trying a header call — we use a simple heuristic:
    // We'll produce a generic table string and rely on the renderer's escape.
    // For HTML we produce <table>, for Markdown we produce GFM, etc.
    // We use a format-detection approach via a probe.
    let header_probe = renderer.header(&SitarManifest::default());
    if header_probe.contains("<!DOCTYPE") {
        html_table(renderer, headers, &rows)
    } else if header_probe.contains("\\documentclass") {
        tex_table(renderer, headers, &rows)
    } else if header_probe.contains("<?xml") {
        docbook_table(renderer, headers, &rows)
    } else if header_probe.starts_with("# SITAR") || header_probe.contains("SITAR") {
        markdown_table(renderer, headers, &rows)
    } else {
        // Fallback: markdown
        markdown_table(renderer, headers, &rows)
    }
}

fn html_table(renderer: &dyn Renderer, headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut s = String::from("<table>\n<tr>");
    for h in headers {
        s.push_str(&format!("<th>{}</th>", renderer.escape(h)));
    }
    s.push_str("</tr>\n");
    for row in rows {
        s.push_str("<tr>");
        for cell in row {
            s.push_str(&format!("<td>{}</td>", renderer.escape(cell)));
        }
        s.push_str("</tr>\n");
    }
    s.push_str("</table>\n");
    s
}

fn tex_table(renderer: &dyn Renderer, headers: &[&str], rows: &[Vec<String>]) -> String {
    let cols = headers.len();
    let col_spec: String = (0..cols).map(|_| "l").collect::<Vec<_>>().join("|");
    let mut s = format!("\\begin{{longtable}}{{|{}|}}\n\\hline\n", col_spec);
    let header_row: Vec<String> = headers.iter().map(|h| format!("\\textbf{{{}}}", renderer.escape(h))).collect();
    s.push_str(&header_row.join(" & "));
    s.push_str(" \\\\\n\\hline\n");
    for row in rows {
        let cells: Vec<String> = row.iter().map(|c| renderer.escape(c)).collect();
        s.push_str(&cells.join(" & "));
        s.push_str(" \\\\\n");
    }
    s.push_str("\\hline\n\\end{longtable}\n");
    s
}

fn docbook_table(renderer: &dyn Renderer, headers: &[&str], rows: &[Vec<String>]) -> String {
    let cols = headers.len();
    let mut s = format!("<informaltable><tgroup cols=\"{}\">\n<thead><row>", cols);
    for h in headers {
        s.push_str(&format!("<entry>{}</entry>", renderer.escape(h)));
    }
    s.push_str("</row></thead>\n<tbody>\n");
    for row in rows {
        s.push_str("<row>");
        for cell in row {
            s.push_str(&format!("<entry>{}</entry>", renderer.escape(cell)));
        }
        s.push_str("</row>\n");
    }
    s.push_str("</tbody>\n</tgroup></informaltable>\n");
    s
}

fn markdown_table(renderer: &dyn Renderer, headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut s = String::new();
    s.push('|');
    for h in headers {
        s.push_str(&format!(" {} |", renderer.escape(h)));
    }
    s.push('\n');
    s.push('|');
    for _ in headers {
        s.push_str(" --- |");
    }
    s.push('\n');
    for row in rows {
        s.push('|');
        for cell in row {
            s.push_str(&format!(" {} |", renderer.escape(cell)));
        }
        s.push('\n');
    }
    s.push('\n');
    s
}

fn make_verbatim(renderer: &dyn Renderer, content: &str) -> String {
    let probe = renderer.header(&SitarManifest::default());
    if probe.contains("<!DOCTYPE") {
        format!("<pre>{}</pre>\n", renderer.escape(content))
    } else if probe.contains("\\documentclass") {
        format!("\\begin{{verbatim}}\n{}\n\\end{{verbatim}}\n", content)
    } else if probe.contains("<?xml") {
        format!("<screen>{}</screen>\n", renderer.escape(content))
    } else {
        format!("```\n{}\n```\n\n", content)
    }
}

// ---------------------------------------------------------------------------
// 30 named render functions (SECTION-MAP order)
// ---------------------------------------------------------------------------

pub fn render_general_info(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.general_info.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.general_info.elements.iter()
        .map(|r| vec![r.key.clone(), r.value.clone()])
        .collect();
    let content = make_table(renderer, &["key", "value"], rows);
    renderer.section("General Information", 1, &content)
}

pub fn render_cpu(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.cpu.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.cpu.elements.iter()
        .map(|r| vec![
            r.processor.clone(), r.vendor_id.clone(), r.model_name.clone(),
            r.cpu_family.clone(), r.model.clone(), r.stepping.clone(),
            r.cpu_mhz.clone(), r.cache_size.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["processor", "vendor_id", "model_name", "cpu_family", "model", "stepping", "cpu_mhz", "cache_size"],
        rows);
    renderer.section("CPU", 1, &content)
}

pub fn render_kernel_params(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.kernel_params.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.kernel_params.elements.iter()
        .map(|r| vec![r.key.clone(), r.value.clone()])
        .collect();
    let content = make_table(renderer, &["key", "value"], rows);
    renderer.section("Kernel Parameters", 1, &content)
}

pub fn render_net_params(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.net_params.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.net_params.elements.iter()
        .map(|r| vec![r.key.clone(), r.value.clone()])
        .collect();
    let content = make_table(renderer, &["key", "value"], rows);
    renderer.section("Network Parameters", 1, &content)
}

pub fn render_devices(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.devices.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.devices.elements.iter()
        .map(|r| vec![r.name.clone(), r.dma.clone(), r.irq.clone(), r.ports.clone()])
        .collect();
    let content = make_table(renderer, &["name", "dma", "irq", "ports"], rows);
    renderer.section("Devices", 1, &content)
}

pub fn render_pci(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.pci.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.pci.elements.iter()
        .map(|r| vec![
            r.pci.clone(), r.device.clone(), r.class.clone(),
            r.vendor.clone(), r.svendor.clone(), r.sdevice.clone(), r.rev.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["pci", "device", "class", "vendor", "svendor", "sdevice", "rev"],
        rows);
    renderer.section("PCI Devices", 1, &content)
}

pub fn render_software_raid(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.storage.software_raid.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.storage.software_raid.elements.iter()
        .map(|r| vec![
            r.device.clone(), r.level.clone(), r.partitions.join(","),
            r.blocks.clone(), r.chunk_size.clone(), r.algorithm.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["device", "level", "partitions", "blocks", "chunk_size", "algorithm"],
        rows);
    renderer.section("Software RAID", 1, &content)
}

pub fn render_partitions(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.storage.partitions.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.storage.partitions.elements.iter()
        .map(|r| vec![
            r.device.clone(), r.r#type.clone(), r.size.clone(), r.fstype.clone(),
            r.mountpoint.clone(), r.uuid.clone(), r.label.clone(), r.source.clone(),
            r.begin_sector.clone(), r.end_sector.clone(), r.mount_options.clone(),
            r.block_size.clone(), r.inode_density.clone(), r.max_mount_count.clone(),
            r.df_blocks_kb.to_string(), r.df_used_kb.to_string(),
            r.df_avail_kb.to_string(), r.df_use_percent.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["device", "type", "size", "fstype", "mountpoint", "uuid", "label", "source",
          "begin_sector", "end_sector", "mount_options", "block_size", "inode_density",
          "max_mount_count", "df_blocks_kb", "df_used_kb", "df_avail_kb", "df_use_percent"],
        rows);
    renderer.section("Partitions, Mounts, LVM", 1, &content)
}

pub fn render_btrfs(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.storage.btrfs.elements.is_empty() { return String::new(); }
    let mut content = String::new();
    for fs_rec in &manifest.storage.btrfs.elements {
        let heading = format!("Btrfs: {} ({})", fs_rec.label, fs_rec.uuid);
        // Devices sub-table
        let dev_rows: Vec<Vec<String>> = fs_rec.devices.iter()
            .map(|d| vec![d.devid.clone(), d.size.clone(), d.used.clone(), d.path.clone()])
            .collect();
        let dev_table = make_table(renderer, &["devid", "size", "used", "path"], dev_rows);
        // Subvolumes sub-table
        let sv_rows: Vec<Vec<String>> = fs_rec.subvolumes.iter()
            .map(|s| vec![s.id.clone(), s.gen.clone(), s.top_level.clone(), s.path.clone()])
            .collect();
        let sv_table = make_table(renderer, &["id", "gen", "top_level", "path"], sv_rows);
        // Space usage
        let space_rows = vec![vec![
            fs_rec.data_total.clone(), fs_rec.data_used.clone(),
            fs_rec.metadata_total.clone(), fs_rec.metadata_used.clone(),
            fs_rec.system_total.clone(), fs_rec.system_used.clone(),
        ]];
        let space_table = make_table(renderer,
            &["data_total", "data_used", "metadata_total", "metadata_used", "system_total", "system_used"],
            space_rows);
        content.push_str(&renderer.section(&heading, 2, &format!("{}{}{}", dev_table, sv_table, space_table)));
    }
    content
}

pub fn render_fstab(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    match &manifest.storage.fstab {
        Some(r) if !r.content.is_empty() => {
            let content = make_verbatim(renderer, &r.content);
            renderer.section("fstab", 2, &content)
        }
        _ => String::new(),
    }
}

pub fn render_lvm_conf(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    match &manifest.storage.lvm_conf {
        Some(r) if !r.content.is_empty() => {
            let content = make_verbatim(renderer, &r.content);
            renderer.section("LVM Configuration", 2, &content)
        }
        _ => String::new(),
    }
}

pub fn render_evms(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    match &manifest.storage.evms {
        Some(r) if !r.content.is_empty() => {
            let content = make_verbatim(renderer, &r.content);
            renderer.section("EVMS", 2, &content)
        }
        _ => String::new(),
    }
}

pub fn render_multipath(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    match &manifest.storage.multipath {
        Some(r) if !r.content.is_empty() => {
            let content = make_verbatim(renderer, &r.content);
            renderer.section("Multipath", 2, &content)
        }
        _ => String::new(),
    }
}

pub fn render_ide(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.storage.ide.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.storage.ide.elements.iter()
        .map(|r| vec![
            r.device.clone(), r.media.clone(), r.model.clone(), r.driver.clone(),
            r.geometry_phys.clone(), r.geometry_log.clone(), r.capacity_blocks.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["device", "media", "model", "driver", "geometry_phys", "geometry_log", "capacity_blocks"],
        rows);
    renderer.section("IDE Devices", 1, &content)
}

pub fn render_scsi(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.storage.scsi.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.storage.scsi.elements.iter()
        .map(|r| vec![
            r.host.clone(), r.channel.clone(), r.id.clone(), r.lun.clone(),
            r.vendor.clone(), r.model.clone(), r.revision.clone(),
            r.r#type.clone(), r.ansi_rev.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["host", "channel", "id", "lun", "vendor", "model", "revision", "type", "ansi_rev"],
        rows);
    renderer.section("SCSI Devices", 1, &content)
}

fn render_controller_scope(
    title: &str,
    scope: &ScopeWrapper<RawControllerRecord>,
    renderer: &dyn Renderer,
) -> String {
    if scope.elements.is_empty() { return String::new(); }
    let mut content = String::new();
    for rec in &scope.elements {
        content.push_str(&format!("Controller: {}\n", rec.controller_id));
        content.push_str(&make_verbatim(renderer, &rec.raw_output));
    }
    renderer.section(title, 1, &content)
}

pub fn render_cciss(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    render_controller_scope("CCISS Controllers", &manifest.storage.cciss, renderer)
}

pub fn render_areca(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    render_controller_scope("Areca Controllers", &manifest.storage.areca, renderer)
}

pub fn render_dac960(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    render_controller_scope("DAC960 Controllers", &manifest.storage.dac960, renderer)
}

pub fn render_gdth(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    render_controller_scope("GDTH Controllers", &manifest.storage.gdth, renderer)
}

pub fn render_ips(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    render_controller_scope("IPS Controllers", &manifest.storage.ips, renderer)
}

pub fn render_compaq_smart(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    render_controller_scope("Compaq Smart Array", &manifest.storage.compaq_smart, renderer)
}

pub fn render_network_interfaces(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.network.interfaces.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.network.interfaces.elements.iter()
        .map(|r| vec![
            r.ifname.clone(), r.link_type.clone(), r.address.clone(),
            r.flags.join(" "), r.mtu.to_string(), r.operstate.clone(),
            r.ip.clone(), r.prefixlen.clone(), r.broadcast.clone(),
            r.ip6.clone(), r.ip6_prefixlen.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["ifname", "link_type", "address", "flags", "mtu", "operstate",
          "ip", "prefixlen", "broadcast", "ip6", "ip6_prefixlen"],
        rows);
    renderer.section("Networking Interfaces", 1, &content)
}

pub fn render_routing(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.network.routes.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.network.routes.elements.iter()
        .map(|r| vec![
            r.dst.clone(), r.gateway.clone(), r.dev.clone(), r.protocol.clone(),
            r.scope.clone(), r.r#type.clone(), r.metric.clone(), r.flags.join(" "),
        ])
        .collect();
    let content = make_table(renderer,
        &["dst", "gateway", "dev", "protocol", "scope", "type", "metric", "flags"],
        rows);
    renderer.section("Routing Table", 1, &content)
}

pub fn render_packet_filter(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.network.packet_filter.elements.is_empty() { return String::new(); }
    let mut content = String::new();
    let header_rows: Vec<Vec<String>> = manifest.network.packet_filter.elements.iter()
        .map(|r| vec![r.engine.clone(), r.table.clone()])
        .collect();
    content.push_str(&make_table(renderer, &["engine", "table"], header_rows));
    for rec in &manifest.network.packet_filter.elements {
        if !rec.raw_output.is_empty() {
            content.push_str(&make_verbatim(renderer, &rec.raw_output));
        }
    }
    renderer.section("Packet Filter", 1, &content)
}

pub fn render_apparmor(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    match &manifest.security_apparmor {
        None => String::new(),
        Some(aa) => {
            let mut content = String::new();
            // kernel_params sub-table
            if !aa.kernel_params.elements.is_empty() {
                let rows: Vec<Vec<String>> = aa.kernel_params.elements.iter()
                    .map(|r| vec![r.key.clone(), r.value.clone()])
                    .collect();
                content.push_str(&make_table(renderer, &["key", "value"], rows));
            }
            // profiles sub-table
            if !aa.profiles.elements.is_empty() {
                let rows: Vec<Vec<String>> = aa.profiles.elements.iter()
                    .map(|r| vec![r.name.clone(), r.mode.clone()])
                    .collect();
                content.push_str(&make_table(renderer, &["name", "mode"], rows));
            }
            // config_files list
            if !aa.config_files.is_empty() {
                content.push_str(&make_verbatim(renderer, &aa.config_files.join("\n")));
            }
            if content.is_empty() { return String::new(); }
            renderer.section("AppArmor", 1, &content)
        }
    }
}

pub fn render_dmi(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    match &manifest.dmi {
        Some(d) if !d.raw_output.is_empty() => {
            let content = make_verbatim(renderer, &d.raw_output);
            renderer.section("DMI Information", 1, &content)
        }
        _ => String::new(),
    }
}

pub fn render_services(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.services.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.services.elements.iter()
        .map(|r| vec![r.name.clone(), r.state.clone()])
        .collect();
    let content = make_table(renderer, &["name", "state"], rows);
    renderer.section("Services", 1, &content)
}

pub fn render_changed_config_files(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.changed_config_files.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.changed_config_files.elements.iter()
        .map(|r| vec![
            r.name.clone(), r.package_name.clone(), r.package_version.clone(),
            r.status.clone(), r.changes.join(","), r.mode.clone(),
            r.user.clone(), r.group.clone(), r.r#type.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["name", "package_name", "package_version", "status", "changes", "mode", "user", "group", "type"],
        rows);
    renderer.section("Changed Configuration Files", 1, &content)
}

pub fn render_packages(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.packages.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.packages.elements.iter()
        .map(|r| vec![
            r.name.clone(), r.version.clone(), r.release.clone(),
            r.arch.clone(), r.size.to_string(), r.summary.clone(),
        ])
        .collect();
    let content = make_table(renderer,
        &["name", "version", "release", "arch", "size", "summary"],
        rows);
    renderer.section("Installed Packages", 1, &content)
}

pub fn render_kernel_config(manifest: &SitarManifest, renderer: &dyn Renderer) -> String {
    if manifest.kernel_config.elements.is_empty() { return String::new(); }
    let rows: Vec<Vec<String>> = manifest.kernel_config.elements.iter()
        .map(|r| vec![r.key.clone(), r.value.clone()])
        .collect();
    let content = make_table(renderer, &["key", "value"], rows);
    renderer.section("Kernel Configuration", 1, &content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interfaces::HtmlRenderer;

    fn make_manifest_with_general_info() -> SitarManifest {
        let mut m = SitarManifest::default();
        m.meta.hostname = "testhost".to_string();
        m.meta.collected_at = "2026-04-03T00:00:00Z".to_string();
        m.general_info.elements = vec![
            GeneralInfoRecord { key: "hostname".to_string(), value: "testhost".to_string() },
            GeneralInfoRecord { key: "uname".to_string(), value: "Linux testhost 5.15.0".to_string() },
        ];
        m
    }

    #[test]
    fn test_render_general_info_html() {
        let m = make_manifest_with_general_info();
        let r = HtmlRenderer;
        let output = render_general_info(&m, &r);
        assert!(output.contains("General Information"));
        assert!(output.contains("hostname"));
        assert!(output.contains("testhost"));
    }

    #[test]
    fn test_render_packages_empty() {
        let m = SitarManifest::default();
        let r = HtmlRenderer;
        let output = render_packages(&m, &r);
        assert!(output.is_empty());
    }

    #[test]
    fn test_render_services() {
        let mut m = SitarManifest::default();
        m.meta.hostname = "h".to_string();
        m.services.elements = vec![
            ServiceRecord { name: "sshd".to_string(), state: "enabled".to_string(), legacy_sysv: false },
        ];
        let r = HtmlRenderer;
        let output = render_services(&m, &r);
        assert!(output.contains("Services"));
        assert!(output.contains("sshd"));
        assert!(output.contains("enabled"));
    }
}
