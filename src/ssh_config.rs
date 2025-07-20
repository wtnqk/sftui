use anyhow::{Result, anyhow};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SshHost {
    pub host: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    pub identity_file: Option<PathBuf>,
}

pub struct SshConfig {
    hosts: HashMap<String, SshHost>,
}

impl SshConfig {
    pub fn new() -> Result<Self> {
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot find home directory"))?
            .join(".ssh")
            .join("config");

        let mut ssh_config = SshConfig {
            hosts: HashMap::new(),
        };

        if config_path.exists() {
            ssh_config.parse_config(&config_path)?;
        }

        Ok(ssh_config)
    }

    fn parse_config(&mut self, config_path: &PathBuf) -> Result<()> {
        let content = fs::read_to_string(config_path)?;
        let mut current_host: Option<SshHost> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }

            let key = parts[0].to_lowercase();
            let value = parts[1..].join(" ");

            match key.as_str() {
                "host" => {
                    if let Some(host) = current_host.take() {
                        self.hosts.insert(host.host.clone(), host);
                    }
                    current_host = Some(SshHost {
                        host: value,
                        hostname: None,
                        user: None,
                        port: None,
                        identity_file: None,
                    });
                }
                "hostname" => {
                    if let Some(ref mut host) = current_host {
                        host.hostname = Some(value);
                    }
                }
                "user" => {
                    if let Some(ref mut host) = current_host {
                        host.user = Some(value);
                    }
                }
                "port" => {
                    if let Some(ref mut host) = current_host {
                        if let Ok(port) = value.parse::<u16>() {
                            host.port = Some(port);
                        }
                    }
                }
                "identityfile" => {
                    if let Some(ref mut host) = current_host {
                        host.identity_file = Some(PathBuf::from(value));
                    }
                }
                _ => {}
            }
        }

        if let Some(host) = current_host {
            self.hosts.insert(host.host.clone(), host);
        }

        Ok(())
    }

    pub fn get_host(&self, name: &str) -> Option<&SshHost> {
        self.hosts.get(name)
    }

    pub fn get_all_hosts(&self) -> Vec<&SshHost> {
        self.hosts.values().collect()
    }
}