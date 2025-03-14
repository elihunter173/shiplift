//! Rust representations of docker json structures

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub description: String,
    pub is_official: bool,
    pub is_automated: bool,
    pub name: String,
    pub star_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Image {
    #[cfg(feature = "chrono")]
    #[serde(deserialize_with = "datetime_from_unix_timestamp")]
    pub created: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created: u64,
    pub id: String,
    pub parent_id: String,
    pub labels: Option<HashMap<String, String>>,
    pub repo_tags: Option<Vec<String>>,
    pub repo_digests: Option<Vec<String>>,
    pub virtual_size: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ImageDetails {
    pub id: String,
    pub repo_tags: Option<Vec<String>>,
    pub repo_digests: Option<Vec<String>>,
    pub parent: String,
    pub comment: String,
    #[cfg(feature = "chrono")]
    pub created: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created: String,
    pub container: String,
    pub container_config: Option<ContainerConfig>,
    pub docker_version: String,
    pub author: String,
    pub config: Option<ContainerConfig>,
    pub architecture: String,
    pub os: String,
    pub os_version: Option<String>,
    pub size: i64,
    pub virtual_size: i64,
    pub graph_driver: GraphDriverData,
    #[serde(rename = "RootFS")]
    pub root_fs: RootFS,
    pub metadata: Metadata,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Container {
    #[cfg(feature = "chrono")]
    #[serde(deserialize_with = "datetime_from_unix_timestamp")]
    pub created: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created: u64,
    pub command: String,
    pub id: String,
    pub image: String,
    #[serde(rename = "ImageID")]
    pub image_id: String,
    pub labels: HashMap<String, String>,
    pub names: Vec<String>,
    pub ports: Vec<Port>,
    pub state: String,
    pub status: String,
    pub size_rw: Option<u64>,
    pub size_root_fs: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerDetails {
    pub app_armor_profile: String,
    pub args: Vec<String>,
    pub config: ContainerConfig,
    #[cfg(feature = "chrono")]
    pub created: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created: String,
    pub driver: String,
    // pub ExecIDs: ??
    pub host_config: HostConfig,
    pub hostname_path: String,
    pub hosts_path: String,
    pub log_path: String,
    pub id: String,
    pub image: String,
    pub mount_label: String,
    pub name: String,
    pub network_settings: NetworkSettings,
    pub path: String,
    pub process_label: String,
    pub resolv_conf_path: String,
    pub restart_count: u64,
    pub state: State,
    pub mounts: Vec<Mount>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mount {
    pub source: String,
    pub destination: String,
    pub mode: String,
    #[serde(rename = "RW")]
    pub rw: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct State {
    pub error: String,
    pub exit_code: u64,
    #[cfg(feature = "chrono")]
    pub finished_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub finished_at: String,
    #[serde(rename = "OOMKilled")]
    pub oom_killed: bool,
    pub paused: bool,
    pub pid: u64,
    pub restarting: bool,
    pub running: bool,
    #[cfg(feature = "chrono")]
    pub started_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub started_at: String,
    pub status: String,
}

type PortDescription = HashMap<String, Option<Vec<HashMap<String, String>>>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkSettings {
    pub bridge: String,
    pub gateway: String,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
    #[serde(rename = "IPPrefixLen")]
    pub ip_prefix_len: u64,
    pub mac_address: String,
    pub ports: Option<PortDescription>,
    pub networks: HashMap<String, NetworkEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkEntry {
    #[serde(rename = "NetworkID")]
    pub network_id: String,
    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
    pub gateway: String,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
    #[serde(rename = "IPPrefixLen")]
    pub ip_prefix_len: u64,
    #[serde(rename = "IPv6Gateway")]
    pub ipv6_gateway: String,
    #[serde(rename = "GlobalIPv6Address")]
    pub global_ipv6_address: String,
    #[serde(rename = "GlobalIPv6PrefixLen")]
    pub global_ipv6_prefix_len: u64,
    pub mac_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HostConfig {
    pub cgroup_parent: Option<String>,
    #[serde(rename = "ContainerIDFile")]
    pub container_id_file: String,
    pub cpu_shares: Option<u64>,
    pub cpuset_cpus: Option<String>,
    pub memory: Option<u64>,
    pub memory_swap: Option<i64>,
    pub network_mode: String,
    pub pid_mode: Option<String>,
    pub port_bindings: Option<HashMap<String, Vec<HashMap<String, String>>>>,
    pub privileged: bool,
    pub publish_all_ports: bool,
    pub readonly_rootfs: Option<bool>, /* pub RestartPolicy: ???
                                        * pub SecurityOpt: Option<???>,
                                        * pub Ulimits: Option<???>
                                        * pub VolumesFrom: Option<??/> */
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    pub hostname: String,
    pub domainname: String,
    pub user: String,
    pub attach_stdin: bool,
    pub attach_stdout: bool,
    pub attach_stderr: bool,
    pub exposed_ports: Option<HashMap<String, HashMap<String, String>>>,
    pub tty: bool,
    pub open_stdin: bool,
    pub stdin_once: bool,
    pub env: Option<Vec<String>>,
    pub cmd: Option<Vec<String>>,
    pub healtcheck: Option<HealthConfig>,
    pub args_escaped: Option<bool>,
    pub image: String,
    pub volumes: Option<HashMap<String, HashMap<String, String>>>,
    pub working_dir: String,
    pub entrypoint: Option<Vec<String>>,
    pub network_disabled: Option<bool>,
    pub mac_address: Option<String>,
    pub on_build: Option<Vec<String>>,
    pub labels: Option<HashMap<String, String>>,
    pub stop_signal: Option<String>,
    pub stop_timeout: Option<u64>,
    pub shell: Option<Vec<String>>,
}

impl ContainerConfig {
    pub fn env(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(ref vars) = self.env {
            for e in vars {
                let pair: Vec<&str> = e.split('=').collect();
                map.insert(pair[0].to_owned(), pair[1].to_owned());
            }
        }
        map
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HealthConfig {
    pub test: Option<Vec<String>>,
    pub interval: Option<u64>,
    pub timeout: Option<u64>,
    pub retries: Option<u64>,
    pub start_period: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GraphDriverData {
    pub name: String,
    pub data: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RootFS {
    #[serde(rename = "Type")]
    pub typ: String,
    pub layers: Option<Vec<String>>,
    pub base_layer: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Metadata {
    #[cfg(feature = "chrono")]
    pub last_tag_time: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub last_tag_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Port {
    pub ip: Option<String>,
    pub private_port: u64,
    pub public_port: Option<u64>,
    #[serde(rename = "Type")]
    pub typ: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub read: String,
    pub networks: HashMap<String, Network>,
    pub memory_stats: MemoryStats,
    pub blkio_stats: BlkioStats,
    pub cpu_stats: CpuStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Network {
    pub rx_dropped: u64,
    pub rx_bytes: u64,
    pub rx_errors: u64,
    pub tx_packets: u64,
    pub tx_dropped: u64,
    pub rx_packets: u64,
    pub tx_errors: u64,
    pub tx_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IPAM {
    pub driver: String,
    pub config: Vec<HashMap<String, String>>,
    pub options: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkDetails {
    pub name: String,
    pub id: String,
    pub scope: String,
    pub driver: String,
    #[serde(rename = "EnableIPv6")]
    pub enable_ipv6: bool,
    #[serde(rename = "IPAM")]
    pub ipam: IPAM,
    pub internal: bool,
    pub attachable: bool,
    pub containers: HashMap<String, NetworkContainerDetails>,
    pub options: Option<HashMap<String, String>>,
    pub labels: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkContainerDetails {
    #[serde(rename = "EndpointID")]
    pub endpoint_id: String,
    pub mac_address: String,
    #[serde(rename = "IPv4Address")]
    pub ipv4_address: String,
    #[serde(rename = "IPv6Address")]
    pub ipv6_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkCreateInfo {
    pub id: String,
    pub warning: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryStats {
    pub max_usage: u64,
    pub usage: u64,
    pub failcnt: Option<u64>,
    pub limit: u64,
    pub stats: MemoryStat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryStat {
    pub total_pgmajfault: u64,
    pub cache: u64,
    pub mapped_file: u64,
    pub total_inactive_file: u64,
    pub pgpgout: u64,
    pub rss: u64,
    pub total_mapped_file: u64,
    pub writeback: u64,
    pub unevictable: u64,
    pub pgpgin: u64,
    pub total_unevictable: u64,
    pub pgmajfault: u64,
    pub total_rss: u64,
    pub total_rss_huge: u64,
    pub total_writeback: u64,
    pub total_inactive_anon: u64,
    pub rss_huge: u64,
    pub hierarchical_memory_limit: u64,
    pub hierarchical_memsw_limit: u64,
    pub total_pgfault: u64,
    pub total_active_file: u64,
    pub active_anon: u64,
    pub total_active_anon: u64,
    pub total_pgpgout: u64,
    pub total_cache: u64,
    pub inactive_anon: u64,
    pub active_file: u64,
    pub pgfault: u64,
    pub inactive_file: u64,
    pub total_pgpgin: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuStats {
    pub cpu_usage: CpuUsage,
    pub system_cpu_usage: u64,
    pub throttling_data: ThrottlingData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuUsage {
    pub percpu_usage: Vec<u64>,
    pub usage_in_usermode: u64,
    pub total_usage: u64,
    pub usage_in_kernelmode: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThrottlingData {
    pub periods: u64,
    pub throttled_periods: u64,
    pub throttled_time: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlkioStats {
    pub io_service_bytes_recursive: Vec<BlkioStat>,
    pub io_serviced_recursive: Vec<BlkioStat>,
    pub io_queue_recursive: Vec<BlkioStat>,
    pub io_service_time_recursive: Vec<BlkioStat>,
    pub io_wait_time_recursive: Vec<BlkioStat>,
    pub io_merged_recursive: Vec<BlkioStat>,
    pub io_time_recursive: Vec<BlkioStat>,
    pub sectors_recursive: Vec<BlkioStat>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlkioStat {
    pub major: u64,
    pub minor: u64,
    pub op: String,
    pub value: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Change {
    pub kind: u64,
    pub path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Top {
    pub titles: Vec<String>,
    pub processes: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Version {
    pub version: String,
    pub api_version: String,
    pub git_commit: String,
    pub go_version: String,
    pub os: String,
    pub arch: String,
    pub kernel_version: String,
    #[cfg(feature = "chrono")]
    pub build_time: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub build_time: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Info {
    pub containers: u64,
    pub images: u64,
    pub driver: String,
    pub docker_root_dir: String,
    pub driver_status: Vec<Vec<String>>,
    #[serde(rename = "ID")]
    pub id: String,
    pub kernel_version: String,
    // pub Labels: Option<???>,
    pub mem_total: u64,
    pub memory_limit: bool,
    #[serde(rename = "NCPU")]
    pub n_cpu: u64,
    pub n_events_listener: u64,
    pub n_goroutines: u64,
    pub name: String,
    pub operating_system: String,
    // pub RegistryConfig:???
    pub swap_limit: bool,
    pub system_time: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerCreateInfo {
    pub id: String,
    pub warnings: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct History {
    pub id: String,
    #[cfg(feature = "chrono")]
    #[serde(deserialize_with = "datetime_from_unix_timestamp")]
    pub created: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created: i64,
    pub created_by: String,
    pub tags: Option<Vec<String>>,
    pub size: i64,
    pub comment: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Exit {
    pub status_code: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "Type")]
    pub typ: String,
    #[serde(rename = "Action")]
    pub action: String,
    #[serde(rename = "Actor")]
    pub actor: Actor,
    pub status: Option<String>,
    pub id: Option<String>,
    pub from: Option<String>,
    #[cfg(feature = "chrono")]
    #[serde(deserialize_with = "datetime_from_unix_timestamp")]
    pub time: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub time: u64,
    #[cfg(feature = "chrono")]
    #[serde(deserialize_with = "datetime_from_nano_timestamp", rename = "timeNano")]
    pub time_nano: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    #[serde(rename = "timeNano")]
    pub time_nano: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExecDetails {
    pub can_remove: bool,
    #[serde(rename = "ContainerID")]
    pub container_id: String,
    pub detach_keys: String,
    pub exit_code: Option<u64>,
    #[serde(rename = "ID")]
    pub id: String,
    pub open_stderr: bool,
    pub open_stdin: bool,
    pub open_stdout: bool,
    pub process_config: ProcessConfig,
    pub running: bool,
    pub pid: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessConfig {
    pub arguments: Vec<String>,
    pub entrypoint: String,
    pub privileged: bool,
    pub tty: bool,
    pub user: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Actor {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Attributes")]
    pub attributes: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Status {
    Untagged(String),
    Deleted(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VolumeCreateInfo {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Volumes {
    pub volumes: Option<Vec<Volume>>,
    pub warnings: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Volume {
    #[cfg(feature = "chrono")]
    pub created_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created_at: String,
    pub driver: String,
    pub labels: Option<HashMap<String, String>>,
    pub name: String,
    pub mountpoint: String,
    pub options: Option<HashMap<String, String>>,
    pub scope: String,
}

//################################################################################
// SERVICES
//################################################################################

pub type Services = Vec<Service>;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Service {
    #[serde(rename = "ID")]
    pub id: String,
    pub version: ObjectVersion,
    #[cfg(feature = "chrono")]
    pub created_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created_at: String,
    #[cfg(feature = "chrono")]
    pub updated_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub updated_at: String,
    pub endpoint: Endpoint,
    pub update_status: Option<UpdateStatus>,
    pub service_status: Option<ServiceStatus>,
    pub job_status: Option<JobStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ObjectVersion {
    pub index: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Endpoint {
    pub spec: EndpointSpec,
    pub ports: Option<Vec<EndpointPortConfig>>,
    #[serde(rename = "VirtualIPs")]
    pub virtual_ips: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EndpointSpec {
    pub mode: Option<String>,
    pub ports: Option<Vec<EndpointPortConfig>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EndpointPortConfig {
    pub name: Option<String>,
    pub protocol: String,
    pub publish_mode: String,
    pub published_port: Option<u64>,
    pub target_port: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdateStatus {
    pub state: String,
    #[cfg(feature = "chrono")]
    pub started_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub started_at: String,
    #[cfg(feature = "chrono")]
    pub completed_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub completed_at: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceStatus {
    pub running_tasks: u64,
    pub desired_tasks: u64,
    pub completed_tasks: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct JobStatus {
    pub job_iteration: ObjectVersion,
    #[cfg(feature = "chrono")]
    pub last_execution: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub last_execution: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceDetails {
    #[serde(rename = "ID")]
    pub id: String,
    pub version: ObjectVersion,
    #[cfg(feature = "chrono")]
    pub created_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub created_at: String,
    #[cfg(feature = "chrono")]
    pub updated_at: DateTime<Utc>,
    #[cfg(not(feature = "chrono"))]
    pub updated_at: String,
    pub spec: ServiceSpec,
    pub endpoint: Endpoint,
    pub update_status: Option<UpdateStatus>,
    pub service_status: Option<ServiceStatus>,
    pub job_status: Option<JobStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceSpec {
    pub name: String,
    pub labels: Option<serde_json::Value>,
    pub task_template: TaskSpec,
    pub mode: Mode,
    pub update_config: Option<UpdateConfig>,
    pub rollback_config: Option<RollbackConfig>,
    pub networks: Option<Vec<NetworkAttachmentConfig>>,
    pub endpoint_spec: EndpointSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
// #TODO: Add missing fields...
pub struct TaskSpec {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mode {
    pub replicated: Option<Replicated>,
    pub global: Option<serde_json::Value>,
    pub replicated_job: Option<ReplicatedJob>,
    pub global_job: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Replicated {
    pub replicas: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReplicatedJob {
    pub max_concurrent: u64,
    pub total_completions: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdateConfig {
    pub parallelism: u64,
    pub delay: u64,
    pub failure_action: String,
    pub monitor: u64,
    pub max_failure_ratio: usize,
    pub order: String,
}

pub type RollbackConfig = UpdateConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkAttachmentConfig {
    pub target: String,
    pub aliases: Vec<String>,
    pub driver_opts: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceCreateInfo {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Warning")]
    pub warning: Option<String>,
}

//################################################################################

#[cfg(feature = "chrono")]
fn datetime_from_unix_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let timestamp = chrono::NaiveDateTime::from_timestamp(i64::deserialize(deserializer)?, 0);
    Ok(DateTime::<Utc>::from_utc(timestamp, Utc))
}

#[cfg(feature = "chrono")]
fn datetime_from_nano_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let timestamp_nano = u64::deserialize(deserializer)?;
    let timestamp = chrono::NaiveDateTime::from_timestamp(
        (timestamp_nano / 1_000_000_000) as i64,
        (timestamp_nano % 1_000_000_000) as u32,
    );
    Ok(DateTime::<Utc>::from_utc(timestamp, Utc))
}
