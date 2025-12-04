use gpui::*;
use crate::models::{NetworkConnectionConfig, NetworkProtocol, AuthMethod};

/// Actions for the network connection dialog
#[derive(Clone, PartialEq)]
pub enum NetworkDialogAction {
    Connect,
    Cancel,
    ProtocolChanged(NetworkProtocol),
    HostChanged(String),
    PortChanged(Option<u16>),
    PathChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    LabelChanged(String),
    UseAnonymous(bool),
}

/// State for the network connection dialog
pub struct NetworkConnectionDialog {
    protocol: NetworkProtocol,
    host: String,
    port: String,
    path: String,
    username: String,
    password: String,
    label: String,
    use_anonymous: bool,
    error_message: Option<String>,
    is_connecting: bool,
    on_connect: Option<Box<dyn Fn(NetworkConnectionConfig) + Send + Sync>>,
    on_cancel: Option<Box<dyn Fn() + Send + Sync>>,
}

impl NetworkConnectionDialog {
    pub fn new() -> Self {
        Self {
            protocol: NetworkProtocol::Smb,
            host: String::new(),
            port: String::new(),
            path: String::from("/"),
            username: String::new(),
            password: String::new(),
            label: String::new(),
            use_anonymous: true,
            error_message: None,
            is_connecting: false,
            on_connect: None,
            on_cancel: None,
        }
    }

    pub fn with_on_connect<F>(mut self, callback: F) -> Self
    where
        F: Fn(NetworkConnectionConfig) + Send + Sync + 'static,
    {
        self.on_connect = Some(Box::new(callback));
        self
    }

    pub fn with_on_cancel<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.on_cancel = Some(Box::new(callback));
        self
    }

    /// Build the connection configuration from current state
    pub fn build_config(&self) -> Option<NetworkConnectionConfig> {
        if self.host.is_empty() {
            return None;
        }

        let port = self.port.parse::<u16>().ok();
        
        let auth = if self.use_anonymous {
            AuthMethod::Anonymous
        } else {
            AuthMethod::Password {
                username: self.username.clone(),
                password: self.password.clone(),
            }
        };

        let mut config = NetworkConnectionConfig::new(self.protocol, self.host.clone())
            .with_path(self.path.clone())
            .with_auth(auth);

        if let Some(p) = port {
            config = config.with_port(p);
        }

        if !self.label.is_empty() {
            config = config.with_label(self.label.clone());
        }

        Some(config)
    }

    /// Validate the current input
    pub fn validate(&self) -> Result<(), String> {
        if self.host.is_empty() {
            return Err("Server address is required".to_string());
        }

        if !self.port.is_empty() {
            if self.port.parse::<u16>().is_err() {
                return Err("Invalid port number".to_string());
            }
        }

        if !self.use_anonymous && self.username.is_empty() {
            return Err("Username is required for authenticated connections".to_string());
        }

        Ok(())
    }

    /// Handle connect action
    pub fn handle_connect(&mut self) {
        if let Err(msg) = self.validate() {
            self.error_message = Some(msg);
            return;
        }

        self.error_message = None;
        self.is_connecting = true;

        if let Some(config) = self.build_config() {
            if let Some(callback) = &self.on_connect {
                callback(config);
            }
        }
    }

    /// Handle cancel action
    pub fn handle_cancel(&self) {
        if let Some(callback) = &self.on_cancel {
            callback();
        }
    }

    /// Set protocol
    pub fn set_protocol(&mut self, protocol: NetworkProtocol) {
        self.protocol = protocol;
        if self.port.is_empty() {
            // Keep empty to use default
        }
    }

    /// Set host
    pub fn set_host(&mut self, host: String) {
        self.host = host;
        self.error_message = None;
    }

    /// Set port
    pub fn set_port(&mut self, port: String) {
        self.port = port;
        self.error_message = None;
    }

    /// Set path
    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }

    /// Set username
    pub fn set_username(&mut self, username: String) {
        self.username = username;
        self.error_message = None;
    }

    /// Set password
    pub fn set_password(&mut self, password: String) {
        self.password = password;
    }

    /// Set label
    pub fn set_label(&mut self, label: String) {
        self.label = label;
    }

    /// Set anonymous auth
    pub fn set_use_anonymous(&mut self, anonymous: bool) {
        self.use_anonymous = anonymous;
        self.error_message = None;
    }

    /// Get current protocol
    pub fn protocol(&self) -> NetworkProtocol {
        self.protocol
    }

    /// Get current host
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Get current port string
    pub fn port_str(&self) -> &str {
        &self.port
    }

    /// Get current path
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get current username
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Get current label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Check if using anonymous auth
    pub fn is_anonymous(&self) -> bool {
        self.use_anonymous
    }

    /// Get error message if any
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// Check if currently connecting
    pub fn is_connecting(&self) -> bool {
        self.is_connecting
    }

    /// Get port placeholder based on protocol
    pub fn port_placeholder(&self) -> String {
        format!("{}", self.protocol.default_port())
    }

    /// Get available protocols
    pub fn available_protocols() -> &'static [NetworkProtocol] {
        &[
            NetworkProtocol::Smb,
            NetworkProtocol::Ftp,
            NetworkProtocol::Sftp,
            NetworkProtocol::WebDav,
            NetworkProtocol::Nfs,
        ]
    }
}

impl Default for NetworkConnectionDialog {
    fn default() -> Self {
        Self::new()
    }
}
