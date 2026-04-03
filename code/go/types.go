package main

// ScopeWrapper is a generic container matching the Machinery _attributes/_elements pattern.
type ScopeWrapper[T any] struct {
	Attributes map[string]interface{} `json:"_attributes"`
	Elements   []T                    `json:"_elements"`
}

// SitarMeta holds the top-level metadata for a sitar run.
type SitarMeta struct {
	FormatVersion int    `json:"format_version"`
	SitarVersion  string `json:"sitar_version"`
	CollectedAt   string `json:"collected_at"`
	Hostname      string `json:"hostname"`
	Uname         string `json:"uname"`
}

// Config holds the resolved configuration from sysconfig file and CLI arguments.
type Config struct {
	Format           string
	Outfile          string
	Outdir           string
	FileSizeLimit    int
	CheckConsistency bool
	FindUnpacked     bool
	All              bool
	AllConfigFiles   string // "On" | "Off" | "Auto"
	AllSubdomain     string // "On" | "Off" | "Auto"
	AllSysconfig     string // "On" | "Off" | "Auto"
	GConf            bool
	LvmArchive       bool
	Exclude          []string
	Verbosity        string // "normal" | "verbose" | "debug"
	Debug            bool
}

// DistributionFamily represents the detected Linux distribution family.
type DistributionFamily string

const (
	FamilyRPM     DistributionFamily = "rpm"
	FamilyDeb     DistributionFamily = "deb"
	FamilyUnknown DistributionFamily = "unknown"
)

// PackageVersioningBackend represents the package management backend.
type PackageVersioningBackend string

const (
	BackendRPM  PackageVersioningBackend = "rpm"
	BackendDpkg PackageVersioningBackend = "dpkg"
	BackendNone PackageVersioningBackend = "none"
)

// DistroInfo holds the result of distribution detection.
type DistroInfo struct {
	Family      DistributionFamily
	Release     string
	Backend     PackageVersioningBackend
	RpmCmd      string
	DpkgStatus  string
}

// ---------------------------------------------------------------------------
// Record types
// ---------------------------------------------------------------------------

// GeneralInfoRecord holds a single key-value pair of general system info.
type GeneralInfoRecord struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

// EnvironmentScope holds environment metadata.
type EnvironmentScope struct {
	Locale     string `json:"locale"`
	SystemType string `json:"system_type"`
}

// OsScope holds OS identification.
type OsScope struct {
	Name         *string `json:"name"`
	Version      *string `json:"version"`
	Architecture *string `json:"architecture"`
}

// PackageRecord holds an installed package record (RPM or dpkg).
type PackageRecord struct {
	Name         string `json:"name"`
	Version      string `json:"version"`
	Release      string `json:"release"`
	Arch         string `json:"arch"`
	Vendor       string `json:"vendor"`
	Checksum     string `json:"checksum"`
	Size         int64  `json:"size"`
	Summary      string `json:"summary"`
	Distribution string `json:"distribution"`
	Packager     string `json:"packager"`
}

// PatternRecord holds a zypper pattern or tasksel task.
type PatternRecord struct {
	Name    string `json:"name"`
	Version string `json:"version"`
	Release string `json:"release"`
}

// RepositoryRecord holds a configured package repository.
type RepositoryRecord struct {
	Alias        string   `json:"alias,omitempty"`
	Name         string   `json:"name,omitempty"`
	URL          string   `json:"url,omitempty"`
	URLs         []string `json:"urls,omitempty"`
	Type         string   `json:"type,omitempty"`
	Enabled      string   `json:"enabled,omitempty"`
	GPGCheck     string   `json:"gpgcheck,omitempty"`
	AutoRefresh  string   `json:"autorefresh,omitempty"`
	Priority     string   `json:"priority,omitempty"`
	Distribution string   `json:"distribution,omitempty"`
	Components   []string `json:"components,omitempty"`
}

// ServiceRecord holds a system service entry.
type ServiceRecord struct {
	Name      string `json:"name"`
	State     string `json:"state"`
	LegacySysV bool  `json:"legacy_sysv,omitempty"`
}

// GroupRecord holds a system group entry.
type GroupRecord struct {
	Name     string   `json:"name"`
	Password string   `json:"password"`
	GID      *int     `json:"gid"`
	Users    []string `json:"users"`
}

// UserRecord holds a system user entry.
type UserRecord struct {
	Name              string `json:"name"`
	Password          string `json:"password"`
	UID               *int   `json:"uid"`
	GID               *int   `json:"gid"`
	Comment           string `json:"comment"`
	Home              string `json:"home"`
	Shell             string `json:"shell"`
	EncryptedPassword string `json:"encrypted_password,omitempty"`
	LastChangedDate   int    `json:"last_changed_date,omitempty"`
	MinDays           int    `json:"min_days,omitempty"`
	MaxDays           int    `json:"max_days,omitempty"`
	WarnDays          int    `json:"warn_days,omitempty"`
	DisableDays       int    `json:"disable_days,omitempty"`
	DisabledDate      int    `json:"disabled_date,omitempty"`
}

// ChangedConfigFileRecord holds a changed RPM config file entry.
type ChangedConfigFileRecord struct {
	Name           string   `json:"name"`
	PackageName    string   `json:"package_name"`
	PackageVersion string   `json:"package_version"`
	Status         string   `json:"status"`
	Changes        []string `json:"changes"`
	Mode           string   `json:"mode"`
	User           string   `json:"user"`
	Group          string   `json:"group"`
	Type           string   `json:"type"`
	ErrorMessage   string   `json:"error_message,omitempty"`
}

// ChangedManagedFileRecord holds a changed RPM non-config file entry.
type ChangedManagedFileRecord struct {
	Name           string   `json:"name"`
	PackageName    string   `json:"package_name"`
	PackageVersion string   `json:"package_version"`
	Status         string   `json:"status"`
	Changes        []string `json:"changes"`
	Mode           string   `json:"mode"`
	User           string   `json:"user"`
	Group          string   `json:"group"`
	Type           string   `json:"type"`
	ErrorMessage   string   `json:"error_message,omitempty"`
}

// UnmanagedFileRecord holds an unmanaged file entry.
type UnmanagedFileRecord struct {
	Name  string `json:"name"`
	Type  string `json:"type"`
	User  string `json:"user"`
	Group string `json:"group"`
	Size  int64  `json:"size"`
	Mode  string `json:"mode"`
	Files int    `json:"files,omitempty"`
	Dirs  int    `json:"dirs,omitempty"`
}

// CpuRecord holds information about one CPU from /proc/cpuinfo.
type CpuRecord struct {
	Processor  string `json:"processor"`
	VendorID   string `json:"vendor_id"`
	ModelName  string `json:"model_name"`
	CpuFamily  string `json:"cpu_family"`
	Model      string `json:"model"`
	Stepping   string `json:"stepping"`
	CpuMHz     string `json:"cpu_mhz"`
	CacheSize  string `json:"cache_size"`
}

// KernelParamRecord holds a single kernel parameter from /proc/sys/kernel/.
type KernelParamRecord struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

// NetParamRecord holds a single network parameter from /proc/sys/net/.
type NetParamRecord struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

// KernelConfigRecord holds a single kernel config entry.
type KernelConfigRecord struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

// DeviceRecord holds device DMA/IRQ/port assignment.
type DeviceRecord struct {
	Name  string `json:"name"`
	DMA   string `json:"dma"`
	IRQ   string `json:"irq"`
	Ports string `json:"ports"`
}

// PciRecord holds a PCI device entry.
type PciRecord struct {
	PCI     string `json:"pci"`
	Device  string `json:"device"`
	Class   string `json:"class"`
	Vendor  string `json:"vendor"`
	SVendor string `json:"svendor"`
	SDevice string `json:"sdevice"`
	Rev     string `json:"rev"`
}

// PartitionRecord holds partition/block device information.
type PartitionRecord struct {
	Device          string `json:"device"`
	MajMin          string `json:"maj_min"`
	Type            string `json:"type"`
	Size            string `json:"size"`
	FsType          string `json:"fstype"`
	MountPoint      string `json:"mountpoint"`
	UUID            string `json:"uuid"`
	Label           string `json:"label"`
	RO              string `json:"ro"`
	PartitionType   string `json:"partition_type"`
	TypeID          string `json:"type_id"`
	BeginSector     string `json:"begin_sector"`
	EndSector       string `json:"end_sector"`
	RawSizeKB       int64  `json:"raw_size_kb"`
	MountOptions    string `json:"mount_options"`
	ReservedBlocks  string `json:"reserved_blocks"`
	BlockSize       string `json:"block_size"`
	InodeDensity    string `json:"inode_density"`
	MaxMountCount   string `json:"max_mount_count"`
	DfBlocksKB      int64  `json:"df_blocks_kb"`
	DfUsedKB        int64  `json:"df_used_kb"`
	DfAvailKB       int64  `json:"df_avail_kb"`
	DfUsePercent    string `json:"df_use_percent"`
	Source          string `json:"source"`
}

// SoftwareRaidRecord holds software RAID information.
type SoftwareRaidRecord struct {
	Device     string   `json:"device"`
	Level      string   `json:"level"`
	Partitions []string `json:"partitions"`
	Blocks     string   `json:"blocks"`
	ChunkSize  string   `json:"chunk_size"`
	Algorithm  string   `json:"algorithm"`
}

// IdeRecord holds IDE device information.
type IdeRecord struct {
	Device          string `json:"device"`
	Media           string `json:"media"`
	Model           string `json:"model"`
	Driver          string `json:"driver"`
	GeometryPhys    string `json:"geometry_phys"`
	GeometryLog     string `json:"geometry_log"`
	CapacityBlocks  string `json:"capacity_blocks"`
}

// ScsiRecord holds SCSI device information.
type ScsiRecord struct {
	Host     string `json:"host"`
	Channel  string `json:"channel"`
	ID       string `json:"id"`
	LUN      string `json:"lun"`
	Vendor   string `json:"vendor"`
	Model    string `json:"model"`
	Revision string `json:"revision"`
	Type     string `json:"type"`
	AnsiRev  string `json:"ansi_rev"`
}

// BtrfsDeviceRecord holds a btrfs device member entry.
type BtrfsDeviceRecord struct {
	DevID string `json:"devid"`
	Size  string `json:"size"`
	Used  string `json:"used"`
	Path  string `json:"path"`
}

// BtrfsSubvolumeRecord holds a btrfs subvolume entry.
type BtrfsSubvolumeRecord struct {
	ID       string `json:"id"`
	Gen      string `json:"gen"`
	TopLevel string `json:"top_level"`
	Path     string `json:"path"`
}

// BtrfsFilesystemRecord holds btrfs filesystem information.
type BtrfsFilesystemRecord struct {
	Label         string                 `json:"label"`
	UUID          string                 `json:"uuid"`
	TotalDevices  string                 `json:"total_devices"`
	BytesUsed     string                 `json:"bytes_used"`
	MountPoint    string                 `json:"mount_point"`
	Devices       []BtrfsDeviceRecord    `json:"devices"`
	Subvolumes    []BtrfsSubvolumeRecord `json:"subvolumes"`
	DataTotal     string                 `json:"data_total"`
	DataUsed      string                 `json:"data_used"`
	MetadataTotal string                 `json:"metadata_total"`
	MetadataUsed  string                 `json:"metadata_used"`
	SystemTotal   string                 `json:"system_total"`
	SystemUsed    string                 `json:"system_used"`
}

// RawControllerRecord holds raw RAID controller output.
type RawControllerRecord struct {
	ControllerID string `json:"controller_id"`
	RawOutput    string `json:"raw_output"`
}

// RawTextRecord holds verbatim file or command output.
type RawTextRecord struct {
	Path    string `json:"path"`
	Content string `json:"content"`
}

// StorageScope holds all storage-related information.
type StorageScope struct {
	Partitions   *ScopeWrapper[PartitionRecord]      `json:"partitions,omitempty"`
	SoftwareRaid *ScopeWrapper[SoftwareRaidRecord]   `json:"software_raid,omitempty"`
	Btrfs        *ScopeWrapper[BtrfsFilesystemRecord] `json:"btrfs,omitempty"`
	Ide          *ScopeWrapper[IdeRecord]             `json:"ide,omitempty"`
	Scsi         *ScopeWrapper[ScsiRecord]            `json:"scsi,omitempty"`
	Cciss        *ScopeWrapper[RawControllerRecord]   `json:"cciss,omitempty"`
	Areca        *ScopeWrapper[RawControllerRecord]   `json:"areca,omitempty"`
	Dac960       *ScopeWrapper[RawControllerRecord]   `json:"dac960,omitempty"`
	Gdth         *ScopeWrapper[RawControllerRecord]   `json:"gdth,omitempty"`
	Ips          *ScopeWrapper[RawControllerRecord]   `json:"ips,omitempty"`
	CompaqSmart  *ScopeWrapper[RawControllerRecord]   `json:"compaq_smart,omitempty"`
	Evms         *RawTextRecord                       `json:"evms,omitempty"`
	Multipath    *RawTextRecord                       `json:"multipath,omitempty"`
	Fstab        *RawTextRecord                       `json:"fstab,omitempty"`
	LvmConf      *RawTextRecord                       `json:"lvm_conf,omitempty"`
}

// NetworkInterfaceRecord holds network interface information.
type NetworkInterfaceRecord struct {
	IfName       string   `json:"ifname"`
	LinkType     string   `json:"link_type"`
	Address      string   `json:"address"`
	Flags        []string `json:"flags"`
	MTU          int      `json:"mtu"`
	OperState    string   `json:"operstate"`
	IP           string   `json:"ip"`
	PrefixLen    string   `json:"prefixlen"`
	Broadcast    string   `json:"broadcast"`
	IP6          string   `json:"ip6"`
	IP6PrefixLen string   `json:"ip6_prefixlen"`
}

// RouteRecord holds a routing table entry.
type RouteRecord struct {
	Dst      string   `json:"dst"`
	Gateway  string   `json:"gateway"`
	Dev      string   `json:"dev"`
	Protocol string   `json:"protocol"`
	Scope    string   `json:"scope"`
	Type     string   `json:"type"`
	Metric   string   `json:"metric"`
	Flags    []string `json:"flags"`
}

// PacketFilterRecord holds packet filter rules.
type PacketFilterRecord struct {
	Engine    string `json:"engine"`
	Table     string `json:"table"`
	RawOutput string `json:"raw_output"`
}

// NetworkScope holds all network-related information.
type NetworkScope struct {
	Interfaces   *ScopeWrapper[NetworkInterfaceRecord] `json:"interfaces,omitempty"`
	Routes       *ScopeWrapper[RouteRecord]             `json:"routes,omitempty"`
	PacketFilter *ScopeWrapper[PacketFilterRecord]      `json:"packet_filter,omitempty"`
}

// ApparmorProfileRecord holds an AppArmor profile entry.
type ApparmorProfileRecord struct {
	Name string `json:"name"`
	Mode string `json:"mode"`
}

// ApparmorKernelRecord holds an AppArmor kernel parameter.
type ApparmorKernelRecord struct {
	Key   string `json:"key"`
	Value string `json:"value"`
}

// SecurityApparmorScope holds AppArmor security information.
type SecurityApparmorScope struct {
	KernelParams *ScopeWrapper[ApparmorKernelRecord]  `json:"kernel_params,omitempty"`
	Profiles     *ScopeWrapper[ApparmorProfileRecord] `json:"profiles,omitempty"`
	ConfigFiles  []string                              `json:"config_files,omitempty"`
}

// ProcessRecord holds a process entry from /proc.
type ProcessRecord struct {
	PID     string `json:"pid"`
	PPID    string `json:"ppid"`
	Comm    string `json:"comm"`
	State   string `json:"state"`
	CmdLine string `json:"cmdline"`
}

// DmiScope holds DMI/SMBIOS information.
type DmiScope struct {
	RawOutput string `json:"raw_output"`
}

// CrontabRecord holds a crontab entry.
type CrontabRecord struct {
	Minute      string `json:"minute"`
	Hour        string `json:"hour"`
	DayOfMonth  string `json:"day_of_month"`
	Month       string `json:"month"`
	DayOfWeek   string `json:"day_of_week"`
	User        string `json:"user"`
	Command     string `json:"command"`
	Source      string `json:"source"`
}

// ChangedFileRecord describes a file whose on-disk state differs from the
// package database's recorded state (used by PackageBackend interface).
type ChangedFileRecord struct {
	Path        string
	PackageName string
	Changes     []string
	Status      string
	ErrorMsg    string
}

// SitarManifest is the top-level data structure for all collected system information.
type SitarManifest struct {
	Meta                SitarMeta                               `json:"meta"`
	GeneralInfo         *ScopeWrapper[GeneralInfoRecord]        `json:"general_info,omitempty"`
	Environment         *EnvironmentScope                       `json:"environment,omitempty"`
	Os                  *OsScope                                `json:"os,omitempty"`
	Packages            *ScopeWrapper[PackageRecord]            `json:"packages,omitempty"`
	Patterns            *ScopeWrapper[PatternRecord]            `json:"patterns,omitempty"`
	Repositories        *ScopeWrapper[RepositoryRecord]         `json:"repositories,omitempty"`
	Services            *ScopeWrapper[ServiceRecord]            `json:"services,omitempty"`
	Groups              *ScopeWrapper[GroupRecord]              `json:"groups,omitempty"`
	Users               *ScopeWrapper[UserRecord]               `json:"users,omitempty"`
	ChangedConfigFiles  *ScopeWrapper[ChangedConfigFileRecord]  `json:"changed_config_files,omitempty"`
	ChangedManagedFiles *ScopeWrapper[ChangedManagedFileRecord] `json:"changed_managed_files,omitempty"`
	UnmanagedFiles      *ScopeWrapper[UnmanagedFileRecord]      `json:"unmanaged_files,omitempty"`
	CPU                 *ScopeWrapper[CpuRecord]                `json:"cpu,omitempty"`
	KernelParams        *ScopeWrapper[KernelParamRecord]        `json:"kernel_params,omitempty"`
	NetParams           *ScopeWrapper[NetParamRecord]           `json:"net_params,omitempty"`
	KernelConfig        *ScopeWrapper[KernelConfigRecord]       `json:"kernel_config,omitempty"`
	Devices             *ScopeWrapper[DeviceRecord]             `json:"devices,omitempty"`
	PCI                 *ScopeWrapper[PciRecord]                `json:"pci,omitempty"`
	Storage             *StorageScope                           `json:"storage,omitempty"`
	Network             *NetworkScope                           `json:"network,omitempty"`
	SecurityApparmor    *SecurityApparmorScope                  `json:"security_apparmor,omitempty"`
	Processes           *ScopeWrapper[ProcessRecord]            `json:"processes,omitempty"`
	DMI                 *DmiScope                               `json:"dmi,omitempty"`
}
