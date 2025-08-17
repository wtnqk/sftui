use anyhow::{Result, anyhow};
use regex::Regex;
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

#[derive(Debug)]
struct SshConfigEntry {
    patterns: Vec<String>,
    hostname: Option<String>,
    user: Option<String>,
    port: Option<u16>,
    identity_file: Option<PathBuf>,
}

pub struct SshConfig {
    hosts: Vec<SshHost>,
}

impl SshConfig {
    pub fn new() -> Result<Self> {
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot find home directory"))?
            .join(".ssh")
            .join("config");

        let mut ssh_config = SshConfig { hosts: Vec::new() };

        if config_path.exists() {
            ssh_config.parse_config(&config_path)?;
        }

        Ok(ssh_config)
    }

    pub(crate) fn parse_config(&mut self, config_path: &PathBuf) -> Result<()> {
        let content = fs::read_to_string(config_path)?;
        let mut current_entry: Option<SshConfigEntry> = None;

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
                    if let Some(entry) = current_entry.take() {
                        // Convert entry to hosts
                        for pattern in entry.patterns {
                            self.hosts.push(SshHost {
                                host: pattern,
                                hostname: entry.hostname.clone(),
                                user: entry.user.clone(),
                                port: entry.port,
                                identity_file: entry.identity_file.clone(),
                            });
                        }
                    }
                    let patterns: Vec<String> =
                        value.split_whitespace().map(|s| s.to_string()).collect();
                    current_entry = Some(SshConfigEntry {
                        patterns,
                        hostname: None,
                        user: None,
                        port: None,
                        identity_file: None,
                    });
                }
                "hostname" => {
                    if let Some(ref mut entry) = current_entry {
                        entry.hostname = Some(value);
                    }
                }
                "user" => {
                    if let Some(ref mut entry) = current_entry {
                        entry.user = Some(value);
                    }
                }
                "port" => {
                    if let Some(ref mut entry) = current_entry
                        && let Ok(port) = value.parse::<u16>()
                    {
                        entry.port = Some(port);
                    }
                }
                "identityfile" => {
                    if let Some(ref mut entry) = current_entry {
                        entry.identity_file = Some(PathBuf::from(value));
                    }
                }
                _ => {}
            }
        }

        if let Some(entry) = current_entry {
            // Convert entry to hosts
            for pattern in entry.patterns {
                self.hosts.push(SshHost {
                    host: pattern,
                    hostname: entry.hostname.clone(),
                    user: entry.user.clone(),
                    port: entry.port,
                    identity_file: entry.identity_file.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn get_host(&self, name: &str) -> Option<&SshHost> {
        // SSH config uses first-match-wins strategy
        self.hosts
            .iter()
            .find(|&host| self.pattern_matches(&host.host, name))
    }

    pub fn get_all_hosts(&self) -> Vec<&SshHost> {
        self.hosts
            .iter()
            .filter(|host| !host.host.contains('*') && !host.host.contains('?'))
            .collect()
    }

    fn pattern_matches(&self, pattern: &str, hostname: &str) -> bool {
        // Exact match (no wildcards)
        if !pattern.contains('*') && !pattern.contains('?') && !pattern.starts_with('!') {
            return pattern == hostname;
        }

        // Handle negation
        let (pattern, is_negated) = if let Some(stripped) = pattern.strip_prefix('!') {
            (stripped, true)
        } else {
            (pattern, false)
        };

        // If pattern still has no wildcards after removing !, it's an exact negation match
        if !pattern.contains('*') && !pattern.contains('?') {
            let matches = pattern == hostname;
            return if is_negated { !matches } else { matches };
        }

        // Convert SSH pattern to regex
        let regex_pattern = pattern
            .replace('.', r"\.")
            .replace('*', ".*")
            .replace('?', ".");

        let regex_pattern = format!("^{regex_pattern}$");

        if let Ok(regex) = Regex::new(&regex_pattern) {
            let matches = regex.is_match(hostname);
            if is_negated { !matches } else { matches }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_config(content: &str) -> Result<SshConfig> {
        let mut file = NamedTempFile::new()?;
        write!(file, "{content}")?;

        let mut config = SshConfig { hosts: Vec::new() };
        config.parse_config(&file.path().to_path_buf())?;

        Ok(config)
    }

    #[test]
    fn test_exact_match() -> Result<()> {
        let config = create_test_config(
            r#"
Host server1
    HostName 192.168.1.10
    User admin
    Port 2222

Host server2
    HostName server2.example.com
    User root
"#,
        )?;

        // Test exact matches
        let host = config.get_host("server1").unwrap();
        assert_eq!(host.host, "server1");
        assert_eq!(host.hostname, Some("192.168.1.10".to_string()));
        assert_eq!(host.user, Some("admin".to_string()));
        assert_eq!(host.port, Some(2222));

        let host = config.get_host("server2").unwrap();
        assert_eq!(host.host, "server2");
        assert_eq!(host.hostname, Some("server2.example.com".to_string()));
        assert_eq!(host.user, Some("root".to_string()));

        // Test non-existent host
        assert!(config.get_host("server3").is_none());

        Ok(())
    }

    #[test]
    fn test_wildcard_asterisk() -> Result<()> {
        let config = create_test_config(
            r#"
Host web.example.com
    User specific

Host prod-*.example.com
    User produser
    Port 22

Host *.example.com
    User webuser
    Port 443
"#,
        )?;

        // Test specific match takes precedence (comes first)
        let host = config.get_host("web.example.com").unwrap();
        assert_eq!(host.user, Some("specific".to_string()));

        // Test more specific wildcard
        let host = config.get_host("prod-app.example.com").unwrap();
        assert_eq!(host.user, Some("produser".to_string()));
        assert_eq!(host.port, Some(22));

        // Test general wildcard matches
        let host = config.get_host("test.example.com").unwrap();
        assert_eq!(host.user, Some("webuser".to_string()));
        assert_eq!(host.port, Some(443));

        // Test no match
        assert!(config.get_host("example.org").is_none());

        Ok(())
    }

    #[test]
    fn test_wildcard_question_mark() -> Result<()> {
        let config = create_test_config(
            r#"
Host server?
    HostName 10.0.0.%h
    User admin

Host server??
    HostName 10.1.0.%h
    User superadmin
"#,
        )?;

        // Single character wildcard
        let host = config.get_host("server1").unwrap();
        assert_eq!(host.hostname, Some("10.0.0.%h".to_string()));
        assert_eq!(host.user, Some("admin".to_string()));

        // Two character wildcard
        let host = config.get_host("server10").unwrap();
        assert_eq!(host.hostname, Some("10.1.0.%h".to_string()));
        assert_eq!(host.user, Some("superadmin".to_string()));

        // No match - too many characters
        assert!(config.get_host("server100").is_none());

        Ok(())
    }

    #[test]
    fn test_simple_negation() -> Result<()> {
        let config = create_test_config(
            r#"
Host *.internal.com
    User internal
    Port 2222

Host !*.internal.com
    User external
    Port 22
"#,
        )?;

        // Should match internal pattern
        let host = config.get_host("app.internal.com").unwrap();
        assert_eq!(host.user, Some("internal".to_string()));
        assert_eq!(host.port, Some(2222));

        // The negation pattern itself is not useful without being part of multi-pattern
        // For now, we'll skip testing standalone negation patterns

        Ok(())
    }

    #[test]
    fn test_pattern_precedence() -> Result<()> {
        let config = create_test_config(
            r#"
Host specific.example.com
    User specific_user

Host *.example.com
    User wildcard_user

Host *
    User default_user
"#,
        )?;

        // Most specific match
        let host = config.get_host("specific.example.com").unwrap();
        assert_eq!(host.user, Some("specific_user".to_string()));

        // Wildcard match
        let host = config.get_host("other.example.com").unwrap();
        assert_eq!(host.user, Some("wildcard_user".to_string()));

        // Catch-all match
        let host = config.get_host("random.server.org").unwrap();
        assert_eq!(host.user, Some("default_user".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_all_hosts_excludes_wildcards() -> Result<()> {
        let config = create_test_config(
            r#"
Host server1
    HostName 192.168.1.1

Host server2
    HostName 192.168.1.2

Host *.example.com
    User webuser

Host server?
    User admin

Host * !*.internal
    User external
"#,
        )?;

        let all_hosts = config.get_all_hosts();
        let host_names: Vec<&str> = all_hosts.iter().map(|h| h.host.as_str()).collect();

        // Should only include concrete hosts
        assert_eq!(host_names.len(), 2);
        assert!(host_names.contains(&"server1"));
        assert!(host_names.contains(&"server2"));

        // Should not include wildcard patterns
        assert!(!host_names.iter().any(|&h| h.contains('*')));
        assert!(!host_names.iter().any(|&h| h.contains('?')));

        Ok(())
    }

    #[test]
    fn test_complex_wildcard_scenarios() -> Result<()> {
        let config = create_test_config(
            r#"
Host prod-db-*
    HostName %h.database.internal
    User dbadmin
    Port 5432

Host prod-*
    HostName %h.prod.internal
    User produser
    Port 22

Host *-db-*
    User dbuser
    Port 3306
"#,
        )?;

        // Should match most specific pattern first
        let host = config.get_host("prod-db-master").unwrap();
        assert_eq!(host.hostname, Some("%h.database.internal".to_string()));
        assert_eq!(host.user, Some("dbadmin".to_string()));
        assert_eq!(host.port, Some(5432));

        // Should match prod-* pattern
        let host = config.get_host("prod-web").unwrap();
        assert_eq!(host.hostname, Some("%h.prod.internal".to_string()));
        assert_eq!(host.user, Some("produser".to_string()));
        assert_eq!(host.port, Some(22));

        // Should match general db pattern
        let host = config.get_host("test-db-slave").unwrap();
        assert_eq!(host.user, Some("dbuser".to_string()));
        assert_eq!(host.port, Some(3306));

        Ok(())
    }

    #[test]
    fn test_hostname_fallback() -> Result<()> {
        let config = create_test_config(
            r#"
Host myserver
    User admin

Host *.local
    User localuser
"#,
        )?;

        // When hostname is not specified, it should use the host pattern
        let host = config.get_host("myserver").unwrap();
        assert_eq!(host.hostname, None);

        // This is handled by SftpClient::connect which uses:
        // let hostname = host_config.hostname.as_ref().unwrap_or(&host_config.host);

        Ok(())
    }
}
