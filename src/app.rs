use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use crate::sftp::{FileInfo, SftpClient};
use crate::ssh_config::{SshConfig, SshHost};
use crate::ui::Ui;

#[derive(Debug, Clone, PartialEq)]
pub enum Pane {
    Local,
    Remote,
}

#[derive(Debug, Clone)]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone)]
pub struct TransferItem {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub direction: TransferDirection,
}

pub struct App {
    pub ssh_config: SshConfig,
    pub sftp_client: Option<SftpClient>,
    pub current_host: Option<String>,
    pub available_hosts: Vec<SshHost>,

    pub active_pane: Pane,
    pub local_path: PathBuf,
    pub remote_path: PathBuf,
    pub local_files: Vec<FileInfo>,
    pub remote_files: Vec<FileInfo>,
    pub local_cursor: usize,
    pub remote_cursor: usize,
    pub local_selected: HashSet<usize>,
    pub remote_selected: HashSet<usize>,

    pub show_connection_dialog: bool,
    pub connection_cursor: usize,
    pub show_transfer_dialog: bool,
    pub transfer_queue: Vec<TransferItem>,

    pub search_mode: bool,
    pub search_query: String,
    pub filtered_local_files: Vec<FileInfo>,
    pub filtered_remote_files: Vec<FileInfo>,

    pub should_quit: bool,
}

impl App {
    pub async fn new(initial_host: Option<String>) -> Result<Self> {
        let ssh_config = SshConfig::new()?;
        let available_hosts = ssh_config.get_all_hosts();

        let local_path = env::current_dir()?;
        let remote_path = PathBuf::from("/");

        let mut app = App {
            ssh_config,
            sftp_client: None,
            current_host: None,
            available_hosts,

            active_pane: Pane::Local,
            local_path,
            remote_path,
            local_files: Vec::new(),
            remote_files: Vec::new(),
            local_cursor: 0,
            remote_cursor: 0,
            local_selected: HashSet::new(),
            remote_selected: HashSet::new(),

            show_connection_dialog: false,
            connection_cursor: 0,
            show_transfer_dialog: false,
            transfer_queue: Vec::new(),

            search_mode: false,
            search_query: String::new(),
            filtered_local_files: Vec::new(),
            filtered_remote_files: Vec::new(),

            should_quit: false,
        };

        app.refresh_local_files()?;

        if let Some(host) = initial_host {
            app.connect_to_host(&host).await?;
        }

        Ok(app)
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut ui = Ui::new()?;

        loop {
            if self.should_quit {
                break;
            }

            ui.draw(self)?;

            if let Some(event) = ui.handle_events()? {
                self.handle_event(event).await?;
            }
        }

        Ok(())
    }

    async fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            if self.show_connection_dialog {
                return self.handle_connection_dialog_event(key.code).await;
            }

            if self.show_transfer_dialog {
                return self.handle_transfer_dialog_event(key.code).await;
            }

            if self.search_mode {
                return self.handle_search_event(key.code).await;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.should_quit = true;
                }
                KeyCode::Tab => {
                    self.active_pane = match self.active_pane {
                        Pane::Local => Pane::Remote,
                        Pane::Remote => Pane::Local,
                    };
                }
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('K') => {
                    self.move_cursor_up();
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.move_cursor_down();
                }
                KeyCode::Enter => {
                    self.change_directory().await?;
                }
                KeyCode::Char(' ') => {
                    self.toggle_selection();
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    self.show_connection_dialog = true;
                }
                KeyCode::Char('t') | KeyCode::Char('T') => {
                    self.prepare_transfer()?;
                }
                KeyCode::Char('/') => {
                    self.start_search();
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_connection_dialog_event(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.show_connection_dialog = false;
            }
            KeyCode::Up => {
                if self.connection_cursor > 0 {
                    self.connection_cursor -= 1;
                }
            }
            KeyCode::Down => {
                if self.connection_cursor < self.available_hosts.len().saturating_sub(1) {
                    self.connection_cursor += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(host) = self.available_hosts.get(self.connection_cursor).cloned() {
                    self.connect_to_host(&host.host).await?;
                    self.show_connection_dialog = false;
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_transfer_dialog_event(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.show_transfer_dialog = false;
                self.transfer_queue.clear();
            }
            KeyCode::Enter => {
                self.execute_transfers().await?;
                self.show_transfer_dialog = false;
            }
            _ => {}
        }

        Ok(())
    }

    async fn connect_to_host(&mut self, host_name: &str) -> Result<()> {
        let host_config = self
            .ssh_config
            .get_host(host_name)
            .unwrap_or_else(|| SshHost {
                host: host_name.to_string(),
                hostname: Some(host_name.to_string()),
                user: None,
                port: None,
                identity_file: None,
            });

        let client = SftpClient::connect(&host_config)?;
        self.sftp_client = Some(client);
        self.current_host = Some(host_name.to_string());
        self.remote_path = PathBuf::from("/");
        self.refresh_remote_files().await?;

        Ok(())
    }

    fn refresh_local_files(&mut self) -> Result<()> {
        self.local_files.clear();

        // Add parent directory entry if not at root
        if let Some(parent) = self.local_path.parent() {
            self.local_files.push(FileInfo {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
                size: 0,
                permissions: 0o755,
            });
        }

        for entry in fs::read_dir(&self.local_path)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = entry.metadata()?;

            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            self.local_files.push(FileInfo {
                name,
                path,
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                permissions: 0o755,
            });
        }

        // Sort with .. always first, then directories, then files
        self.local_files.sort_by(|a, b| {
            if a.name == ".." {
                std::cmp::Ordering::Less
            } else if b.name == ".." {
                std::cmp::Ordering::Greater
            } else {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            }
        });

        self.local_cursor = 0;
        self.local_selected.clear();

        Ok(())
    }

    async fn refresh_remote_files(&mut self) -> Result<()> {
        if let Some(client) = &self.sftp_client {
            self.remote_files = client.list_directory(&self.remote_path)?;

            // Add parent directory entry if not at root
            if self.remote_path != PathBuf::from("/") {
                if let Some(parent) = self.remote_path.parent() {
                    self.remote_files.insert(
                        0,
                        FileInfo {
                            name: "..".to_string(),
                            path: parent.to_path_buf(),
                            is_dir: true,
                            size: 0,
                            permissions: 0o755,
                        },
                    );
                }
            }

            self.remote_cursor = 0;
            self.remote_selected.clear();
        }

        Ok(())
    }

    fn move_cursor_up(&mut self) {
        match self.active_pane {
            Pane::Local => {
                if self.local_cursor > 0 {
                    self.local_cursor -= 1;
                }
            }
            Pane::Remote => {
                if self.remote_cursor > 0 {
                    self.remote_cursor -= 1;
                }
            }
        }
    }

    fn move_cursor_down(&mut self) {
        match self.active_pane {
            Pane::Local => {
                let files_len = self.get_current_local_files().len();
                if self.local_cursor < files_len.saturating_sub(1) {
                    self.local_cursor += 1;
                }
            }
            Pane::Remote => {
                let files_len = self.get_current_remote_files().len();
                if self.remote_cursor < files_len.saturating_sub(1) {
                    self.remote_cursor += 1;
                }
            }
        }
    }

    async fn change_directory(&mut self) -> Result<()> {
        match self.active_pane {
            Pane::Local => {
                let files = self.get_current_local_files();
                if let Some(file) = files.get(self.local_cursor) {
                    if file.is_dir {
                        self.local_path = file.path.clone();
                        self.search_mode = false;
                        self.search_query.clear();
                        self.clear_search_filter();
                        self.refresh_local_files()?;
                    }
                }
            }
            Pane::Remote => {
                let files = self.get_current_remote_files();
                if let Some(file) = files.get(self.remote_cursor) {
                    if file.is_dir {
                        self.remote_path = file.path.clone();
                        self.search_mode = false;
                        self.search_query.clear();
                        self.clear_search_filter();
                        self.refresh_remote_files().await?;
                    }
                }
            }
        }

        Ok(())
    }

    fn toggle_selection(&mut self) {
        match self.active_pane {
            Pane::Local => {
                if self.local_selected.contains(&self.local_cursor) {
                    self.local_selected.remove(&self.local_cursor);
                } else {
                    self.local_selected.insert(self.local_cursor);
                }
            }
            Pane::Remote => {
                if self.remote_selected.contains(&self.remote_cursor) {
                    self.remote_selected.remove(&self.remote_cursor);
                } else {
                    self.remote_selected.insert(self.remote_cursor);
                }
            }
        }
    }

    fn prepare_transfer(&mut self) -> Result<()> {
        self.transfer_queue.clear();

        for &index in &self.local_selected {
            if let Some(file) = self.local_files.get(index) {
                let destination = self.remote_path.join(&file.name);
                self.transfer_queue.push(TransferItem {
                    source: file.path.clone(),
                    destination,
                    direction: TransferDirection::Upload,
                });
            }
        }

        for &index in &self.remote_selected {
            if let Some(file) = self.remote_files.get(index) {
                let destination = self.local_path.join(&file.name);
                self.transfer_queue.push(TransferItem {
                    source: file.path.clone(),
                    destination,
                    direction: TransferDirection::Download,
                });
            }
        }

        if !self.transfer_queue.is_empty() {
            self.show_transfer_dialog = true;
        }

        Ok(())
    }

    async fn execute_transfers(&mut self) -> Result<()> {
        if let Some(client) = &self.sftp_client {
            for item in &self.transfer_queue {
                match item.direction {
                    TransferDirection::Upload => {
                        // Check if source is a directory
                        if item.source.is_dir() {
                            client.upload_directory(&item.source, &item.destination)?;
                        } else {
                            client.upload_file(&item.source, &item.destination)?;
                        }
                    }
                    TransferDirection::Download => {
                        client.download_file(&item.source, &item.destination)?;
                    }
                }
            }
        }

        self.transfer_queue.clear();
        self.local_selected.clear();
        self.remote_selected.clear();

        self.refresh_local_files()?;
        self.refresh_remote_files().await?;

        Ok(())
    }

    async fn handle_search_event(&mut self, key: KeyCode) -> Result<()> {
        match key {
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_query.clear();
                self.clear_search_filter();
            }
            KeyCode::Enter => {
                self.search_mode = false;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search_filter();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.update_search_filter();
            }
            _ => {}
        }

        Ok(())
    }

    fn start_search(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
        self.clear_search_filter();
        self.local_cursor = 0;
        self.remote_cursor = 0;
    }

    fn update_search_filter(&mut self) {
        if self.search_query.is_empty() {
            self.clear_search_filter();
            return;
        }

        let query = self.search_query.to_lowercase();

        // Filter local files
        self.filtered_local_files = self
            .local_files
            .iter()
            .filter(|file| file.name.to_lowercase().contains(&query))
            .cloned()
            .collect();

        // Filter remote files
        self.filtered_remote_files = self
            .remote_files
            .iter()
            .filter(|file| file.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
    }

    fn clear_search_filter(&mut self) {
        self.filtered_local_files.clear();
        self.filtered_remote_files.clear();
    }

    pub fn get_current_local_files(&self) -> &[FileInfo] {
        if self.search_mode && !self.search_query.is_empty() {
            &self.filtered_local_files
        } else {
            &self.local_files
        }
    }

    pub fn get_current_remote_files(&self) -> &[FileInfo] {
        if self.search_mode && !self.search_query.is_empty() {
            &self.filtered_remote_files
        } else {
            &self.remote_files
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_toggle() {
        let pane = Pane::Local;
        assert_eq!(pane, Pane::Local);

        let pane = Pane::Remote;
        assert_eq!(pane, Pane::Remote);
    }

    #[test]
    fn test_transfer_item_upload() {
        let item = TransferItem {
            source: PathBuf::from("/source/file.txt"),
            destination: PathBuf::from("/dest/file.txt"),
            direction: TransferDirection::Upload,
        };

        assert_eq!(item.source, PathBuf::from("/source/file.txt"));
        assert_eq!(item.destination, PathBuf::from("/dest/file.txt"));
        assert!(matches!(item.direction, TransferDirection::Upload));
    }

    #[test]
    fn test_transfer_item_download() {
        let item = TransferItem {
            source: PathBuf::from("/remote/file.txt"),
            destination: PathBuf::from("/local/file.txt"),
            direction: TransferDirection::Download,
        };

        assert!(matches!(item.direction, TransferDirection::Download));
    }

    #[test]
    fn test_transfer_direction_clone() {
        let upload = TransferDirection::Upload;
        let cloned = upload.clone();
        assert!(matches!(cloned, TransferDirection::Upload));
    }
}
