use crate::ssh_config::SshHost;
use anyhow::{Result, anyhow};
use ssh2::{Session, Sftp};
use std::fs;
use std::io::prelude::*;
use std::net::TcpStream;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    #[allow(dead_code)]
    pub size: u64,
    #[allow(dead_code)]
    pub permissions: u32,
}

pub struct SftpClient {
    _session: Session,
    sftp: Sftp,
}

impl SftpClient {
    pub fn connect(host_config: &SshHost) -> Result<Self> {
        let hostname = host_config.hostname.as_ref().unwrap_or(&host_config.host);
        let port = host_config.port.unwrap_or(22);
        let user = host_config
            .user
            .as_ref()
            .ok_or_else(|| anyhow!("No username specified"))?;

        let tcp = TcpStream::connect(format!("{hostname}:{port}"))?;
        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // Try authentication methods
        let auth_result = if let Some(identity_file) = &host_config.identity_file {
            // First try with public key file if it exists
            let pubkey_path = PathBuf::from(format!("{}.pub", identity_file.display()));
            if pubkey_path.exists() {
                session.userauth_pubkey_file(user, Some(&pubkey_path), identity_file, None)
            } else {
                session.userauth_pubkey_file(user, None, identity_file, None)
            }
        } else {
            // No identity file specified, use ssh-agent
            session.userauth_agent(user)
        };

        // If identity file auth failed, try ssh-agent as fallback
        if auth_result.is_err() {
            session.userauth_agent(user)?;
        }

        if !session.authenticated() {
            return Err(anyhow!("Authentication failed"));
        }

        let sftp = session.sftp()?;

        Ok(SftpClient {
            _session: session,
            sftp,
        })
    }

    pub fn list_directory(&self, path: &Path) -> Result<Vec<FileInfo>> {
        let mut files = Vec::new();

        for (path_buf, stat) in self.sftp.readdir(path)? {
            let name = path_buf
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let is_dir = stat.is_dir();
            let size = stat.size.unwrap_or(0);
            let permissions = stat.perm.unwrap_or(0);

            files.push(FileInfo {
                name,
                path: path_buf,
                is_dir,
                size,
                permissions,
            });
        }

        files.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(files)
    }

    pub fn download_file(&self, remote_path: &Path, local_path: &Path) -> Result<()> {
        let mut remote_file = self.sftp.open(remote_path)?;
        let mut local_file = fs::File::create(local_path)?;

        let mut buffer = [0; 8192];
        loop {
            let bytes_read = remote_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            local_file.write_all(&buffer[..bytes_read])?;
        }

        Ok(())
    }

    pub fn upload_file(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        let mut local_file = fs::File::open(local_path)?;
        let mut remote_file = self.sftp.create(remote_path)?;

        let mut buffer = [0; 8192];
        loop {
            let bytes_read = local_file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            remote_file.write_all(&buffer[..bytes_read])?;
        }

        Ok(())
    }

    pub fn create_directory(&self, remote_path: &Path) -> Result<()> {
        self.sftp.mkdir(remote_path, 0o755)?;
        Ok(())
    }

    pub fn upload_directory(&self, local_path: &Path, remote_path: &Path) -> Result<()> {
        // Create the remote directory
        self.create_directory(remote_path)?;

        // Read local directory contents
        let entries = fs::read_dir(local_path)?;

        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let file_name = entry.file_name();
            let local_file_path = entry.path();
            let remote_file_path = remote_path.join(&file_name);

            if file_type.is_dir() {
                // Recursively upload subdirectory
                self.upload_directory(&local_file_path, &remote_file_path)?;
            } else {
                // Upload file
                self.upload_file(&local_file_path, &remote_file_path)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_info_creation() {
        let file_info = FileInfo {
            name: "test.txt".to_string(),
            path: PathBuf::from("/home/user/test.txt"),
            is_dir: false,
            size: 1024,
            permissions: 0o644,
        };

        assert_eq!(file_info.name, "test.txt");
        assert_eq!(file_info.path, PathBuf::from("/home/user/test.txt"));
        assert!(!file_info.is_dir);
        assert_eq!(file_info.size, 1024);
        assert_eq!(file_info.permissions, 0o644);
    }

    #[test]
    fn test_file_info_directory() {
        let dir_info = FileInfo {
            name: "documents".to_string(),
            path: PathBuf::from("/home/user/documents"),
            is_dir: true,
            size: 4096,
            permissions: 0o755,
        };

        assert!(dir_info.is_dir);
        assert_eq!(dir_info.permissions, 0o755);
    }

    #[test]
    fn test_file_info_clone() {
        let original = FileInfo {
            name: "file.rs".to_string(),
            path: PathBuf::from("/project/src/file.rs"),
            is_dir: false,
            size: 2048,
            permissions: 0o644,
        };

        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.path, cloned.path);
        assert_eq!(original.is_dir, cloned.is_dir);
        assert_eq!(original.size, cloned.size);
        assert_eq!(original.permissions, cloned.permissions);
    }
}
