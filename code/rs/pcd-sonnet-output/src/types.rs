// types.rs — All struct and enum definitions for sitar
// Derived from sitar.md TYPES section
#![allow(dead_code)]

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ScopeWrapper<T> — Machinery-compatible generic wrapper
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScopeWrapper<T> {
    #[serde(rename = "_attributes")]
    pub attributes: HashMap<String, serde_json::Value>,
    #[serde(rename = "_elements")]
    pub elements: Vec<T>,
}

impl<T> Default for ScopeWrapper<T> {
    fn default() -> Self {
        Self {
            attributes: HashMap::new(),
            elements: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// OutputFormat
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Html,
    Tex,
    Sdocbook,
    Json,
    Markdown,
    All,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<OutputFormat> {
        match s.to_lowercase().as_str() {
            "html"     => Some(OutputFormat::Html),
            "tex"      => Some(OutputFormat::Tex),
            "sdocbook" => Some(OutputFormat::Sdocbook),
            "json"     => Some(OutputFormat::Json),
            "markdown" => Some(OutputFormat::Markdown),
            "all"      => Some(OutputFormat::All),
            ""         => Some(OutputFormat::All),
            _          => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Html     => ".html",
            OutputFormat::Tex      => ".tex",
            OutputFormat::Sdocbook => ".sdocbook.xml",
            OutputFormat::Json     => ".json",
            OutputFormat::Markdown => ".md",
            OutputFormat::All      => "",
        }
    }
}

// ---------------------------------------------------------------------------
// Verbosity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Verbosity {
    Normal,
    Verbose,
    Debug,
}

impl Default for Verbosity {
    fn default() -> Self { Verbosity::Normal }
}

// ---------------------------------------------------------------------------
// DistributionFamily
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum DistributionFamily {
    Rpm,
    Deb,
    Unknown,
}

// ---------------------------------------------------------------------------
// PackageVersioningBackend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum PackageVersioningBackend {
    Rpm,
    Dpkg,
    None,
}

// ---------------------------------------------------------------------------
// Config — output of prepare-config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Config {
    pub format:            Option<OutputFormat>,
    pub outfile:           String,
    pub outdir:            String,
    pub file_size_limit:   u64,
    pub check_consistency: bool,
    pub find_unpacked:     bool,
    pub all:               bool,
    pub allconfigfiles:    String,   // "On" | "Off" | "Auto"
    pub allsubdomain:      String,   // "On" | "Off" | "Auto"
    pub allsysconfig:      String,   // "On" | "Off" | "Auto"
    pub gconf:             bool,
    pub lvmarchive:        bool,
    pub exclude:           Vec<String>,
    pub verbosity:         Verbosity,
    pub debug:             bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            format:            None,
            outfile:           String::new(),
            outdir:            String::new(),
            file_size_limit:   700_000,
            check_consistency: false,
            find_unpacked:     false,
            all:               false,
            allconfigfiles:    "Auto".to_string(),
            allsubdomain:      "Auto".to_string(),
            allsysconfig:      "Auto".to_string(),
            gconf:             false,
            lvmarchive:        false,
            exclude:           vec!["/etc/shadow".to_string()],
            verbosity:         Verbosity::Normal,
            debug:             false,
        }
    }
}

// ---------------------------------------------------------------------------
// DistributionInfo — output of detect-distribution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct DistributionInfo {
    pub family:      DistributionFamily,
    pub release:     String,
    pub backend:     PackageVersioningBackend,
    pub rpm_cmd:     String,
    pub dpkg_status: String,
}

// ---------------------------------------------------------------------------
// SitarMeta
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SitarMeta {
    pub format_version: u32,
    pub sitar_version:  String,
    pub collected_at:   String,
    pub hostname:       String,
    pub uname:          String,
}

// ---------------------------------------------------------------------------
// EnvironmentScope
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct EnvironmentScope {
    pub locale:      String,
    pub system_type: String,
}

// ---------------------------------------------------------------------------
// OsScope
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OsScope {
    pub name:         Option<String>,
    pub version:      Option<String>,
    pub architecture: Option<String>,
}

// ---------------------------------------------------------------------------
// PackageRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PackageRecord {
    pub name:         String,
    pub version:      String,
    pub release:      String,
    pub arch:         String,
    pub vendor:       String,
    pub checksum:     String,
    pub size:         i64,
    pub summary:      String,
    pub distribution: String,
    pub packager:     String,
}

// ---------------------------------------------------------------------------
// PatternRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PatternRecord {
    pub name:    String,
    pub version: String,
    pub release: String,
}

// ---------------------------------------------------------------------------
// RepositoryRecord — zypp/yum/apt (union of fields)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RepositoryRecord {
    pub alias:       String,
    pub name:        String,
    pub url:         String,
    pub r#type:      String,
    pub enabled:     bool,
    pub gpgcheck:    bool,
    pub autorefresh: bool,
    pub priority:    i32,
    // apt-specific
    pub distribution: String,
    pub components:   Vec<String>,
}

// ---------------------------------------------------------------------------
// ServiceRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ServiceRecord {
    pub name:        String,
    pub state:       String,
    pub legacy_sysv: bool,
}

// ---------------------------------------------------------------------------
// GroupRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GroupRecord {
    pub name:     String,
    pub password: String,
    pub gid:      Option<i64>,
    pub users:    Vec<String>,
}

// ---------------------------------------------------------------------------
// UserRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UserRecord {
    pub name:               String,
    pub password:           String,
    pub uid:                Option<i64>,
    pub gid:                Option<i64>,
    pub comment:            String,
    pub home:               String,
    pub shell:              String,
    pub encrypted_password: String,
    pub last_changed_date:  i64,
    pub min_days:           i64,
    pub max_days:           i64,
    pub warn_days:          i64,
    pub disable_days:       i64,
    pub disabled_date:      i64,
}

// ---------------------------------------------------------------------------
// ChangedConfigFileRecord / ChangedManagedFileRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ChangedConfigFileRecord {
    pub name:            String,
    pub package_name:    String,
    pub package_version: String,
    pub status:          String,
    pub changes:         Vec<String>,
    pub mode:            String,
    pub user:            String,
    pub group:           String,
    pub r#type:          String,
    pub target:          String,
    pub error_message:   String,
}

pub type ChangedManagedFileRecord = ChangedConfigFileRecord;

// ---------------------------------------------------------------------------
// UnmanagedFileRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UnmanagedFileRecord {
    pub name:         String,
    pub r#type:       String,
    pub user:         String,
    pub group:        String,
    pub size:         i64,
    pub mode:         String,
    pub files:        i64,
    pub dirs:         i64,
    pub file_objects: i64,
}

// ---------------------------------------------------------------------------
// CpuRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CpuRecord {
    pub processor:  String,
    pub vendor_id:  String,
    pub model_name: String,
    pub cpu_family: String,
    pub model:      String,
    pub stepping:   String,
    pub cpu_mhz:    String,
    pub cache_size: String,
}

// ---------------------------------------------------------------------------
// KernelParamRecord / NetParamRecord / KernelConfigRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KernelParamRecord {
    pub key:   String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NetParamRecord {
    pub key:   String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct KernelConfigRecord {
    pub key:   String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// DeviceRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DeviceRecord {
    pub name:  String,
    pub dma:   String,
    pub irq:   String,
    pub ports: String,
}

// ---------------------------------------------------------------------------
// PciRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PciRecord {
    pub pci:     String,
    pub device:  String,
    pub class:   String,
    pub vendor:  String,
    pub svendor: String,
    pub sdevice: String,
    pub rev:     String,
}

// ---------------------------------------------------------------------------
// Storage types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PartitionRecord {
    pub device:          String,
    pub maj_min:         String,
    pub r#type:          String,
    pub size:            String,
    pub fstype:          String,
    pub mountpoint:      String,
    pub uuid:            String,
    pub label:           String,
    pub ro:              String,
    pub partition_type:  String,
    pub type_id:         String,
    pub begin_sector:    String,
    pub end_sector:      String,
    pub raw_size_kb:     i64,
    pub mount_options:   String,
    pub reserved_blocks: String,
    pub block_size:      String,
    pub inode_density:   String,
    pub max_mount_count: String,
    pub df_blocks_kb:    i64,
    pub df_used_kb:      i64,
    pub df_avail_kb:     i64,
    pub df_use_percent:  String,
    pub source:          String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SoftwareRaidRecord {
    pub device:     String,
    pub level:      String,
    pub partitions: Vec<String>,
    pub blocks:     String,
    pub chunk_size: String,
    pub algorithm:  String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IdeRecord {
    pub device:           String,
    pub media:            String,
    pub model:            String,
    pub driver:           String,
    pub geometry_phys:    String,
    pub geometry_log:     String,
    pub capacity_blocks:  String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ScsiRecord {
    pub host:     String,
    pub channel:  String,
    pub id:       String,
    pub lun:      String,
    pub vendor:   String,
    pub model:    String,
    pub revision: String,
    pub r#type:   String,
    pub ansi_rev: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BtrfsDeviceRecord {
    pub devid: String,
    pub size:  String,
    pub used:  String,
    pub path:  String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BtrfsSubvolumeRecord {
    pub id:        String,
    pub gen:       String,
    pub top_level: String,
    pub path:      String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct BtrfsFilesystemRecord {
    pub label:          String,
    pub uuid:           String,
    pub total_devices:  String,
    pub bytes_used:     String,
    pub mount_point:    String,
    pub devices:        Vec<BtrfsDeviceRecord>,
    pub subvolumes:     Vec<BtrfsSubvolumeRecord>,
    pub data_total:     String,
    pub data_used:      String,
    pub metadata_total: String,
    pub metadata_used:  String,
    pub system_total:   String,
    pub system_used:    String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RawControllerRecord {
    pub controller_id: String,
    pub raw_output:    String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RawTextRecord {
    pub path:    String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StorageScope {
    pub partitions:    ScopeWrapper<PartitionRecord>,
    pub software_raid: ScopeWrapper<SoftwareRaidRecord>,
    pub btrfs:         ScopeWrapper<BtrfsFilesystemRecord>,
    pub ide:           ScopeWrapper<IdeRecord>,
    pub scsi:          ScopeWrapper<ScsiRecord>,
    pub cciss:         ScopeWrapper<RawControllerRecord>,
    pub areca:         ScopeWrapper<RawControllerRecord>,
    pub dac960:        ScopeWrapper<RawControllerRecord>,
    pub gdth:          ScopeWrapper<RawControllerRecord>,
    pub ips:           ScopeWrapper<RawControllerRecord>,
    pub compaq_smart:  ScopeWrapper<RawControllerRecord>,
    pub evms:          Option<RawTextRecord>,
    pub multipath:     Option<RawTextRecord>,
    pub fstab:         Option<RawTextRecord>,
    pub lvm_conf:      Option<RawTextRecord>,
}

// ---------------------------------------------------------------------------
// Network types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NetworkInterfaceRecord {
    pub ifname:        String,
    pub link_type:     String,
    pub address:       String,
    pub flags:         Vec<String>,
    pub mtu:           i64,
    pub operstate:     String,
    pub ip:            String,
    pub prefixlen:     String,
    pub broadcast:     String,
    pub ip6:           String,
    pub ip6_prefixlen: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RouteRecord {
    pub dst:      String,
    pub gateway:  String,
    pub dev:      String,
    pub protocol: String,
    pub scope:    String,
    pub r#type:   String,
    pub metric:   String,
    pub flags:    Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PacketFilterRecord {
    pub engine:     String,
    pub table:      String,
    pub raw_output: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NetworkScope {
    pub interfaces:    ScopeWrapper<NetworkInterfaceRecord>,
    pub routes:        ScopeWrapper<RouteRecord>,
    pub packet_filter: ScopeWrapper<PacketFilterRecord>,
}

// ---------------------------------------------------------------------------
// AppArmor types
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ApparmorProfileRecord {
    pub name: String,
    pub mode: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ApparmorKernelRecord {
    pub key:   String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SecurityApparmorScope {
    pub kernel_params: ScopeWrapper<ApparmorKernelRecord>,
    pub profiles:      ScopeWrapper<ApparmorProfileRecord>,
    pub config_files:  Vec<String>,
}

// ---------------------------------------------------------------------------
// ProcessRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ProcessRecord {
    pub pid:     String,
    pub ppid:    String,
    pub comm:    String,
    pub state:   String,
    pub cmdline: String,
}

// ---------------------------------------------------------------------------
// DmiScope
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DmiScope {
    pub raw_output: String,
}

// ---------------------------------------------------------------------------
// GeneralInfoRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GeneralInfoRecord {
    pub key:   String,
    pub value: String,
}

// ---------------------------------------------------------------------------
// CrontabRecord
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CrontabRecord {
    pub minute:       String,
    pub hour:         String,
    pub day_of_month: String,
    pub month:        String,
    pub day_of_week:  String,
    pub user:         String,
    pub command:      String,
    pub source:       String,
}

// ---------------------------------------------------------------------------
// FileInfo — used by Filesystem interface
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct FileInfo {
    pub uid:   u32,
    pub gid:   u32,
    pub mode:  String,
    pub size:  u64,
    pub mtime: String,
}

// ---------------------------------------------------------------------------
// SitarManifest — the complete intermediate representation
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SitarManifest {
    pub meta:                  SitarMeta,
    pub general_info:          ScopeWrapper<GeneralInfoRecord>,
    pub environment:           EnvironmentScope,
    pub os:                    OsScope,
    pub packages:              ScopeWrapper<PackageRecord>,
    pub patterns:              ScopeWrapper<PatternRecord>,
    pub repositories:          ScopeWrapper<RepositoryRecord>,
    pub services:              ScopeWrapper<ServiceRecord>,
    pub groups:                ScopeWrapper<GroupRecord>,
    pub users:                 ScopeWrapper<UserRecord>,
    pub changed_config_files:  ScopeWrapper<ChangedConfigFileRecord>,
    pub changed_managed_files: ScopeWrapper<ChangedManagedFileRecord>,
    pub unmanaged_files:       ScopeWrapper<UnmanagedFileRecord>,
    pub cpu:                   ScopeWrapper<CpuRecord>,
    pub kernel_params:         ScopeWrapper<KernelParamRecord>,
    pub net_params:            ScopeWrapper<NetParamRecord>,
    pub kernel_config:         ScopeWrapper<KernelConfigRecord>,
    pub devices:               ScopeWrapper<DeviceRecord>,
    pub pci:                   ScopeWrapper<PciRecord>,
    pub storage:               StorageScope,
    pub network:               NetworkScope,
    pub security_apparmor:     Option<SecurityApparmorScope>,
    pub processes:             ScopeWrapper<ProcessRecord>,
    pub dmi:                   Option<DmiScope>,
}
