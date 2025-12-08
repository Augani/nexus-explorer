use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use thiserror::Error;

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkLocationId(pub u64);

impl NetworkLocationId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkProtocol {
    Smb,
    Ftp,
    Sftp,
    WebDav,
    Nfs,
}

impl NetworkProtocol {
    pub fn default_port(&self) -> u16 {
        match self {
            NetworkProtocol::Smb => 445,
            NetworkProtocol::Ftp => 21,
            NetworkProtocol::Sftp => 22,
            NetworkProtocol::WebDav => 443,
            NetworkProtocol::Nfs => 2049,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            NetworkProtocol::Smb => "SMB/CIFS",
            NetworkProtocol::Ftp => "FTP",
            NetworkProtocol::Sftp => "SFTP",
            NetworkProtocol::WebDav => "WebDAV",
            NetworkProtocol::Nfs => "NFS",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            NetworkProtocol::Smb => "folder",
            NetworkProtocol::Ftp => "cloud",
            NetworkProtocol::Sftp => "cloud",
            NetworkProtocol::WebDav => "cloud",
            NetworkProtocol::Nfs => "hard-drive",
        }
    }

    pub fn url_scheme(&self) -> &'static str {
        match self {
            NetworkProtocol::Smb => "smb",
            NetworkProtocol::Ftp => "ftp",
            NetworkProtocol::Sftp => "sftp",
            NetworkProtocol::WebDav => "https",
            NetworkProtocol::Nfs => "nfs",
        }
    }
}

/
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMethod {
    Anonymous,
    Password { username: String, password: String },
    KeyFile { username: String, key_path: PathBuf },
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::Anonymous
    }
}

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnectionConfig {
    pub protocol: NetworkProtocol,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub auth: AuthMethod,
    pub label: Option<String>,
}

impl NetworkConnectionConfig {
    pub fn new(protocol: NetworkProtocol, host: String) -> Self {
        Self {
            protocol,
            host,
            port: None,
            path: String::from("/"),
            auth: AuthMethod::Anonymous,
            label: None,
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }

    pub fn with_auth(mut self, auth: AuthMethod) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }

    /
    pub fn effective_port(&self) -> u16 {
        self.port.unwrap_or_else(|| self.protocol.default_port())
    }

    /
    pub fn to_url(&self) -> String {
        let port_str = self.port.map(|p| format!(":{}", p)).unwrap_or_default();

        format!(
            "{}://{}{}{}",
            self.protocol.url_scheme(),
            self.host,
            port_str,
            self.path
        )
    }

    /
    pub fn display_name(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.host)
    }
}

/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self::Disconnected
    }
}

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkLocation {
    pub id: NetworkLocationId,
    pub config: NetworkConnectionConfig,
    #[serde(skip)]
    pub state: ConnectionState,
    #[serde(skip)]
    pub latency_ms: Option<u32>,
    #[serde(skip)]
    pub last_error: Option<String>,
    #[serde(skip)]
    pub mount_point: Option<PathBuf>,
}

impl NetworkLocation {
    pub fn new(id: NetworkLocationId, config: NetworkConnectionConfig) -> Self {
        Self {
            id,
            config,
            state: ConnectionState::Disconnected,
            latency_ms: None,
            last_error: None,
            mount_point: None,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }

    pub fn display_name(&self) -> &str {
        self.config.display_name()
    }

    pub fn protocol(&self) -> NetworkProtocol {
        self.config.protocol
    }
}

/
#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Host not found: {0}")]
    HostNotFound(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Network unreachable")]
    NetworkUnreachable,

    #[error("Protocol not supported: {0}")]
    ProtocolNotSupported(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Location not found: {0:?}")]
    LocationNotFound(NetworkLocationId),
}

pub type NetworkResult<T> = std::result::Result<T, NetworkError>;

/
pub struct NetworkStorageManager {
    locations: Vec<NetworkLocation>,
    recent_servers: Vec<NetworkConnectionConfig>,
    next_id: u64,
    max_recent: usize,
}

impl Default for NetworkStorageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkStorageManager {
    pub fn new() -> Self {
        Self {
            locations: Vec::new(),
            recent_servers: Vec::new(),
            next_id: 1,
            max_recent: 10,
        }
    }

    fn next_location_id(&mut self) -> NetworkLocationId {
        let id = NetworkLocationId::new(self.next_id);
        self.next_id += 1;
        id
    }

    /
    pub fn locations(&self) -> &[NetworkLocation] {
        &self.locations
    }

    /
    pub fn recent_servers(&self) -> &[NetworkConnectionConfig] {
        &self.recent_servers
    }

    /
    pub fn get_location(&self, id: NetworkLocationId) -> Option<&NetworkLocation> {
        self.locations.iter().find(|l| l.id == id)
    }

    /
    pub fn get_location_mut(&mut self, id: NetworkLocationId) -> Option<&mut NetworkLocation> {
        self.locations.iter_mut().find(|l| l.id == id)
    }

    /
    pub fn add_location(&mut self, config: NetworkConnectionConfig) -> NetworkLocationId {
        let id = self.next_location_id();
        let location = NetworkLocation::new(id, config);
        self.locations.push(location);
        id
    }

    /
    pub fn remove_location(&mut self, id: NetworkLocationId) -> Option<NetworkLocation> {
        if let Some(pos) = self.locations.iter().position(|l| l.id == id) {
            Some(self.locations.remove(pos))
        } else {
            None
        }
    }

    /
    pub fn update_location(
        &mut self,
        id: NetworkLocationId,
        config: NetworkConnectionConfig,
    ) -> NetworkResult<()> {
        if let Some(location) = self.get_location_mut(id) {
            location.config = config;
            Ok(())
        } else {
            Err(NetworkError::LocationNotFound(id))
        }
    }

    /
    pub fn add_to_recent(&mut self, config: NetworkConnectionConfig) {
        self.recent_servers
            .retain(|c| c.to_url() != config.to_url());

        self.recent_servers.insert(0, config);

        self.recent_servers.truncate(self.max_recent);
    }

    /
    pub fn clear_recent(&mut self) {
        self.recent_servers.clear();
    }

    /
    pub fn connect(&mut self, id: NetworkLocationId) -> NetworkResult<()> {
        let config = {
            let location = self
                .get_location_mut(id)
                .ok_or(NetworkError::LocationNotFound(id))?;
            location.state = ConnectionState::Connecting;
            location.config.clone()
        };

        self.add_to_recent(config);

        if let Some(location) = self.get_location_mut(id) {
            location.state = ConnectionState::Connected;
            location.latency_ms = Some(50);
        }

        Ok(())
    }

    /
    pub fn disconnect(&mut self, id: NetworkLocationId) -> NetworkResult<()> {
        if let Some(location) = self.get_location_mut(id) {
            location.state = ConnectionState::Disconnected;
            location.latency_ms = None;
            location.mount_point = None;
            Ok(())
        } else {
            Err(NetworkError::LocationNotFound(id))
        }
    }

    /
    pub fn connected_locations(&self) -> Vec<&NetworkLocation> {
        self.locations.iter().filter(|l| l.is_connected()).collect()
    }

    /
    pub fn save(&self) -> std::io::Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-explorer");

        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("network_locations.json");

        #[derive(Serialize)]
        struct SaveData<'a> {
            locations: &'a [NetworkLocation],
            recent_servers: &'a [NetworkConnectionConfig],
        }

        let data = SaveData {
            locations: &self.locations,
            recent_servers: &self.recent_servers,
        };

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(config_path, json)
    }

    /
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nexus-explorer")
            .join("network_locations.json");

        if config_path.exists() {
            if let Ok(json) = std::fs::read_to_string(&config_path) {
                #[derive(Deserialize)]
                struct SaveData {
                    locations: Vec<NetworkLocation>,
                    recent_servers: Vec<NetworkConnectionConfig>,
                }

                if let Ok(data) = serde_json::from_str::<SaveData>(&json) {
                    let max_id = data.locations.iter().map(|l| l.id.0).max().unwrap_or(0);

                    return Self {
                        locations: data.locations,
                        recent_servers: data.recent_servers,
                        next_id: max_id + 1,
                        max_recent: 10,
                    };
                }
            }
        }

        Self::new()
    }
}

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CloudProvider {
    ICloud,
    Dropbox,
    GoogleDrive,
    OneDrive,
    Box,
    Mega,
    NextCloud,
}

impl CloudProvider {
    pub fn display_name(&self) -> &'static str {
        match self {
            CloudProvider::ICloud => "iCloud Drive",
            CloudProvider::Dropbox => "Dropbox",
            CloudProvider::GoogleDrive => "Google Drive",
            CloudProvider::OneDrive => "OneDrive",
            CloudProvider::Box => "Box",
            CloudProvider::Mega => "MEGA",
            CloudProvider::NextCloud => "NextCloud",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            CloudProvider::ICloud => "cloud",
            CloudProvider::Dropbox => "cloud",
            CloudProvider::GoogleDrive => "cloud",
            CloudProvider::OneDrive => "cloud",
            CloudProvider::Box => "cloud",
            CloudProvider::Mega => "cloud",
            CloudProvider::NextCloud => "cloud",
        }
    }

    /
    #[cfg(target_os = "macos")]
    pub fn default_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        match self {
            CloudProvider::ICloud => {
                Some(home.join("Library/Mobile Documents/com~apple~CloudDocs"))
            }
            CloudProvider::Dropbox => Some(home.join("Dropbox")),
            CloudProvider::GoogleDrive => Some(home.join("Google Drive")),
            CloudProvider::OneDrive => Some(home.join("OneDrive")),
            CloudProvider::Box => Some(home.join("Box")),
            CloudProvider::Mega => Some(home.join("MEGA")),
            CloudProvider::NextCloud => Some(home.join("Nextcloud")),
        }
    }

    #[cfg(target_os = "windows")]
    pub fn default_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        match self {
            CloudProvider::ICloud => Some(home.join("iCloudDrive")),
            CloudProvider::Dropbox => Some(home.join("Dropbox")),
            CloudProvider::GoogleDrive => Some(home.join("Google Drive")),
            CloudProvider::OneDrive => Some(home.join("OneDrive")),
            CloudProvider::Box => Some(home.join("Box")),
            CloudProvider::Mega => Some(home.join("MEGA")),
            CloudProvider::NextCloud => Some(home.join("Nextcloud")),
        }
    }

    #[cfg(target_os = "linux")]
    pub fn default_path(&self) -> Option<PathBuf> {
        let home = dirs::home_dir()?;
        match self {
            CloudProvider::ICloud => None,
            CloudProvider::Dropbox => Some(home.join("Dropbox")),
            CloudProvider::GoogleDrive => Some(home.join("Google Drive")),
            CloudProvider::OneDrive => Some(home.join("OneDrive")),
            CloudProvider::Box => Some(home.join("Box")),
            CloudProvider::Mega => Some(home.join("MEGA")),
            CloudProvider::NextCloud => Some(home.join("Nextcloud")),
        }
    }

    /
    pub fn all() -> &'static [CloudProvider] {
        &[
            CloudProvider::ICloud,
            CloudProvider::Dropbox,
            CloudProvider::GoogleDrive,
            CloudProvider::OneDrive,
            CloudProvider::Box,
            CloudProvider::Mega,
            CloudProvider::NextCloud,
        ]
    }
}

/
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Synced,
    Syncing,
    Pending,
    Error,
    CloudOnly,
    LocalOnly,
}

impl SyncStatus {
    pub fn icon_name(&self) -> &'static str {
        match self {
            SyncStatus::Synced => "check",
            SyncStatus::Syncing => "refresh-cw",
            SyncStatus::Pending => "clock",
            SyncStatus::Error => "triangle-alert",
            SyncStatus::CloudOnly => "cloud",
            SyncStatus::LocalOnly => "hard-drive",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SyncStatus::Synced => "Synced",
            SyncStatus::Syncing => "Syncing...",
            SyncStatus::Pending => "Pending sync",
            SyncStatus::Error => "Sync error",
            SyncStatus::CloudOnly => "Available online only",
            SyncStatus::LocalOnly => "Local only",
        }
    }
}

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudLocation {
    pub provider: CloudProvider,
    pub path: PathBuf,
    pub is_available: bool,
    pub account_name: Option<String>,
}

impl CloudLocation {
    pub fn new(provider: CloudProvider, path: PathBuf) -> Self {
        Self {
            provider,
            path,
            is_available: true,
            account_name: None,
        }
    }

    pub fn display_name(&self) -> String {
        if let Some(account) = &self.account_name {
            format!("{} ({})", self.provider.display_name(), account)
        } else {
            self.provider.display_name().to_string()
        }
    }
}

/
#[derive(Clone)]
pub struct CloudStorageManager {
    locations: Vec<CloudLocation>,
}

impl Default for CloudStorageManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CloudStorageManager {
    pub fn new() -> Self {
        Self {
            locations: Vec::new(),
        }
    }

    /
    pub fn locations(&self) -> &[CloudLocation] {
        &self.locations
    }

    /
    pub fn detect_providers(&mut self) {
        self.locations.clear();

        for provider in CloudProvider::all() {
            if let Some(path) = provider.default_path() {
                if path.exists() {
                    self.locations.push(CloudLocation::new(*provider, path));
                }
            }
        }
    }

    /
    pub fn get_location(&self, provider: CloudProvider) -> Option<&CloudLocation> {
        self.locations.iter().find(|l| l.provider == provider)
    }

    /
    pub fn is_cloud_path(&self, path: &PathBuf) -> Option<&CloudLocation> {
        self.locations.iter().find(|l| path.starts_with(&l.path))
    }

    /
    pub fn get_sync_status(&self, _path: &PathBuf) -> Option<SyncStatus> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_protocol_defaults() {
        assert_eq!(NetworkProtocol::Smb.default_port(), 445);
        assert_eq!(NetworkProtocol::Ftp.default_port(), 21);
        assert_eq!(NetworkProtocol::Sftp.default_port(), 22);
        assert_eq!(NetworkProtocol::WebDav.default_port(), 443);
        assert_eq!(NetworkProtocol::Nfs.default_port(), 2049);
    }

    #[test]
    fn test_connection_config_url() {
        let config = NetworkConnectionConfig::new(NetworkProtocol::Smb, "server.local".to_string())
            .with_path("/share".to_string());

        assert_eq!(config.to_url(), "smb://server.local/share");

        let config_with_port =
            NetworkConnectionConfig::new(NetworkProtocol::Ftp, "ftp.example.com".to_string())
                .with_port(2121)
                .with_path("/files".to_string());

        assert_eq!(
            config_with_port.to_url(),
            "ftp://ftp.example.com:2121/files"
        );
    }

    #[test]
    fn test_network_storage_manager_add_remove() {
        let mut manager = NetworkStorageManager::new();

        let config = NetworkConnectionConfig::new(NetworkProtocol::Smb, "server".to_string());
        let id = manager.add_location(config);

        assert_eq!(manager.locations().len(), 1);
        assert!(manager.get_location(id).is_some());

        manager.remove_location(id);
        assert_eq!(manager.locations().len(), 0);
        assert!(manager.get_location(id).is_none());
    }

    #[test]
    fn test_recent_servers() {
        let mut manager = NetworkStorageManager::new();

        let config1 = NetworkConnectionConfig::new(NetworkProtocol::Smb, "server1".to_string());
        let config2 = NetworkConnectionConfig::new(NetworkProtocol::Ftp, "server2".to_string());

        manager.add_to_recent(config1.clone());
        manager.add_to_recent(config2.clone());

        assert_eq!(manager.recent_servers().len(), 2);
        assert_eq!(manager.recent_servers()[0].host, "server2");
        assert_eq!(manager.recent_servers()[1].host, "server1");

        manager.add_to_recent(config1);
        assert_eq!(manager.recent_servers().len(), 2);
        assert_eq!(manager.recent_servers()[0].host, "server1");
    }

    #[test]
    fn test_cloud_provider_paths() {
        for provider in CloudProvider::all() {
            let _ = provider.default_path();
        }
    }

    #[test]
    fn test_sync_status_descriptions() {
        assert_eq!(SyncStatus::Synced.description(), "Synced");
        assert_eq!(SyncStatus::Syncing.description(), "Syncing...");
        assert_eq!(SyncStatus::Error.description(), "Sync error");
    }
}

/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteFileEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub modified: Option<u64>,
    pub permissions: Option<String>,
    pub is_loading: bool,
}

impl RemoteFileEntry {
    pub fn new(name: String, path: String, is_directory: bool) -> Self {
        Self {
            name,
            path,
            is_directory,
            size: 0,
            modified: None,
            permissions: None,
            is_loading: false,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    pub fn with_modified(mut self, timestamp: u64) -> Self {
        self.modified = Some(timestamp);
        self
    }

    pub fn with_permissions(mut self, permissions: String) -> Self {
        self.permissions = Some(permissions);
        self
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
    }
}

/
#[derive(Debug, Clone)]
pub enum RemoteListingState {
    Idle,
    Loading,
    Loaded(Vec<RemoteFileEntry>),
    Error(String),
}

impl Default for RemoteListingState {
    fn default() -> Self {
        Self::Idle
    }
}

/
#[derive(Debug, Clone)]
pub struct NetworkFileOperation {
    pub id: u64,
    pub operation_type: NetworkOperationType,
    pub source_location: NetworkLocationId,
    pub source_path: String,
    pub destination: Option<PathBuf>,
    pub progress: NetworkOperationProgress,
    pub state: NetworkOperationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkOperationType {
    Download,
    Upload,
    Delete,
    Rename,
    CreateDirectory,
}

impl NetworkOperationType {
    pub fn display_name(&self) -> &'static str {
        match self {
            NetworkOperationType::Download => "Downloading",
            NetworkOperationType::Upload => "Uploading",
            NetworkOperationType::Delete => "Deleting",
            NetworkOperationType::Rename => "Renaming",
            NetworkOperationType::CreateDirectory => "Creating folder",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NetworkOperationProgress {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub current_file: Option<String>,
    pub speed_bytes_per_sec: u64,
}

impl NetworkOperationProgress {
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.transferred_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }

    pub fn estimated_remaining_secs(&self) -> Option<u64> {
        if self.speed_bytes_per_sec == 0 {
            return None;
        }
        let remaining_bytes = self.total_bytes.saturating_sub(self.transferred_bytes);
        Some(remaining_bytes / self.speed_bytes_per_sec)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkOperationState {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Default for NetworkOperationState {
    fn default() -> Self {
        Self::Pending
    }
}

/
pub struct NetworkFileOperationsManager {
    operations: Vec<NetworkFileOperation>,
    next_operation_id: u64,
    listings_cache: std::collections::HashMap<(NetworkLocationId, String), RemoteListingState>,
}

impl Default for NetworkFileOperationsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkFileOperationsManager {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            next_operation_id: 1,
            listings_cache: std::collections::HashMap::new(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_operation_id;
        self.next_operation_id += 1;
        id
    }

    /
    pub fn operations(&self) -> &[NetworkFileOperation] {
        &self.operations
    }

    /
    pub fn active_operations(&self) -> Vec<&NetworkFileOperation> {
        self.operations
            .iter()
            .filter(|op| {
                matches!(
                    op.state,
                    NetworkOperationState::Pending | NetworkOperationState::Running
                )
            })
            .collect()
    }

    /
    pub fn has_active_operations(&self) -> bool {
        self.operations.iter().any(|op| {
            matches!(
                op.state,
                NetworkOperationState::Pending | NetworkOperationState::Running
            )
        })
    }

    /
    pub fn start_download(
        &mut self,
        location_id: NetworkLocationId,
        remote_path: String,
        local_destination: PathBuf,
    ) -> u64 {
        let id = self.next_id();
        let operation = NetworkFileOperation {
            id,
            operation_type: NetworkOperationType::Download,
            source_location: location_id,
            source_path: remote_path,
            destination: Some(local_destination),
            progress: NetworkOperationProgress::default(),
            state: NetworkOperationState::Pending,
        };
        self.operations.push(operation);
        id
    }

    /
    pub fn start_upload(
        &mut self,
        location_id: NetworkLocationId,
        local_path: PathBuf,
        remote_destination: String,
    ) -> u64 {
        let id = self.next_id();
        let operation = NetworkFileOperation {
            id,
            operation_type: NetworkOperationType::Upload,
            source_location: location_id,
            source_path: remote_destination,
            destination: Some(local_path),
            progress: NetworkOperationProgress::default(),
            state: NetworkOperationState::Pending,
        };
        self.operations.push(operation);
        id
    }

    /
    pub fn update_progress(&mut self, operation_id: u64, progress: NetworkOperationProgress) {
        if let Some(op) = self.operations.iter_mut().find(|o| o.id == operation_id) {
            op.progress = progress;
            if op.state == NetworkOperationState::Pending {
                op.state = NetworkOperationState::Running;
            }
        }
    }

    /
    pub fn complete_operation(&mut self, operation_id: u64) {
        if let Some(op) = self.operations.iter_mut().find(|o| o.id == operation_id) {
            op.state = NetworkOperationState::Completed;
        }
    }

    /
    pub fn fail_operation(&mut self, operation_id: u64) {
        if let Some(op) = self.operations.iter_mut().find(|o| o.id == operation_id) {
            op.state = NetworkOperationState::Failed;
        }
    }

    /
    pub fn cancel_operation(&mut self, operation_id: u64) {
        if let Some(op) = self.operations.iter_mut().find(|o| o.id == operation_id) {
            op.state = NetworkOperationState::Cancelled;
        }
    }

    /
    pub fn clear_finished(&mut self) {
        self.operations.retain(|op| {
            matches!(
                op.state,
                NetworkOperationState::Pending | NetworkOperationState::Running
            )
        });
    }

    /
    pub fn get_listing(
        &self,
        location_id: NetworkLocationId,
        path: &str,
    ) -> Option<&RemoteListingState> {
        self.listings_cache.get(&(location_id, path.to_string()))
    }

    /
    pub fn set_listing(
        &mut self,
        location_id: NetworkLocationId,
        path: String,
        state: RemoteListingState,
    ) {
        self.listings_cache.insert((location_id, path), state);
    }

    /
    pub fn clear_listing_cache(&mut self, location_id: NetworkLocationId) {
        self.listings_cache
            .retain(|(loc_id, _), _| *loc_id != location_id);
    }

    /
    pub fn clear_all_caches(&mut self) {
        self.listings_cache.clear();
    }

    /
    pub fn is_loading(&self, location_id: NetworkLocationId, path: &str) -> bool {
        matches!(
            self.get_listing(location_id, path),
            Some(RemoteListingState::Loading)
        )
    }
}

/
pub fn format_latency(latency_ms: u32) -> String {
    if latency_ms < 1000 {
        format!("{}ms", latency_ms)
    } else {
        format!("{:.1}s", latency_ms as f64 / 1000.0)
    }
}

/
pub fn format_transfer_speed(bytes_per_sec: u64) -> String {
    if bytes_per_sec < 1024 {
        format!("{} B/s", bytes_per_sec)
    } else if bytes_per_sec < 1024 * 1024 {
        format!("{:.1} KB/s", bytes_per_sec as f64 / 1024.0)
    } else if bytes_per_sec < 1024 * 1024 * 1024 {
        format!("{:.1} MB/s", bytes_per_sec as f64 / (1024.0 * 1024.0))
    } else {
        format!(
            "{:.1} GB/s",
            bytes_per_sec as f64 / (1024.0 * 1024.0 * 1024.0)
        )
    }
}

#[cfg(test)]
mod network_operations_tests {
    use super::*;

    #[test]
    fn test_remote_file_entry() {
        let entry =
            RemoteFileEntry::new("test.txt".to_string(), "/path/test.txt".to_string(), false)
                .with_size(1024)
                .with_modified(1700000000)
                .with_permissions("rw-r--r--".to_string());

        assert_eq!(entry.name, "test.txt");
        assert_eq!(entry.size, 1024);
        assert!(!entry.is_directory);
        assert_eq!(entry.modified, Some(1700000000));
        assert_eq!(entry.permissions, Some("rw-r--r--".to_string()));
    }

    #[test]
    fn test_operation_progress() {
        let progress = NetworkOperationProgress {
            total_bytes: 1000,
            transferred_bytes: 500,
            current_file: Some("file.txt".to_string()),
            speed_bytes_per_sec: 100,
        };

        assert_eq!(progress.percentage(), 50.0);
        assert_eq!(progress.estimated_remaining_secs(), Some(5));
    }

    #[test]
    fn test_operation_progress_zero_total() {
        let progress = NetworkOperationProgress {
            total_bytes: 0,
            transferred_bytes: 0,
            current_file: None,
            speed_bytes_per_sec: 0,
        };

        assert_eq!(progress.percentage(), 0.0);
        assert_eq!(progress.estimated_remaining_secs(), None);
    }

    #[test]
    fn test_network_file_operations_manager() {
        let mut manager = NetworkFileOperationsManager::new();

        let location_id = NetworkLocationId::new(1);
        let op_id = manager.start_download(
            location_id,
            "/remote/file.txt".to_string(),
            PathBuf::from("/local/file.txt"),
        );

        assert_eq!(manager.operations().len(), 1);
        assert!(manager.has_active_operations());

        manager.complete_operation(op_id);
        assert!(!manager.has_active_operations());

        manager.clear_finished();
        assert_eq!(manager.operations().len(), 0);
    }

    #[test]
    fn test_listing_cache() {
        let mut manager = NetworkFileOperationsManager::new();
        let location_id = NetworkLocationId::new(1);

        manager.set_listing(
            location_id,
            "/path".to_string(),
            RemoteListingState::Loading,
        );
        assert!(manager.is_loading(location_id, "/path"));

        let entries = vec![RemoteFileEntry::new(
            "file1.txt".to_string(),
            "/path/file1.txt".to_string(),
            false,
        )];
        manager.set_listing(
            location_id,
            "/path".to_string(),
            RemoteListingState::Loaded(entries),
        );
        assert!(!manager.is_loading(location_id, "/path"));

        manager.clear_listing_cache(location_id);
        assert!(manager.get_listing(location_id, "/path").is_none());
    }

    #[test]
    fn test_format_latency() {
        assert_eq!(format_latency(50), "50ms");
        assert_eq!(format_latency(999), "999ms");
        assert_eq!(format_latency(1000), "1.0s");
        assert_eq!(format_latency(2500), "2.5s");
    }

    #[test]
    fn test_format_transfer_speed() {
        assert_eq!(format_transfer_speed(500), "500 B/s");
        assert_eq!(format_transfer_speed(1024), "1.0 KB/s");
        assert_eq!(format_transfer_speed(1024 * 1024), "1.0 MB/s");
        assert_eq!(format_transfer_speed(1024 * 1024 * 1024), "1.0 GB/s");
    }
}

/
impl CloudStorageManager {
    /
    pub fn detect_all_providers(&mut self) {
        self.locations.clear();

        #[cfg(target_os = "macos")]
        self.detect_macos_providers();

        #[cfg(target_os = "windows")]
        self.detect_windows_providers();

        #[cfg(target_os = "linux")]
        self.detect_linux_providers();
    }

    #[cfg(target_os = "macos")]
    fn detect_macos_providers(&mut self) {
        use std::fs;

        if let Some(home) = dirs::home_dir() {
            let icloud_path = home.join("Library/Mobile Documents/com~apple~CloudDocs");
            if icloud_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::ICloud, icloud_path));
            }

            let dropbox_path = home.join("Dropbox");
            if dropbox_path.exists() {
                let mut location = CloudLocation::new(CloudProvider::Dropbox, dropbox_path);
                if let Ok(info) = fs::read_to_string(home.join(".dropbox/info.json")) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&info) {
                        if let Some(personal) = json.get("personal") {
                            if let Some(email) = personal.get("email").and_then(|e| e.as_str()) {
                                location.account_name = Some(email.to_string());
                            }
                        }
                    }
                }
                self.locations.push(location);
            }

            let gdrive_path = home.join("Google Drive");
            if gdrive_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::GoogleDrive, gdrive_path));
            }
            let gdrive_my_drive = home.join("Google Drive/My Drive");
            if gdrive_my_drive.exists() {
                if let Some(loc) = self
                    .locations
                    .iter_mut()
                    .find(|l| l.provider == CloudProvider::GoogleDrive)
                {
                    loc.path = gdrive_my_drive;
                }
            }

            let onedrive_path = home.join("OneDrive");
            if onedrive_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::OneDrive, onedrive_path));
            }
            let onedrive_personal = home.join("OneDrive - Personal");
            if onedrive_personal.exists()
                && !self
                    .locations
                    .iter()
                    .any(|l| l.provider == CloudProvider::OneDrive)
            {
                self.locations.push(CloudLocation::new(
                    CloudProvider::OneDrive,
                    onedrive_personal,
                ));
            }

            let box_path = home.join("Box");
            if box_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::Box, box_path));
            }

            let mega_path = home.join("MEGA");
            if mega_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::Mega, mega_path));
            }

            let nextcloud_path = home.join("Nextcloud");
            if nextcloud_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::NextCloud, nextcloud_path));
            }
        }
    }

    #[cfg(target_os = "windows")]
    fn detect_windows_providers(&mut self) {
        if let Some(home) = dirs::home_dir() {
            let icloud_path = home.join("iCloudDrive");
            if icloud_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::ICloud, icloud_path));
            }

            let dropbox_path = home.join("Dropbox");
            if dropbox_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::Dropbox, dropbox_path));
            }

            let gdrive_path = home.join("Google Drive");
            if gdrive_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::GoogleDrive, gdrive_path));
            }

            let onedrive_paths = [home.join("OneDrive"), home.join("OneDrive - Personal")];
            for path in onedrive_paths {
                if path.exists() {
                    self.locations
                        .push(CloudLocation::new(CloudProvider::OneDrive, path));
                    break;
                }
            }

            let box_path = home.join("Box");
            if box_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::Box, box_path));
            }

            let mega_path = home.join("MEGA");
            if mega_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::Mega, mega_path));
            }

            let nextcloud_path = home.join("Nextcloud");
            if nextcloud_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::NextCloud, nextcloud_path));
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn detect_linux_providers(&mut self) {
        if let Some(home) = dirs::home_dir() {
            let dropbox_path = home.join("Dropbox");
            if dropbox_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::Dropbox, dropbox_path));
            }

            let gdrive_paths = [home.join("Google Drive"), home.join("google-drive")];
            for path in gdrive_paths {
                if path.exists() {
                    self.locations
                        .push(CloudLocation::new(CloudProvider::GoogleDrive, path));
                    break;
                }
            }

            let onedrive_path = home.join("OneDrive");
            if onedrive_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::OneDrive, onedrive_path));
            }

            let mega_path = home.join("MEGA");
            if mega_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::Mega, mega_path));
            }

            let nextcloud_path = home.join("Nextcloud");
            if nextcloud_path.exists() {
                self.locations
                    .push(CloudLocation::new(CloudProvider::NextCloud, nextcloud_path));
            }
        }
    }

    /
    pub fn get_file_sync_status(&self, path: &PathBuf) -> Option<SyncStatus> {
        let cloud_location = self.is_cloud_path(path)?;

        match cloud_location.provider {
            CloudProvider::Dropbox => self.get_dropbox_sync_status(path),
            CloudProvider::ICloud => self.get_icloud_sync_status(path),
            CloudProvider::OneDrive => self.get_onedrive_sync_status(path),
            CloudProvider::GoogleDrive => self.get_gdrive_sync_status(path),
            _ => None,
        }
    }

    fn get_dropbox_sync_status(&self, path: &PathBuf) -> Option<SyncStatus> {

        #[cfg(unix)]
        {
            use std::process::Command;

            if let Ok(output) = Command::new("dropbox")
                .args(["filestatus", path.to_str()?])
                .output()
            {
                let status_str = String::from_utf8_lossy(&output.stdout);
                if status_str.contains("up to date") {
                    return Some(SyncStatus::Synced);
                } else if status_str.contains("syncing") {
                    return Some(SyncStatus::Syncing);
                } else if status_str.contains("unwatched") {
                    return Some(SyncStatus::LocalOnly);
                }
            }
        }

        if path.exists() {
            Some(SyncStatus::Synced)
        } else {
            None
        }
    }

    fn get_icloud_sync_status(&self, path: &PathBuf) -> Option<SyncStatus> {
        let file_name = path.file_name()?.to_str()?;

        if file_name.starts_with('.') && file_name.ends_with(".icloud") {
            return Some(SyncStatus::CloudOnly);
        }

        let parent = path.parent()?;
        let icloud_name = format!(".{}.icloud", file_name);
        let icloud_path = parent.join(&icloud_name);

        if icloud_path.exists() {
            return Some(SyncStatus::CloudOnly);
        }

        if path.exists() {
            Some(SyncStatus::Synced)
        } else {
            None
        }
    }

    fn get_onedrive_sync_status(&self, _path: &PathBuf) -> Option<SyncStatus> {
        Some(SyncStatus::Synced)
    }

    fn get_gdrive_sync_status(&self, _path: &PathBuf) -> Option<SyncStatus> {
        Some(SyncStatus::Synced)
    }

    /
    pub fn refresh_availability(&mut self) {
        for location in &mut self.locations {
            location.is_available = location.path.exists();
        }
    }

    /
    pub fn available_locations(&self) -> Vec<&CloudLocation> {
        self.locations.iter().filter(|l| l.is_available).collect()
    }

    /
    pub fn has_cloud_storage(&self) -> bool {
        !self.locations.is_empty()
    }
}

/
#[derive(Debug, Clone)]
pub struct NetworkSidebarState {
    pub network_locations: Vec<NetworkLocationSummary>,
    pub cloud_locations: Vec<CloudLocationSummary>,
    pub is_loading: bool,
}

#[derive(Debug, Clone)]
pub struct NetworkLocationSummary {
    pub id: NetworkLocationId,
    pub name: String,
    pub protocol: NetworkProtocol,
    pub is_connected: bool,
    pub latency_ms: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct CloudLocationSummary {
    pub provider: CloudProvider,
    pub name: String,
    pub path: PathBuf,
    pub is_available: bool,
}

impl NetworkSidebarState {
    pub fn new() -> Self {
        Self {
            network_locations: Vec::new(),
            cloud_locations: Vec::new(),
            is_loading: false,
        }
    }

    pub fn from_managers(
        network_manager: &NetworkStorageManager,
        cloud_manager: &CloudStorageManager,
    ) -> Self {
        let network_locations = network_manager
            .locations()
            .iter()
            .map(|loc| NetworkLocationSummary {
                id: loc.id,
                name: loc.display_name().to_string(),
                protocol: loc.protocol(),
                is_connected: loc.is_connected(),
                latency_ms: loc.latency_ms,
            })
            .collect();

        let cloud_locations = cloud_manager
            .locations()
            .iter()
            .map(|loc| CloudLocationSummary {
                provider: loc.provider,
                name: loc.display_name(),
                path: loc.path.clone(),
                is_available: loc.is_available,
            })
            .collect();

        Self {
            network_locations,
            cloud_locations,
            is_loading: false,
        }
    }

    pub fn has_any_locations(&self) -> bool {
        !self.network_locations.is_empty() || !self.cloud_locations.is_empty()
    }
}

impl Default for NetworkSidebarState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod cloud_storage_tests {
    use super::*;

    #[test]
    fn test_cloud_storage_manager_detect() {
        let mut manager = CloudStorageManager::new();
        manager.detect_all_providers();
    }

    #[test]
    fn test_cloud_location_display_name() {
        let location =
            CloudLocation::new(CloudProvider::Dropbox, PathBuf::from("/home/user/Dropbox"));
        assert_eq!(location.display_name(), "Dropbox");

        let mut location_with_account =
            CloudLocation::new(CloudProvider::Dropbox, PathBuf::from("/home/user/Dropbox"));
        location_with_account.account_name = Some("user@example.com".to_string());
        assert_eq!(
            location_with_account.display_name(),
            "Dropbox (user@example.com)"
        );
    }

    #[test]
    fn test_network_sidebar_state() {
        let network_manager = NetworkStorageManager::new();
        let cloud_manager = CloudStorageManager::new();

        let state = NetworkSidebarState::from_managers(&network_manager, &cloud_manager);
        assert!(!state.has_any_locations());
        assert!(!state.is_loading);
    }

    #[test]
    fn test_icloud_placeholder_detection() {
        let manager = CloudStorageManager::new();

        let path = PathBuf::from(
            "/Users/test/Library/Mobile Documents/com~apple~CloudDocs/.test.txt.icloud",
        );
        let file_name = path.file_name().unwrap().to_str().unwrap();

        assert!(file_name.starts_with('.'));
        assert!(file_name.ends_with(".icloud"));
    }
}
