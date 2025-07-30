use anyhow::{Result, anyhow};
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
    entries: Vec<SshConfigEntry>,
}

impl SshConfig {
    pub fn new() -> Result<Self> {
        let config_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot find home directory"))?
            .join(".ssh")
            .join("config");

        let mut ssh_config = SshConfig {
            entries: Vec::new(),
        };

        if config_path.exists() {
            ssh_config.parse_config(&config_path)?;
        }

        Ok(ssh_config)
    }

    fn parse_config(&mut self, config_path: &PathBuf) -> Result<()> {
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
                        self.entries.push(entry);
                    }
                    let patterns: Vec<String> = value
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
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
                    if let Some(ref mut entry) = current_entry {
                        if let Ok(port) = value.parse::<u16>() {
                            entry.port = Some(port);
                        }
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
            self.entries.push(entry);
        }

        Ok(())
    }

    fn matches_pattern(pattern: &str, hostname: &str) -> bool {
        // 完全一致の場合
        if !pattern.contains('*') {
            return pattern == hostname;
        }
        
        // パターンを*で分割
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                // 空のパート（連続する*または先頭/末尾の*）はスキップ
                continue;
            }
            
            // 最初のパートかつパターンが*で始まらない場合
            if i == 0 && !pattern.starts_with('*') {
                if !hostname.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } 
            // 最後のパートかつパターンが*で終わらない場合
            else if i == parts.len() - 1 && !pattern.ends_with('*') {
                if !hostname[pos..].ends_with(part) {
                    return false;
                }
            }
            // 中間のパート
            else {
                match hostname[pos..].find(part) {
                    Some(found) => pos += found + part.len(),
                    None => return false,
                }
            }
        }
        
        true
    }

    pub fn get_host(&self, name: &str) -> Option<SshHost> {
        let mut result = SshHost {
            host: name.to_string(),
            hostname: None,
            user: None,
            port: None,
            identity_file: None,
        };
        
        let mut found_match = false;
        
        // すべてのエントリを確認し、マッチするものから設定を累積的に適用
        for entry in &self.entries {
            for pattern in &entry.patterns {
                if Self::matches_pattern(pattern, name) {
                    found_match = true;
                    
                    // 各フィールドについて、まだ設定されていない場合のみ適用
                    if result.hostname.is_none() && entry.hostname.is_some() {
                        result.hostname = entry.hostname.clone();
                    }
                    if result.user.is_none() && entry.user.is_some() {
                        result.user = entry.user.clone();
                    }
                    if result.port.is_none() && entry.port.is_some() {
                        result.port = entry.port;
                    }
                    if result.identity_file.is_none() && entry.identity_file.is_some() {
                        result.identity_file = entry.identity_file.clone();
                    }
                }
            }
        }
        
        if found_match {
            Some(result)
        } else {
            None
        }
    }

    pub fn get_all_hosts(&self) -> Vec<SshHost> {
        let mut hosts = Vec::new();
        
        for entry in &self.entries {
            for pattern in &entry.patterns {
                // ワイルドカードを含まないパターンのみを返す
                if !pattern.contains('*') {
                    hosts.push(SshHost {
                        host: pattern.clone(),
                        hostname: entry.hostname.clone(),
                        user: entry.user.clone(),
                        port: entry.port,
                        identity_file: entry.identity_file.clone(),
                    });
                }
            }
        }
        
        hosts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_matching() {
        // 完全一致のテスト
        assert!(SshConfig::matches_pattern("example.com", "example.com"));
        assert!(!SshConfig::matches_pattern("example.com", "test.com"));
        
        // * ワイルドカードのテスト
        assert!(SshConfig::matches_pattern("*.example.com", "server.example.com"));
        assert!(SshConfig::matches_pattern("*.example.com", "test.example.com"));
        assert!(!SshConfig::matches_pattern("*.example.com", "example.com"));
        
        assert!(SshConfig::matches_pattern("server-*", "server-01"));
        assert!(SshConfig::matches_pattern("server-*", "server-prod"));
        assert!(!SshConfig::matches_pattern("server-*", "server"));
        
        assert!(SshConfig::matches_pattern("*-server", "prod-server"));
        assert!(SshConfig::matches_pattern("*-server", "dev-server"));
        assert!(!SshConfig::matches_pattern("*-server", "server"));
        
        // 複数のワイルドカード
        assert!(SshConfig::matches_pattern("*.*", "example.com"));
        assert!(SshConfig::matches_pattern("192.168.*.*", "192.168.1.100"));
        
        // マルチレイヤなワイルドカード
        // *.*.example.com は最低2つのドットがexample.comの前に必要
        assert!(SshConfig::matches_pattern("*.*.example.com", "dev.api.example.com"));
        assert!(SshConfig::matches_pattern("*.*.example.com", "prod.web.example.com"));
        assert!(!SshConfig::matches_pattern("*.*.example.com", "api.example.com")); // 1つのドットしかない
        assert!(!SshConfig::matches_pattern("*.*.example.com", "example.com")); // ドットがない
        
        // 複雑なパターン
        assert!(SshConfig::matches_pattern("dev-*-*.example.com", "dev-api-v1.example.com"));
        assert!(SshConfig::matches_pattern("dev-*-*.example.com", "dev-web-prod.example.com"));
        assert!(!SshConfig::matches_pattern("dev-*-*.example.com", "dev-api.example.com")); // ハイフンが1つしかない
    }

    #[test] 
    #[ignore] // ? と ! は未実装
    fn test_question_mark_and_negation_patterns() {
        // ? ワイルドカードのテスト（1文字にマッチ）
        assert!(SshConfig::matches_pattern("192.168.0.?", "192.168.0.1"));
        assert!(SshConfig::matches_pattern("192.168.0.?", "192.168.0.9"));
        assert!(!SshConfig::matches_pattern("192.168.0.?", "192.168.0.10")); // 2文字
        assert!(!SshConfig::matches_pattern("192.168.0.?", "192.168.0.")); // 0文字
        
        assert!(SshConfig::matches_pattern("server-?", "server-1"));
        assert!(SshConfig::matches_pattern("server-?", "server-a"));
        assert!(!SshConfig::matches_pattern("server-?", "server-10"));
        
        // ! 否定パターンのテスト
        // 注: 否定パターンは通常、複数のパターンと組み合わせて使用される
        // 例: "Host * !*.local" は .local 以外のすべてのホストにマッチ
    }

    #[test]
    fn test_get_host_with_patterns() {
        let config = SshConfig {
            entries: vec![
                SshConfigEntry {
                    patterns: vec!["*.example.com".to_string()],
                    hostname: Some("bastion.example.com".to_string()),
                    user: Some("admin".to_string()),
                    port: Some(2222),
                    identity_file: None,
                },
                SshConfigEntry {
                    patterns: vec!["specific-host".to_string()],
                    hostname: Some("192.168.1.100".to_string()),
                    user: Some("user".to_string()),
                    port: None,
                    identity_file: None,
                },
            ],
        };

        // ワイルドカードマッチ
        let host = config.get_host("test.example.com").unwrap();
        assert_eq!(host.host, "test.example.com");
        assert_eq!(host.hostname, Some("bastion.example.com".to_string()));
        assert_eq!(host.user, Some("admin".to_string()));
        assert_eq!(host.port, Some(2222));

        // 完全一致
        let host = config.get_host("specific-host").unwrap();
        assert_eq!(host.host, "specific-host");
        assert_eq!(host.hostname, Some("192.168.1.100".to_string()));
        
        // マッチしない
        assert!(config.get_host("no-match.org").is_none());
    }

    #[test]
    fn test_get_all_hosts_excludes_wildcards() {
        let config = SshConfig {
            entries: vec![
                SshConfigEntry {
                    patterns: vec!["*.example.com".to_string()],
                    hostname: Some("bastion.example.com".to_string()),
                    user: Some("admin".to_string()),
                    port: None,
                    identity_file: None,
                },
                SshConfigEntry {
                    patterns: vec!["host1".to_string(), "host2".to_string()],
                    hostname: Some("192.168.1.1".to_string()),
                    user: None,
                    port: None,
                    identity_file: None,
                },
                SshConfigEntry {
                    patterns: vec!["server-*".to_string()],
                    hostname: None,
                    user: Some("deploy".to_string()),
                    port: None,
                    identity_file: None,
                },
            ],
        };

        let hosts = config.get_all_hosts();
        // ワイルドカードを含まないホストのみ返される
        assert_eq!(hosts.len(), 2);
        assert!(hosts.iter().any(|h| h.host == "host1"));
        assert!(hosts.iter().any(|h| h.host == "host2"));
        assert!(!hosts.iter().any(|h| h.host.contains('*')));
    }

    #[test]
    fn test_first_value_wins() {
        let config = SshConfig {
            entries: vec![
                SshConfigEntry {
                    patterns: vec!["*.example.com".to_string()],
                    hostname: Some("bastion1.example.com".to_string()),
                    user: Some("user1".to_string()),
                    port: Some(2222),
                    identity_file: None,
                },
                SshConfigEntry {
                    patterns: vec!["test.example.com".to_string()],
                    hostname: Some("specific.example.com".to_string()),
                    user: Some("user2".to_string()),
                    port: None,
                    identity_file: Some(PathBuf::from("~/.ssh/specific_key")),
                },
            ],
        };

        // 各パラメータについて最初に見つかった値が使用される
        let host = config.get_host("test.example.com").unwrap();
        assert_eq!(host.hostname, Some("bastion1.example.com".to_string())); // 最初のエントリから
        assert_eq!(host.user, Some("user1".to_string())); // 最初のエントリから
        assert_eq!(host.port, Some(2222)); // 最初のエントリから
        assert_eq!(host.identity_file, Some(PathBuf::from("~/.ssh/specific_key"))); // 2番目のエントリから（最初のエントリには設定なし）
    }

    #[test]
    fn test_cumulative_config_application() {
        // SSH configの累積的な適用をテスト
        let config = SshConfig {
            entries: vec![
                SshConfigEntry {
                    patterns: vec!["stg_*".to_string()],
                    hostname: None,
                    user: Some("stg-user".to_string()),
                    port: Some(2222),
                    identity_file: Some(PathBuf::from("~/.ssh/stg_key")),
                },
                SshConfigEntry {
                    patterns: vec!["stg_server1".to_string()],
                    hostname: Some("192.168.1.101".to_string()),
                    user: None,
                    port: None,
                    identity_file: None,
                },
                SshConfigEntry {
                    patterns: vec!["stg_cert".to_string()],
                    hostname: Some("192.168.1.102".to_string()),
                    user: None,
                    port: None,
                    identity_file: Some(PathBuf::from("~/.ssh/stg_special_key")),
                },
            ],
        };

        // stg_server1: stg_*の設定とstg_server1の設定が合成される
        let host = config.get_host("stg_server1").unwrap();
        assert_eq!(host.hostname, Some("192.168.1.101".to_string())); // stg_server1から
        assert_eq!(host.user, Some("stg-user".to_string())); // stg_*から継承
        assert_eq!(host.port, Some(2222)); // stg_*から継承
        assert_eq!(host.identity_file, Some(PathBuf::from("~/.ssh/stg_key"))); // stg_*から継承

        // stg_cert: 最初に設定されたidentity_fileが使われる（上書きされない）
        let host = config.get_host("stg_cert").unwrap();
        assert_eq!(host.hostname, Some("192.168.1.102".to_string())); // stg_certから
        assert_eq!(host.user, Some("stg-user".to_string())); // stg_*から
        assert_eq!(host.port, Some(2222)); // stg_*から
        assert_eq!(host.identity_file, Some(PathBuf::from("~/.ssh/stg_key"))); // stg_*から（最初の値が優先）

        // stg_database: stg_*の設定のみ適用
        let host = config.get_host("stg_database").unwrap();
        assert_eq!(host.hostname, None); // hostnameは設定されていない
        assert_eq!(host.user, Some("stg-user".to_string()));
        assert_eq!(host.port, Some(2222));
        assert_eq!(host.identity_file, Some(PathBuf::from("~/.ssh/stg_key")));
    }

    #[test]
    fn test_specific_before_wildcard() {
        // 具体的なホスト名を先に定義した場合
        let config = SshConfig {
            entries: vec![
                SshConfigEntry {
                    patterns: vec!["stg_server1".to_string(), "stg_server2".to_string()],
                    hostname: Some("192.168.1.100".to_string()),
                    user: Some("admin".to_string()),
                    port: None,
                    identity_file: None,
                },
                SshConfigEntry {
                    patterns: vec!["stg_*".to_string()],
                    hostname: Some("staging-gateway.example.com".to_string()),
                    user: Some("stg-user".to_string()),
                    port: Some(2222),
                    identity_file: None,
                },
            ],
        };

        // 具体的なホスト名が先にマッチする
        let host = config.get_host("stg_server1").unwrap();
        assert_eq!(host.hostname, Some("192.168.1.100".to_string()));
        assert_eq!(host.user, Some("admin".to_string()));

        // ワイルドカードは他のホストにマッチ
        let host = config.get_host("stg_database").unwrap();
        assert_eq!(host.hostname, Some("staging-gateway.example.com".to_string()));
        assert_eq!(host.port, Some(2222));
    }
}
