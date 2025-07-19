//! Bounded Infrastructure for WRTD Daemon
//!
//! This module provides bounded alternatives for daemon collections
//! to ensure static memory allocation throughout the daemon operations.


use wrt_foundation::{
    bounded::{BoundedVec, BoundedString},
    bounded_collections::BoundedMap as BoundedHashMap,
    safe_memory::NoStdProvider,
    capabilities::CapabilityAwareProvider,
    capability_context, safe_capability_alloc, CrateId,
    traits::{Checksummable, ToBytes, FromBytes},
    WrtResult,
};

/// Budget-aware memory provider for WRTD daemon (64KB)
pub type WrtdProvider = CapabilityAwareProvider<NoStdProvider<32768>>;

/// Helper function to create a capability-aware provider for WRTD
fn create_wrtd_provider() -> WrtResult<WrtdProvider> {
    let context = capability_context!(dynamic(CrateId::Platform, 32768))?;
    safe_capability_alloc!(context, CrateId::Platform, 32768)
}

/// Maximum number of daemon services
pub const MAX_DAEMON_SERVICES: usize = 32;

/// Maximum number of active connections
pub const MAX_ACTIVE_CONNECTIONS: usize = 128;

/// Maximum number of service configurations
pub const MAX_SERVICE_CONFIGS: usize = 64;

/// Maximum number of runtime processes
pub const MAX_RUNTIME_PROCESSES: usize = 64;

/// Maximum number of log entries
pub const MAX_LOG_ENTRIES: usize = 1024;

/// Maximum number of metrics entries
pub const MAX_METRICS_ENTRIES: usize = 512;

/// Maximum number of health checks
pub const MAX_HEALTH_CHECKS: usize = 128;

/// Maximum service name length
pub const MAX_SERVICE_NAME_LEN: usize = 128;

/// Maximum configuration key length
pub const MAX_CONFIG_KEY_LEN: usize = 128;

/// Maximum configuration value length
pub const MAX_CONFIG_VALUE_LEN: usize = 512;

/// Maximum log message length
pub const MAX_LOG_MESSAGE_LEN: usize = 1024;

/// Maximum process command length
pub const MAX_PROCESS_COMMAND_LEN: usize = 512;

/// Maximum connection ID length
pub const MAX_CONNECTION_ID_LEN: usize = 64;

/// Maximum number of environment variables
pub const MAX_ENV_VARS: usize = 128;

/// Maximum environment variable name length
pub const MAX_ENV_VAR_NAME_LEN: usize = 128;

/// Maximum environment variable value length
pub const MAX_ENV_VAR_VALUE_LEN: usize = 512;

/// Bounded vector for daemon services
pub type BoundedDaemonServiceVec<T> = BoundedVec<T, MAX_DAEMON_SERVICES, WrtdProvider>;

/// Bounded vector for active connections
pub type BoundedConnectionVec<T> = BoundedVec<T, MAX_ACTIVE_CONNECTIONS, WrtdProvider>;

/// Bounded vector for service configurations
pub type BoundedServiceConfigVec<T> = BoundedVec<T, MAX_SERVICE_CONFIGS, WrtdProvider>;

/// Bounded vector for runtime processes
pub type BoundedProcessVec<T> = BoundedVec<T, MAX_RUNTIME_PROCESSES, WrtdProvider>;

/// Bounded vector for log entries
pub type BoundedLogEntryVec<T> = BoundedVec<T, MAX_LOG_ENTRIES, WrtdProvider>;

/// Bounded vector for metrics entries
pub type BoundedMetricsVec<T> = BoundedVec<T, MAX_METRICS_ENTRIES, WrtdProvider>;

/// Bounded vector for health checks
pub type BoundedHealthCheckVec<T> = BoundedVec<T, MAX_HEALTH_CHECKS, WrtdProvider>;

/// Bounded vector for environment variables
pub type BoundedEnvVarVec<T> = BoundedVec<T, MAX_ENV_VARS, WrtdProvider>;

/// Bounded string for service names
pub type BoundedServiceName = BoundedString<MAX_SERVICE_NAME_LEN, WrtdProvider>;

/// Bounded string for configuration keys
pub type BoundedConfigKey = BoundedString<MAX_CONFIG_KEY_LEN, WrtdProvider>;

/// Bounded string for configuration values
pub type BoundedConfigValue = BoundedString<MAX_CONFIG_VALUE_LEN, WrtdProvider>;

/// Bounded string for log messages
pub type BoundedLogMessage = BoundedString<MAX_LOG_MESSAGE_LEN, WrtdProvider>;

/// Bounded string for process commands
pub type BoundedProcessCommand = BoundedString<MAX_PROCESS_COMMAND_LEN, WrtdProvider>;

/// Bounded string for connection IDs
pub type BoundedConnectionId = BoundedString<MAX_CONNECTION_ID_LEN, WrtdProvider>;

/// Bounded string for environment variable names
pub type BoundedEnvVarName = BoundedString<MAX_ENV_VAR_NAME_LEN, WrtdProvider>;

/// Bounded string for environment variable values
pub type BoundedEnvVarValue = BoundedString<MAX_ENV_VAR_VALUE_LEN, WrtdProvider>;

/// Bounded map for daemon services
pub type BoundedServiceMap<V> = BoundedHashMap<
    BoundedServiceName,
    V,
    MAX_DAEMON_SERVICES,
    WrtdProvider
>;

/// Bounded map for active connections
pub type BoundedConnectionMap<V> = BoundedHashMap<
    BoundedConnectionId,
    V,
    MAX_ACTIVE_CONNECTIONS,
    WrtdProvider
>;

/// Bounded map for service configurations
pub type BoundedConfigMap = BoundedHashMap<
    BoundedConfigKey,
    BoundedConfigValue,
    MAX_SERVICE_CONFIGS,
    WrtdProvider
>;

/// Bounded map for runtime processes
pub type BoundedProcessMap<V> = BoundedHashMap<
    u32, // Process ID
    V,
    MAX_RUNTIME_PROCESSES,
    WrtdProvider
>;

/// Bounded map for environment variables
pub type BoundedEnvMap = BoundedHashMap<
    BoundedEnvVarName,
    BoundedEnvVarValue,
    MAX_ENV_VARS,
    WrtdProvider
>;

/// Create a new bounded daemon service vector
pub fn new_daemon_service_vec<T>() -> WrtResult<BoundedDaemonServiceVec<T>> 
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded connection vector
pub fn new_connection_vec<T>() -> WrtResult<BoundedConnectionVec<T>> 
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded service config vector
pub fn new_service_config_vec<T>() -> WrtResult<BoundedServiceConfigVec<T>> 
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded process vector
pub fn new_process_vec<T>() -> WrtResult<BoundedProcessVec<T>> 
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded log entry vector
pub fn new_log_entry_vec<T>() -> WrtResult<BoundedLogEntryVec<T>> 
where
    T: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedVec::new(provider)
}

/// Create a new bounded service name
pub fn new_service_name() -> WrtResult<BoundedServiceName> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str("", provider)
        .map_err(|_| wrt_error::Error::runtime_execution_error("
        ))
}

/// Create a bounded service name from str
pub fn bounded_service_name_from_str(s: &str) -> WrtResult<BoundedServiceName> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str(s, provider)
        .map_err(|_| wrt_error::Error::new(wrt_error::ErrorCategory::Resource,
            wrt_foundation::codes::ALLOCATION_FAILED,
            "))
}

/// Create a new bounded configuration key
pub fn new_config_key() -> WrtResult<BoundedConfigKey> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str("", provider)
        .map_err(|_| wrt_error::Error::runtime_execution_error("
        ))
}

/// Create a bounded configuration key from str
pub fn bounded_config_key_from_str(s: &str) -> WrtResult<BoundedConfigKey> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str(s, provider)
        .map_err(|_| wrt_error::Error::new(wrt_error::ErrorCategory::Resource,
            wrt_foundation::codes::ALLOCATION_FAILED,
            "))
}

/// Create a new bounded configuration value
pub fn new_config_value() -> WrtResult<BoundedConfigValue> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str("", provider)
        .map_err(|_| wrt_error::Error::runtime_execution_error("
        ))
}

/// Create a bounded configuration value from str
pub fn bounded_config_value_from_str(s: &str) -> WrtResult<BoundedConfigValue> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str(s, provider)
        .map_err(|_| wrt_error::Error::new(wrt_error::ErrorCategory::Resource,
            wrt_foundation::codes::ALLOCATION_FAILED,
            "))
}

/// Create a new bounded log message
pub fn new_log_message() -> WrtResult<BoundedLogMessage> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str("", provider)
        .map_err(|_| wrt_error::Error::runtime_execution_error("
        ))
}

/// Create a bounded log message from str
pub fn bounded_log_message_from_str(s: &str) -> WrtResult<BoundedLogMessage> {
    let provider = create_wrtd_provider()?;
    BoundedString::from_str(s, provider)
        .map_err(|_| wrt_error::Error::new(wrt_error::ErrorCategory::Resource,
            wrt_foundation::codes::ALLOCATION_FAILED,
            "))
}

/// Create a new bounded service map
pub fn new_service_map<V>() -> WrtResult<BoundedServiceMap<V>> 
where
    V: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedHashMap::new(provider)
}

/// Create a new bounded connection map
pub fn new_connection_map<V>() -> WrtResult<BoundedConnectionMap<V>>
where
    V: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedHashMap::new(provider)
}

/// Create a new bounded configuration map
pub fn new_config_map() -> WrtResult<BoundedConfigMap> {
    let provider = create_wrtd_provider()?;
    BoundedHashMap::new(provider)
}

/// Create a new bounded process map
pub fn new_process_map<V>() -> WrtResult<BoundedProcessMap<V>>
where
    V: Sized + Checksummable + ToBytes + FromBytes + Default + Clone + PartialEq + Eq,
{
    let provider = create_wrtd_provider()?;
    BoundedHashMap::new(provider)
}

/// Create a new bounded environment map
pub fn new_env_map() -> WrtResult<BoundedEnvMap> {
    let provider = create_wrtd_provider()?;
    BoundedHashMap::new(provider)
}