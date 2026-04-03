package main

import (
	"fmt"
	"os"
	"strings"
)

// renderHuman renders a SitarManifest to a human-readable format file.
func renderHuman(manifest *SitarManifest, format string, outpath string) error {
	var r SitarRenderer
	switch format {
	case "html":
		r = &HTMLRenderer{}
	case "tex":
		r = &TeXRenderer{}
	case "sdocbook":
		r = &DocBookRenderer{}
	case "markdown":
		r = &MarkdownRenderer{}
	default:
		return fmt.Errorf("unknown format: %s", format)
	}

	f, err := os.Create(outpath)
	if err != nil {
		return fmt.Errorf("create %s: %w", outpath, err)
	}
	defer f.Close()

	var sb strings.Builder
	sb.WriteString(r.Header(manifest))

	// Build section list for TOC
	var sections []string
	sectionFuncs := []struct {
		title string
		fn    func(*SitarManifest, SitarRenderer) string
	}{
		{"General Information", renderGeneralInfo},
		{"CPU", renderCPU},
		{"Kernel Parameters", renderKernelParams},
		{"Network Parameters", renderNetParams},
		{"Devices", renderDevices},
		{"PCI Devices", renderPCI},
		{"Software RAID", renderSoftwareRAID},
		{"Partitions, Mounts, LVM", renderPartitions},
		{"Btrfs Filesystems", renderBtrfs},
		{"Fstab", renderFstab},
		{"LVM Configuration", renderLvmConf},
		{"EVMS", renderEVMS},
		{"Multipath", renderMultipath},
		{"IDE Devices", renderIDE},
		{"SCSI Devices", renderSCSI},
		{"CCISS Controllers", renderCCISS},
		{"Areca Controllers", renderAreca},
		{"DAC960 Controllers", renderDAC960},
		{"GDTH Controllers", renderGDTH},
		{"IPS Controllers", renderIPS},
		{"Compaq Smart Array", renderCompaqSmart},
		{"Networking Interfaces", renderNetworkInterfaces},
		{"Routing Table", renderRouting},
		{"Packet Filter", renderPacketFilter},
		{"AppArmor Security", renderAppArmor},
		{"DMI/BIOS Information", renderDMI},
		{"Services", renderServices},
		{"Changed Config Files", renderChangedConfigFiles},
		{"Installed Packages", renderPackages},
		{"Kernel Configuration", renderKernelConfig},
	}

	for _, s := range sectionFuncs {
		sections = append(sections, s.title)
	}
	sb.WriteString(r.TOC(sections))

	for _, s := range sectionFuncs {
		content := s.fn(manifest, r)
		if content != "" {
			sb.WriteString(r.Section(s.title, 1, content))
		}
	}

	sb.WriteString(r.Footer())

	body := sb.String()
	if _, err := f.WriteString(body); err != nil {
		return fmt.Errorf("write %s: %w", outpath, err)
	}
	return nil
}

// renderTable produces a format-specific table from headers and rows.
func renderTable(r SitarRenderer, headers []string, rows [][]string) string {
	if len(rows) == 0 {
		return ""
	}
	switch r.(type) {
	case *HTMLRenderer:
		return renderTableHTML(r, headers, rows)
	case *TeXRenderer:
		return renderTableTeX(r, headers, rows)
	case *DocBookRenderer:
		return renderTableDocBook(r, headers, rows)
	case *MarkdownRenderer:
		return renderTableMarkdown(r, headers, rows)
	default:
		return renderTableMarkdown(r, headers, rows)
	}
}

func renderTableHTML(r SitarRenderer, headers []string, rows [][]string) string {
	var sb strings.Builder
	sb.WriteString("<table>\n<thead><tr>")
	for _, h := range headers {
		sb.WriteString("<th>" + r.Escape(h) + "</th>")
	}
	sb.WriteString("</tr></thead>\n<tbody>\n")
	for _, row := range rows {
		sb.WriteString("<tr>")
		for _, cell := range row {
			sb.WriteString("<td>" + r.Escape(cell) + "</td>")
		}
		sb.WriteString("</tr>\n")
	}
	sb.WriteString("</tbody></table>\n")
	return sb.String()
}

func renderTableTeX(r SitarRenderer, headers []string, rows [][]string) string {
	var sb strings.Builder
	cols := strings.Repeat("l", len(headers))
	sb.WriteString("\\begin{longtable}{" + cols + "}\n")
	sb.WriteString("\\hline\n")
	var hcells []string
	for _, h := range headers {
		hcells = append(hcells, r.Escape(h))
	}
	sb.WriteString(strings.Join(hcells, " & ") + " \\\\\n")
	sb.WriteString("\\hline\n\\endhead\n")
	for _, row := range rows {
		var cells []string
		for _, cell := range row {
			cells = append(cells, r.Escape(cell))
		}
		sb.WriteString(strings.Join(cells, " & ") + " \\\\\n")
	}
	sb.WriteString("\\hline\n\\end{longtable}\n")
	return sb.String()
}

func renderTableDocBook(r SitarRenderer, headers []string, rows [][]string) string {
	var sb strings.Builder
	sb.WriteString("<informaltable><tgroup cols=\"" + fmt.Sprintf("%d", len(headers)) + "\">\n")
	sb.WriteString("<thead><row>")
	for _, h := range headers {
		sb.WriteString("<entry>" + r.Escape(h) + "</entry>")
	}
	sb.WriteString("</row></thead>\n<tbody>\n")
	for _, row := range rows {
		sb.WriteString("<row>")
		for _, cell := range row {
			sb.WriteString("<entry>" + r.Escape(cell) + "</entry>")
		}
		sb.WriteString("</row>\n")
	}
	sb.WriteString("</tbody></tgroup></informaltable>\n")
	return sb.String()
}

func renderTableMarkdown(r SitarRenderer, headers []string, rows [][]string) string {
	var sb strings.Builder
	sb.WriteString("| " + strings.Join(escapeRow(r, headers), " | ") + " |\n")
	var sep []string
	for range headers {
		sep = append(sep, "---")
	}
	sb.WriteString("| " + strings.Join(sep, " | ") + " |\n")
	for _, row := range rows {
		sb.WriteString("| " + strings.Join(escapeRow(r, row), " | ") + " |\n")
	}
	return sb.String()
}

func escapeRow(r SitarRenderer, cells []string) []string {
	var out []string
	for _, c := range cells {
		out = append(out, r.Escape(c))
	}
	return out
}

func renderVerbatim(r SitarRenderer, content string) string {
	switch r.(type) {
	case *HTMLRenderer:
		return "<pre>" + r.Escape(content) + "</pre>\n"
	case *TeXRenderer:
		return "\\begin{verbatim}\n" + content + "\n\\end{verbatim}\n"
	case *DocBookRenderer:
		return "<screen>" + r.Escape(content) + "</screen>\n"
	case *MarkdownRenderer:
		return "```\n" + content + "\n```\n"
	default:
		return content
	}
}

// ---------------------------------------------------------------------------
// The 30 named render functions (SECTION-MAP order)
// ---------------------------------------------------------------------------

func renderGeneralInfo(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.GeneralInfo == nil || len(manifest.GeneralInfo.Elements) == 0 {
		return ""
	}
	headers := []string{"key", "value"}
	var rows [][]string
	for _, rec := range manifest.GeneralInfo.Elements {
		rows = append(rows, []string{rec.Key, rec.Value})
	}
	return renderTable(r, headers, rows)
}

func renderCPU(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.CPU == nil || len(manifest.CPU.Elements) == 0 {
		return ""
	}
	headers := []string{"processor", "vendor_id", "model_name", "cpu_family", "model", "stepping", "cpu_mhz", "cache_size"}
	var rows [][]string
	for _, rec := range manifest.CPU.Elements {
		rows = append(rows, []string{
			rec.Processor, rec.VendorID, rec.ModelName,
			rec.CpuFamily, rec.Model, rec.Stepping,
			rec.CpuMHz, rec.CacheSize,
		})
	}
	return renderTable(r, headers, rows)
}

func renderKernelParams(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.KernelParams == nil || len(manifest.KernelParams.Elements) == 0 {
		return ""
	}
	headers := []string{"key", "value"}
	var rows [][]string
	for _, rec := range manifest.KernelParams.Elements {
		rows = append(rows, []string{rec.Key, rec.Value})
	}
	return renderTable(r, headers, rows)
}

func renderNetParams(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.NetParams == nil || len(manifest.NetParams.Elements) == 0 {
		return ""
	}
	headers := []string{"key", "value"}
	var rows [][]string
	for _, rec := range manifest.NetParams.Elements {
		rows = append(rows, []string{rec.Key, rec.Value})
	}
	return renderTable(r, headers, rows)
}

func renderDevices(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Devices == nil || len(manifest.Devices.Elements) == 0 {
		return ""
	}
	headers := []string{"name", "dma", "irq", "ports"}
	var rows [][]string
	for _, rec := range manifest.Devices.Elements {
		rows = append(rows, []string{rec.Name, rec.DMA, rec.IRQ, rec.Ports})
	}
	return renderTable(r, headers, rows)
}

func renderPCI(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.PCI == nil || len(manifest.PCI.Elements) == 0 {
		return ""
	}
	headers := []string{"pci", "device", "class", "vendor", "svendor", "sdevice", "rev"}
	var rows [][]string
	for _, rec := range manifest.PCI.Elements {
		rows = append(rows, []string{
			rec.PCI, rec.Device, rec.Class, rec.Vendor,
			rec.SVendor, rec.SDevice, rec.Rev,
		})
	}
	return renderTable(r, headers, rows)
}

func renderSoftwareRAID(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.SoftwareRaid == nil || len(manifest.Storage.SoftwareRaid.Elements) == 0 {
		return ""
	}
	headers := []string{"device", "level", "partitions", "blocks", "chunk_size", "algorithm"}
	var rows [][]string
	for _, rec := range manifest.Storage.SoftwareRaid.Elements {
		rows = append(rows, []string{
			rec.Device, rec.Level,
			strings.Join(rec.Partitions, ","),
			rec.Blocks, rec.ChunkSize, rec.Algorithm,
		})
	}
	return renderTable(r, headers, rows)
}

func renderPartitions(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.Partitions == nil || len(manifest.Storage.Partitions.Elements) == 0 {
		return ""
	}
	headers := []string{"device", "type", "size", "fstype", "mountpoint", "uuid", "label", "source",
		"begin_sector", "end_sector", "mount_options", "block_size", "inode_density",
		"max_mount_count", "df_blocks_kb", "df_used_kb", "df_avail_kb", "df_use_percent"}
	var rows [][]string
	for _, rec := range manifest.Storage.Partitions.Elements {
		rows = append(rows, []string{
			rec.Device, rec.Type, rec.Size, rec.FsType, rec.MountPoint,
			rec.UUID, rec.Label, rec.Source, rec.BeginSector, rec.EndSector,
			rec.MountOptions, rec.BlockSize, rec.InodeDensity, rec.MaxMountCount,
			fmt.Sprintf("%d", rec.DfBlocksKB), fmt.Sprintf("%d", rec.DfUsedKB),
			fmt.Sprintf("%d", rec.DfAvailKB), rec.DfUsePercent,
		})
	}
	return renderTable(r, headers, rows)
}

func renderBtrfs(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.Btrfs == nil || len(manifest.Storage.Btrfs.Elements) == 0 {
		return ""
	}
	var sb strings.Builder
	for _, rec := range manifest.Storage.Btrfs.Elements {
		sb.WriteString(fmt.Sprintf("Btrfs: %s (%s)\n", rec.Label, rec.UUID))
		// Devices sub-table
		if len(rec.Devices) > 0 {
			var rows [][]string
			for _, d := range rec.Devices {
				rows = append(rows, []string{d.DevID, d.Size, d.Used, d.Path})
			}
			sb.WriteString(renderTable(r, []string{"devid", "size", "used", "path"}, rows))
		}
		// Subvolumes sub-table
		if len(rec.Subvolumes) > 0 {
			var rows [][]string
			for _, s := range rec.Subvolumes {
				rows = append(rows, []string{s.ID, s.Gen, s.TopLevel, s.Path})
			}
			sb.WriteString(renderTable(r, []string{"id", "gen", "top_level", "path"}, rows))
		}
		// Space usage
		spaceRows := [][]string{
			{"data_total", rec.DataTotal},
			{"data_used", rec.DataUsed},
			{"metadata_total", rec.MetadataTotal},
			{"metadata_used", rec.MetadataUsed},
			{"system_total", rec.SystemTotal},
			{"system_used", rec.SystemUsed},
		}
		sb.WriteString(renderTable(r, []string{"metric", "value"}, spaceRows))
	}
	return sb.String()
}

func renderFstab(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.Fstab == nil {
		return ""
	}
	return renderVerbatim(r, manifest.Storage.Fstab.Content)
}

func renderLvmConf(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.LvmConf == nil {
		return ""
	}
	return renderVerbatim(r, manifest.Storage.LvmConf.Content)
}

func renderEVMS(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.Evms == nil {
		return ""
	}
	return renderVerbatim(r, manifest.Storage.Evms.Content)
}

func renderMultipath(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.Multipath == nil {
		return ""
	}
	return renderVerbatim(r, manifest.Storage.Multipath.Content)
}

func renderIDE(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.Ide == nil || len(manifest.Storage.Ide.Elements) == 0 {
		return ""
	}
	headers := []string{"device", "media", "model", "driver", "geometry_phys", "geometry_log", "capacity_blocks"}
	var rows [][]string
	for _, rec := range manifest.Storage.Ide.Elements {
		rows = append(rows, []string{
			rec.Device, rec.Media, rec.Model, rec.Driver,
			rec.GeometryPhys, rec.GeometryLog, rec.CapacityBlocks,
		})
	}
	return renderTable(r, headers, rows)
}

func renderSCSI(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil || manifest.Storage.Scsi == nil || len(manifest.Storage.Scsi.Elements) == 0 {
		return ""
	}
	headers := []string{"host", "channel", "id", "lun", "vendor", "model", "revision", "type", "ansi_rev"}
	var rows [][]string
	for _, rec := range manifest.Storage.Scsi.Elements {
		rows = append(rows, []string{
			rec.Host, rec.Channel, rec.ID, rec.LUN,
			rec.Vendor, rec.Model, rec.Revision, rec.Type, rec.AnsiRev,
		})
	}
	return renderTable(r, headers, rows)
}

func renderControllerScope(scope *ScopeWrapper[RawControllerRecord], r SitarRenderer) string {
	if scope == nil || len(scope.Elements) == 0 {
		return ""
	}
	var sb strings.Builder
	for _, rec := range scope.Elements {
		sb.WriteString("Controller: " + rec.ControllerID + "\n")
		sb.WriteString(renderVerbatim(r, rec.RawOutput))
	}
	return sb.String()
}

func renderCCISS(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil {
		return ""
	}
	return renderControllerScope(manifest.Storage.Cciss, r)
}

func renderAreca(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil {
		return ""
	}
	return renderControllerScope(manifest.Storage.Areca, r)
}

func renderDAC960(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil {
		return ""
	}
	return renderControllerScope(manifest.Storage.Dac960, r)
}

func renderGDTH(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil {
		return ""
	}
	return renderControllerScope(manifest.Storage.Gdth, r)
}

func renderIPS(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil {
		return ""
	}
	return renderControllerScope(manifest.Storage.Ips, r)
}

func renderCompaqSmart(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Storage == nil {
		return ""
	}
	return renderControllerScope(manifest.Storage.CompaqSmart, r)
}

func renderNetworkInterfaces(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Network == nil || manifest.Network.Interfaces == nil || len(manifest.Network.Interfaces.Elements) == 0 {
		return ""
	}
	headers := []string{"ifname", "link_type", "address", "flags", "mtu", "operstate", "ip", "prefixlen", "broadcast", "ip6", "ip6_prefixlen"}
	var rows [][]string
	for _, rec := range manifest.Network.Interfaces.Elements {
		rows = append(rows, []string{
			rec.IfName, rec.LinkType, rec.Address,
			strings.Join(rec.Flags, " "),
			fmt.Sprintf("%d", rec.MTU),
			rec.OperState, rec.IP, rec.PrefixLen, rec.Broadcast,
			rec.IP6, rec.IP6PrefixLen,
		})
	}
	return renderTable(r, headers, rows)
}

func renderRouting(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Network == nil || manifest.Network.Routes == nil || len(manifest.Network.Routes.Elements) == 0 {
		return ""
	}
	headers := []string{"dst", "gateway", "dev", "protocol", "scope", "type", "metric", "flags"}
	var rows [][]string
	for _, rec := range manifest.Network.Routes.Elements {
		rows = append(rows, []string{
			rec.Dst, rec.Gateway, rec.Dev, rec.Protocol,
			rec.Scope, rec.Type, rec.Metric,
			strings.Join(rec.Flags, " "),
		})
	}
	return renderTable(r, headers, rows)
}

func renderPacketFilter(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Network == nil || manifest.Network.PacketFilter == nil || len(manifest.Network.PacketFilter.Elements) == 0 {
		return ""
	}
	var sb strings.Builder
	for _, rec := range manifest.Network.PacketFilter.Elements {
		sb.WriteString("Engine: " + rec.Engine)
		if rec.Table != "" {
			sb.WriteString(", Table: " + rec.Table)
		}
		sb.WriteString("\n")
		if rec.RawOutput != "" {
			sb.WriteString(renderVerbatim(r, rec.RawOutput))
		}
	}
	return sb.String()
}

func renderAppArmor(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.SecurityApparmor == nil {
		return ""
	}
	var sb strings.Builder
	if manifest.SecurityApparmor.KernelParams != nil && len(manifest.SecurityApparmor.KernelParams.Elements) > 0 {
		var rows [][]string
		for _, rec := range manifest.SecurityApparmor.KernelParams.Elements {
			rows = append(rows, []string{rec.Key, rec.Value})
		}
		sb.WriteString(renderTable(r, []string{"key", "value"}, rows))
	}
	if manifest.SecurityApparmor.Profiles != nil && len(manifest.SecurityApparmor.Profiles.Elements) > 0 {
		var rows [][]string
		for _, rec := range manifest.SecurityApparmor.Profiles.Elements {
			rows = append(rows, []string{rec.Name, rec.Mode})
		}
		sb.WriteString(renderTable(r, []string{"name", "mode"}, rows))
	}
	if len(manifest.SecurityApparmor.ConfigFiles) > 0 {
		sb.WriteString("Config files: " + strings.Join(manifest.SecurityApparmor.ConfigFiles, ", ") + "\n")
	}
	return sb.String()
}

func renderDMI(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.DMI == nil {
		return ""
	}
	return renderVerbatim(r, manifest.DMI.RawOutput)
}

func renderServices(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Services == nil || len(manifest.Services.Elements) == 0 {
		return ""
	}
	headers := []string{"name", "state"}
	var rows [][]string
	for _, rec := range manifest.Services.Elements {
		rows = append(rows, []string{rec.Name, rec.State})
	}
	return renderTable(r, headers, rows)
}

func renderChangedConfigFiles(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.ChangedConfigFiles == nil || len(manifest.ChangedConfigFiles.Elements) == 0 {
		return ""
	}
	headers := []string{"name", "package_name", "package_version", "status", "changes", "mode", "user", "group", "type"}
	var rows [][]string
	for _, rec := range manifest.ChangedConfigFiles.Elements {
		rows = append(rows, []string{
			rec.Name, rec.PackageName, rec.PackageVersion, rec.Status,
			strings.Join(rec.Changes, ","), rec.Mode, rec.User, rec.Group, rec.Type,
		})
	}
	return renderTable(r, headers, rows)
}

func renderPackages(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.Packages == nil || len(manifest.Packages.Elements) == 0 {
		return ""
	}
	headers := []string{"name", "version", "release", "arch", "size", "summary"}
	var rows [][]string
	for _, rec := range manifest.Packages.Elements {
		rows = append(rows, []string{
			rec.Name, rec.Version, rec.Release, rec.Arch,
			fmt.Sprintf("%d", rec.Size), rec.Summary,
		})
	}
	return renderTable(r, headers, rows)
}

func renderKernelConfig(manifest *SitarManifest, r SitarRenderer) string {
	if manifest.KernelConfig == nil || len(manifest.KernelConfig.Elements) == 0 {
		return ""
	}
	headers := []string{"key", "value"}
	var rows [][]string
	for _, rec := range manifest.KernelConfig.Elements {
		rows = append(rows, []string{rec.Key, rec.Value})
	}
	return renderTable(r, headers, rows)
}
