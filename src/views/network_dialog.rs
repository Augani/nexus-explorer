use crate::models::{AuthMethod, NetworkConnectionConfig, NetworkProtocol};
use gpui::*;


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


    pub fn handle_cancel(&self) {
        if let Some(callback) = &self.on_cancel {
            callback();
        }
    }


    pub fn set_protocol(&mut self, protocol: NetworkProtocol) {
        self.protocol = protocol;
        if self.port.is_empty() {
        }
    }


    pub fn set_host(&mut self, host: String) {
        self.host = host;
        self.error_message = None;
    }


    pub fn set_port(&mut self, port: String) {
        self.port = port;
        self.error_message = None;
    }


    pub fn set_path(&mut self, path: String) {
        self.path = path;
    }


    pub fn set_username(&mut self, username: String) {
        self.username = username;
        self.error_message = None;
    }


    pub fn set_password(&mut self, password: String) {
        self.password = password;
    }


    pub fn set_label(&mut self, label: String) {
        self.label = label;
    }


    pub fn set_use_anonymous(&mut self, anonymous: bool) {
        self.use_anonymous = anonymous;
        self.error_message = None;
    }


    pub fn protocol(&self) -> NetworkProtocol {
        self.protocol
    }


    pub fn host(&self) -> &str {
        &self.host
    }


    pub fn port_str(&self) -> &str {
        &self.port
    }


    pub fn path(&self) -> &str {
        &self.path
    }


    pub fn username(&self) -> &str {
        &self.username
    }


    pub fn label(&self) -> &str {
        &self.label
    }


    pub fn is_anonymous(&self) -> bool {
        self.use_anonymous
    }


    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }


    pub fn is_connecting(&self) -> bool {
        self.is_connecting
    }


    pub fn port_placeholder(&self) -> String {
        format!("{}", self.protocol.default_port())
    }


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
