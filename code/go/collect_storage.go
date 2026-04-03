package main

import (
	"encoding/json"
	"fmt"
	"math"
	"os"
	"strings"
)

// collectStorage collects partition, mount, RAID, IDE, SCSI, and controller information.
func collectStorage(fs Filesystem, cr CommandRunner) *StorageScope {
	scope := &StorageScope{
		Partitions:   &ScopeWrapper[PartitionRecord]{Attributes: map[string]interface{}{}, Elements: []PartitionRecord{}},
		SoftwareRaid: &ScopeWrapper[SoftwareRaidRecord]{Attributes: map[string]interface{}{}, Elements: []SoftwareRaidRecord{}},
		Ide:          &ScopeWrapper[IdeRecord]{Attributes: map[string]interface{}{}, Elements: []IdeRecord{}},
		Scsi:         &ScopeWrapper[ScsiRecord]{Attributes: map[string]interface{}{}, Elements: []ScsiRecord{}},
	}

	// Step 1: Collect block device info (primary: lsblk -J)
	partitions := collectPartitionsLsblk(cr)
	if len(partitions) == 0 {
		partitions = collectPartitionsFdisk(cr)
	}

	// Step 2: Collect mount options via findmnt or mount
	mountOpts := collectMountOptions(cr)
	for i := range partitions {
		if opts, ok := mountOpts[partitions[i].Device]; ok {
			partitions[i].MountOptions = opts
		}
	}

	// Step 3: df usage statistics
	dfStats := collectDfStats(cr)
	for i := range partitions {
		if stat, ok := dfStats[partitions[i].Device]; ok {
			partitions[i].DfBlocksKB = stat.blocks
			partitions[i].DfUsedKB = stat.used
			partitions[i].DfAvailKB = stat.avail
			partitions[i].DfUsePercent = stat.usePct
		}
	}

	// Step 4: ext2/3/4 attributes via tune2fs
	for i := range partitions {
		if partitions[i].FsType == "ext2" || partitions[i].FsType == "ext3" || partitions[i].FsType == "ext4" {
			collectTune2fs(cr, &partitions[i])
		}
	}

	scope.Partitions.Elements = partitions

	// Step 5: Software RAID from /proc/mdstat
	scope.SoftwareRaid.Elements = collectSoftwareRaid(fs)

	// Step 6: IDE devices
	scope.Ide.Elements = collectIDE(fs)

	// Step 7: SCSI devices
	scope.Scsi.Elements = collectSCSI(fs)

	// Step 8: RAID controllers (stubs - hardware specific)
	// Skipped if hardware not detected

	// Step 9-12: EVMS, multipath, fstab, lvm.conf
	if content, err := fs.ReadFile("/etc/fstab"); err == nil {
		scope.Fstab = &RawTextRecord{Path: "/etc/fstab", Content: content}
	}
	if content, err := fs.ReadFile("/etc/lvm/lvm.conf"); err == nil {
		scope.LvmConf = &RawTextRecord{Path: "/etc/lvm/lvm.conf", Content: content}
	}

	return scope
}

type lsblkOutput struct {
	Blockdevices []lsblkDevice `json:"blockdevices"`
}

type lsblkDevice struct {
	Name       string        `json:"name"`
	MajMin     string        `json:"maj:min"`
	Type       string        `json:"type"`
	Size       string        `json:"size"`
	FsType     string        `json:"fstype"`
	MountPoint string        `json:"mountpoint"`
	UUID       string        `json:"uuid"`
	Label      string        `json:"label"`
	RO         string        `json:"ro"`
	Children   []lsblkDevice `json:"children"`
}

func flattenLsblk(dev lsblkDevice, out *[]PartitionRecord) {
	if dev.Type == "part" || dev.Type == "disk" || dev.Type == "lvm" {
		p := PartitionRecord{
			Device:     "/dev/" + dev.Name,
			MajMin:     dev.MajMin,
			Type:       dev.Type,
			Size:       dev.Size,
			FsType:     dev.FsType,
			MountPoint: dev.MountPoint,
			UUID:       dev.UUID,
			Label:      dev.Label,
			RO:         dev.RO,
			Source:     "lsblk",
		}
		*out = append(*out, p)
	}
	for _, child := range dev.Children {
		flattenLsblk(child, out)
	}
}

func collectPartitionsLsblk(cr CommandRunner) []PartitionRecord {
	stdout, _, err := cr.Run("lsblk", []string{"-J", "-o", "NAME,MAJ:MIN,TYPE,SIZE,FSTYPE,MOUNTPOINT,UUID,LABEL,RO"})
	if err != nil || stdout == "" {
		return nil
	}
	var out lsblkOutput
	if err := json.Unmarshal([]byte(stdout), &out); err != nil {
		fmt.Fprintf(os.Stderr, "sitar: lsblk JSON parse error: %v\n", err)
		return nil
	}
	var records []PartitionRecord
	for _, dev := range out.Blockdevices {
		flattenLsblk(dev, &records)
	}
	return records
}

func collectPartitionsFdisk(cr CommandRunner) []PartitionRecord {
	stdout, _, err := cr.Run("fdisk", []string{"-l"})
	if err != nil || stdout == "" {
		return nil
	}
	var records []PartitionRecord
	for _, line := range strings.Split(stdout, "\n") {
		if !strings.HasPrefix(line, "/dev/") {
			continue
		}
		line = strings.Replace(line, "*", " ", 1)
		fields := strings.Fields(line)
		if len(fields) < 5 {
			continue
		}
		p := PartitionRecord{
			Device:      fields[0],
			BeginSector: fields[1],
			EndSector:   fields[2],
			Source:      "fdisk",
		}
		if len(fields) >= 4 {
			p.TypeID = fields[3]
		}
		if len(fields) >= 5 {
			p.PartitionType = strings.Join(fields[4:], " ")
		}
		if p.TypeID == "8e" {
			p.PartitionType = "LVM-PV"
		} else if p.TypeID == "fe" {
			p.PartitionType = "old LVM"
		}
		records = append(records, p)
	}
	return records
}

func collectMountOptions(cr CommandRunner) map[string]string {
	result := map[string]string{}
	stdout, _, err := cr.Run("findmnt", []string{"-J"})
	if err == nil && stdout != "" {
		// Parse findmnt JSON - simplified
		type fmEntry struct {
			Target  string    `json:"target"`
			Source  string    `json:"source"`
			Options string    `json:"options"`
			Children []fmEntry `json:"children"`
		}
		type fmOutput struct {
			Filesystems []fmEntry `json:"filesystems"`
		}
		var fm fmOutput
		if err := json.Unmarshal([]byte(stdout), &fm); err == nil {
			var walk func(entries []fmEntry)
			walk = func(entries []fmEntry) {
				for _, e := range entries {
					if strings.HasPrefix(e.Source, "/dev/") {
						result[e.Source] = e.Options
					}
					walk(e.Children)
				}
			}
			walk(fm.Filesystems)
		}
		return result
	}
	// Fallback: parse mount output
	stdout, _, _ = cr.Run("mount", []string{})
	for _, line := range strings.Split(stdout, "\n") {
		// format: device on mountpoint type fstype (options)
		if !strings.HasPrefix(line, "/dev/") {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) < 6 {
			continue
		}
		dev := fields[0]
		opts := strings.Trim(fields[5], "()")
		result[dev] = opts
	}
	return result
}

type dfStat struct {
	blocks int64
	used   int64
	avail  int64
	usePct string
}

func collectDfStats(cr CommandRunner) map[string]dfStat {
	result := map[string]dfStat{}
	stdout, _, err := cr.Run("df", []string{"-PPk"})
	if err != nil {
		return result
	}
	for _, line := range strings.Split(stdout, "\n") {
		if !strings.HasPrefix(line, "/dev/") {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) < 5 {
			continue
		}
		dev := fields[0]
		var stat dfStat
		if v, err := parseInt64(fields[1]); err == nil {
			stat.blocks = v
		}
		if v, err := parseInt64(fields[2]); err == nil {
			stat.used = v
		}
		if v, err := parseInt64(fields[3]); err == nil {
			stat.avail = v
		}
		if len(fields) >= 5 {
			stat.usePct = fields[4]
		}
		result[dev] = stat
	}
	return result
}

func parseInt64(s string) (int64, error) {
	var v int64
	_, err := fmt.Sscanf(s, "%d", &v)
	return v, err
}

func collectTune2fs(cr CommandRunner, p *PartitionRecord) {
	stdout, _, err := cr.Run("tune2fs", []string{"-l", p.Device})
	if err != nil {
		return
	}
	var inodeCount, blockCount, blockSize int64
	for _, line := range strings.Split(stdout, "\n") {
		idx := strings.Index(line, ":")
		if idx < 0 {
			continue
		}
		key := strings.TrimSpace(line[:idx])
		val := strings.TrimSpace(line[idx+1:])
		switch key {
		case "Reserved block count":
			p.ReservedBlocks = val
		case "Block size":
			p.BlockSize = val
			if v, err := parseInt64(val); err == nil {
				blockSize = v
			}
		case "Inode count":
			if v, err := parseInt64(val); err == nil {
				inodeCount = v
			}
		case "Block count":
			if v, err := parseInt64(val); err == nil {
				blockCount = v
			}
		case "Maximum mount count":
			p.MaxMountCount = val
		}
	}
	if inodeCount > 0 && blockCount > 0 && blockSize > 0 {
		ratio := float64(blockCount) / float64(inodeCount)
		log2 := math.Log2(ratio)
		rounded := math.Round(log2)
		density := math.Pow(2, rounded) * float64(blockSize)
		p.InodeDensity = fmt.Sprintf("%.0f", density)
	}
}

func collectSoftwareRaid(fs Filesystem) []SoftwareRaidRecord {
	content, err := fs.ReadFile("/proc/mdstat")
	if err != nil {
		return nil
	}
	var records []SoftwareRaidRecord
	lines := strings.Split(content, "\n")
	for i, line := range lines {
		if !strings.HasPrefix(line, "md") {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) < 4 || fields[1] != ":" || fields[2] != "active" {
			continue
		}
		level := fields[3]
		var parts []string
		for _, f := range fields[4:] {
			// partition references like sda1[0]
			if strings.Contains(f, "[") {
				parts = append(parts, strings.Split(f, "[")[0])
			}
		}
		r := SoftwareRaidRecord{
			Device:     fields[0],
			Level:      level,
			Partitions: parts,
		}
		// Parse next line for blocks/chunk
		if i+1 < len(lines) {
			nextLine := lines[i+1]
			nextFields := strings.Fields(nextLine)
			if len(nextFields) > 0 {
				r.Blocks = nextFields[0]
			}
			for _, f := range nextFields {
				if strings.HasPrefix(f, "chunk=") {
					r.ChunkSize = strings.TrimPrefix(f, "chunk=")
				}
				if strings.HasPrefix(f, "algorithm=") {
					r.Algorithm = strings.TrimPrefix(f, "algorithm=")
				}
			}
		}
		records = append(records, r)
	}
	return records
}

func collectIDE(fs Filesystem) []IdeRecord {
	var records []IdeRecord
	devices := []string{"hda", "hdb", "hdc", "hdd", "hde", "hdf", "hdg", "hdh", "hdi"}
	for _, dev := range devices {
		base := "/proc/ide/" + dev
		if !fs.Exists(base) {
			continue
		}
		r := IdeRecord{Device: "/dev/" + dev}
		if content, err := fs.ReadFile(base + "/media"); err == nil {
			r.Media = strings.TrimSpace(content)
		}
		if content, err := fs.ReadFile(base + "/model"); err == nil {
			r.Model = strings.TrimSpace(content)
		}
		if content, err := fs.ReadFile(base + "/driver"); err == nil {
			r.Driver = strings.TrimSpace(content)
		}
		if r.Media == "disk" {
			if content, err := fs.ReadFile(base + "/geometry"); err == nil {
				lines := strings.Split(content, "\n")
				for _, line := range lines {
					if strings.Contains(line, "physical") {
						r.GeometryPhys = strings.TrimSpace(line)
					} else if strings.Contains(line, "logical") {
						r.GeometryLog = strings.TrimSpace(line)
					}
				}
			}
			if content, err := fs.ReadFile(base + "/capacity"); err == nil {
				r.CapacityBlocks = strings.TrimSpace(content)
			}
		}
		records = append(records, r)
	}
	return records
}

func collectSCSI(fs Filesystem) []ScsiRecord {
	content, err := fs.ReadFile("/proc/scsi/scsi")
	if err != nil {
		return nil
	}
	var records []ScsiRecord
	var current *ScsiRecord
	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		if strings.HasPrefix(line, "Host:") {
			if current != nil {
				records = append(records, *current)
			}
			current = &ScsiRecord{}
			// Host: scsi0 Channel: 00 Id: 00 Lun: 00
			parts := strings.Fields(line)
			for i, p := range parts {
				switch p {
				case "Host:":
					if i+1 < len(parts) {
						current.Host = parts[i+1]
					}
				case "Channel:":
					if i+1 < len(parts) {
						current.Channel = parts[i+1]
					}
				case "Id:":
					if i+1 < len(parts) {
						current.ID = parts[i+1]
					}
				case "Lun:":
					if i+1 < len(parts) {
						current.LUN = parts[i+1]
					}
				}
			}
		} else if strings.HasPrefix(line, "Vendor:") && current != nil {
			// Vendor: ATA      Model: VBOX HARDDISK    Rev: 1.0
			parts := strings.Fields(line)
			state := ""
			for _, p := range parts {
				switch p {
				case "Vendor:":
					state = "vendor"
				case "Model:":
					state = "model"
				case "Rev:":
					state = "rev"
				default:
					switch state {
					case "vendor":
						current.Vendor += p + " "
					case "model":
						current.Model += p + " "
					case "rev":
						current.Revision += p + " "
					}
				}
			}
			current.Vendor = strings.TrimSpace(current.Vendor)
			current.Model = strings.TrimSpace(current.Model)
			current.Revision = strings.TrimSpace(current.Revision)
		} else if strings.HasPrefix(line, "Type:") && current != nil {
			parts := strings.Fields(line)
			if len(parts) >= 2 {
				current.Type = parts[1]
			}
			for i, p := range parts {
				if p == "ANSI" && i+2 < len(parts) {
					current.AnsiRev = parts[i+2]
				}
			}
		}
	}
	if current != nil {
		records = append(records, *current)
	}
	return records
}

// collectBtrfs collects btrfs filesystem details.
func collectBtrfs(fs Filesystem, cr CommandRunner, storage *StorageScope) *ScopeWrapper[BtrfsFilesystemRecord] {
	if storage == nil || storage.Partitions == nil {
		return nil
	}

	// Check if btrfs tool is available
	if !fs.IsExecutable("/sbin/btrfs") && !fs.IsExecutable("/usr/sbin/btrfs") && !fs.IsExecutable("/bin/btrfs") && !fs.IsExecutable("/usr/bin/btrfs") {
		return nil
	}

	// Find btrfs mountpoints
	var mountpoints []string
	seen := map[string]bool{}
	for _, p := range storage.Partitions.Elements {
		if p.FsType == "btrfs" && p.MountPoint != "" && !seen[p.UUID] {
			mountpoints = append(mountpoints, p.MountPoint)
			if p.UUID != "" {
				seen[p.UUID] = true
			}
		}
	}
	if len(mountpoints) == 0 {
		return nil
	}

	scope := &ScopeWrapper[BtrfsFilesystemRecord]{
		Attributes: map[string]interface{}{},
		Elements:   []BtrfsFilesystemRecord{},
	}

	for _, mp := range mountpoints {
		rec := BtrfsFilesystemRecord{MountPoint: mp}

		// btrfs filesystem show
		stdout, _, err := cr.Run("btrfs", []string{"filesystem", "show", mp})
		if err == nil {
			for _, line := range strings.Split(stdout, "\n") {
				line = strings.TrimSpace(line)
				if strings.HasPrefix(line, "Label:") {
					// Label: 'myfs'  uuid: abc-123
					parts := strings.Fields(line)
					for i, p := range parts {
						if p == "Label:" && i+1 < len(parts) {
							rec.Label = strings.Trim(parts[i+1], "'\"")
							if rec.Label == "none" {
								rec.Label = "<unlabeled>"
							}
						}
						if p == "uuid:" && i+1 < len(parts) {
							rec.UUID = parts[i+1]
						}
					}
				} else if strings.HasPrefix(line, "Total devices") {
					parts := strings.Fields(line)
					if len(parts) >= 3 {
						rec.TotalDevices = parts[2]
					}
				} else if strings.HasPrefix(line, "FS bytes used") {
					parts := strings.Fields(line)
					if len(parts) >= 4 {
						rec.BytesUsed = parts[3]
					}
				} else if strings.HasPrefix(line, "devid") {
					// devid    1 size 50.00GiB used 10.00GiB path /dev/sda2
					parts := strings.Fields(line)
					bdr := BtrfsDeviceRecord{}
					for i, p := range parts {
						switch p {
						case "devid":
							if i+1 < len(parts) {
								bdr.DevID = parts[i+1]
							}
						case "size":
							if i+1 < len(parts) {
								bdr.Size = parts[i+1]
							}
						case "used":
							if i+1 < len(parts) {
								bdr.Used = parts[i+1]
							}
						case "path":
							if i+1 < len(parts) {
								bdr.Path = parts[i+1]
							}
						}
					}
					rec.Devices = append(rec.Devices, bdr)
				}
			}
		}

		// btrfs subvolume list
		stdout, _, err = cr.Run("btrfs", []string{"subvolume", "list", mp})
		if err == nil {
			for _, line := range strings.Split(stdout, "\n") {
				// ID 256 gen 7 top level 5 path @
				parts := strings.Fields(line)
				if len(parts) < 9 {
					continue
				}
				bsr := BtrfsSubvolumeRecord{}
				for i, p := range parts {
					switch p {
					case "ID":
						if i+1 < len(parts) {
							bsr.ID = parts[i+1]
						}
					case "gen":
						if i+1 < len(parts) {
							bsr.Gen = parts[i+1]
						}
					case "level":
						if i+1 < len(parts) {
							bsr.TopLevel = parts[i+1]
						}
					case "path":
						if i+1 < len(parts) {
							bsr.Path = strings.Join(parts[i+1:], " ")
						}
					}
				}
				rec.Subvolumes = append(rec.Subvolumes, bsr)
			}
		}

		// btrfs filesystem df
		stdout, _, err = cr.Run("btrfs", []string{"filesystem", "df", mp})
		if err == nil {
			for _, line := range strings.Split(stdout, "\n") {
				// Data, single: total=1.00GiB, used=512.00MiB
				if idx := strings.Index(line, ":"); idx >= 0 {
					typePart := strings.TrimSpace(line[:idx])
					valPart := line[idx+1:]
					var total, used string
					for _, kv := range strings.Split(valPart, ",") {
						kv = strings.TrimSpace(kv)
						if strings.HasPrefix(kv, "total=") {
							total = strings.TrimPrefix(kv, "total=")
						} else if strings.HasPrefix(kv, "used=") {
							used = strings.TrimPrefix(kv, "used=")
						}
					}
					typeLower := strings.ToLower(typePart)
					if strings.HasPrefix(typeLower, "data") {
						rec.DataTotal = total
						rec.DataUsed = used
					} else if strings.HasPrefix(typeLower, "metadata") {
						rec.MetadataTotal = total
						rec.MetadataUsed = used
					} else if strings.HasPrefix(typeLower, "system") {
						rec.SystemTotal = total
						rec.SystemUsed = used
					}
				}
			}
		}

		scope.Elements = append(scope.Elements, rec)
	}

	if len(scope.Elements) == 0 {
		return nil
	}
	return scope
}
