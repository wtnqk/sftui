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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_basic_host() {
        let config_content = r#"
Host testserver
    HostName example.com
    User testuser
    Port 2222
    IdentityFile ~/.ssh/test_key
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();

        let mut config = SshConfig::new().unwrap();
        config.parse_config(&temp_file.path().to_path_buf()).unwrap();

        let host = config.get_host("testserver").unwrap();
        assert_eq!(host.host, "testserver");
        assert_eq!(host.hostname.as_ref().unwrap(), "example.com");
        assert_eq!(host.user.as_ref().unwrap(), "testuser");
        assert_eq!(host.port.unwrap(), 2222);
        assert_eq!(
            host.identity_file.as_ref().unwrap(),
            &PathBuf::from("~/.ssh/test_key")
        );
    }

    #[test]
    fn test_parse_multiple_hosts() {
        let config_content = r#"
Host server1
    HostName server1.example.com
    User user1

Host server2
    HostName server2.example.com
    User user2
    Port 22
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();

        let mut config = SshConfig::new().unwrap();
        config.parse_config(&temp_file.path().to_path_buf()).unwrap();

        let hosts = config.get_all_hosts();
        // Check that we have at least the 2 hosts we defined (may include default Host *)
        assert!(hosts.len() >= 2);

        let server1 = config.get_host("server1").unwrap();
        assert_eq!(server1.hostname.as_ref().unwrap(), "server1.example.com");
        assert_eq!(server1.user.as_ref().unwrap(), "user1");
        assert_eq!(server1.port, None);

        let server2 = config.get_host("server2").unwrap();
        assert_eq!(server2.hostname.as_ref().unwrap(), "server2.example.com");
        assert_eq!(server2.user.as_ref().unwrap(), "user2");
        assert_eq!(server2.port, Some(22));
    }

    #[test]
    fn test_parse_with_comments_and_empty_lines() {
        let config_content = r#"
# This is a comment
Host myserver
    # Another comment
    HostName example.com
    
    User myuser
    # Port 2222
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();

        let mut config = SshConfig::new().unwrap();
        config.parse_config(&temp_file.path().to_path_buf()).unwrap();

        let host = config.get_host("myserver").unwrap();
        assert_eq!(host.hostname.as_ref().unwrap(), "example.com");
        assert_eq!(host.user.as_ref().unwrap(), "myuser");
        assert_eq!(host.port, None); // Commented out port should not be parsed
    }

    #[test]
    fn test_nonexistent_host() {
        let config = SshConfig::new().unwrap();
        assert!(config.get_host("nonexistent").is_none());
    }

    #[test]
    fn test_parse_invalid_port() {
        let config_content = r#"
Host testserver
    Port invalid_port
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();

        let mut config = SshConfig::new().unwrap();
        config.parse_config(&temp_file.path().to_path_buf()).unwrap();

        let host = config.get_host("testserver").unwrap();
        assert_eq!(host.port, None); // Invalid port should be ignored
    }

    #[test]
    fn test_load_from_path() {
        let config_content = r#"
Host testhost
    HostName test.example.com
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();

        let mut config = SshConfig::new().unwrap();
        config.parse_config(&temp_file.path().to_path_buf()).unwrap();
        // Check that we have at least the 1 host we defined (may include default Host *)
        assert!(config.get_all_hosts().len() >= 1);
        
        let host = config.get_host("testhost").unwrap();
        assert_eq!(host.hostname.as_ref().unwrap(), "test.example.com");
    }

    #[test]
    fn test_case_insensitive_keys() {
        let config_content = r#"
Host testserver
    HOSTNAME example.com
    USER testuser
    Port 22
    identityfile ~/.ssh/key
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();

        let mut config = SshConfig::new().unwrap();
        config.parse_config(&temp_file.path().to_path_buf()).unwrap();

        let host = config.get_host("testserver").unwrap();
        assert_eq!(host.hostname.as_ref().unwrap(), "example.com");
        assert_eq!(host.user.as_ref().unwrap(), "testuser");
        assert_eq!(host.identity_file.as_ref().unwrap(), &PathBuf::from("~/.ssh/key"));
    }
}
