# SFTUI - A Terminal UI SFTP Client

A terminal-based SFTP client with dual panes for local and remote file browsing, built with Rust and Ratatui.

## Features

- **Dual Pane Interface**: Side-by-side local and remote file browsing
- **SSH Config Integration**: Automatically imports connection settings from `~/.ssh/config`
- **Runtime Connection Switching**: Change SFTP destinations without restarting
- **File Selection & Staging**: Select multiple files/directories for transfer
- **Confirmation Dialogs**: Review transfers before execution
- **Directory Navigation**: Browse local and remote directories

## Requirements

- **Nerd Font**: The application uses Nerd Font icons for the best visual experience. Install any Nerd Font (e.g., JetBrainsMono Nerd Font, FiraCode Nerd Font) and configure your terminal to use it.

## Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd sftui
   ```

2. Build the application:
   ```bash
   cargo build --release
   ```

3. Run the application:
   ```bash
   ./target/release/sftui
   ```

## Usage

### Basic Navigation

- **Tab**: Switch between local and remote panes
- **↑/↓**: Navigate file list
- **Enter**: Change directory (when on a directory)
- **Space**: Select/deselect files for transfer
- **q**: Quit application

### Search Function

- **/** : Start search mode
- **Type**: Enter search query (case-insensitive)
- **Backspace**: Delete characters from search
- **Enter**: Exit search mode (keep filtered results)
- **Esc**: Cancel search and return to full listing
- **Real-time filtering**: Results update as you type

### Directory Navigation

- **..** entry appears at the top of directory listings (except at root)
- Navigate up one level by selecting **..** and pressing **Enter**
- Standard Unix-style directory navigation

### Connection Management

- **c**: Open connection dialog to switch SFTP destinations
- The application reads SSH hosts from `~/.ssh/config`
- You can specify a host at startup: `sftui -H hostname`

### File Transfers

- **t**: Open transfer dialog with staged files
- **Enter** (in transfer dialog): Confirm and execute transfers
- **Esc** (in transfer dialog): Cancel transfers

### File Types

-  Directories
-  Files
-  Parent directory (..)
-  Upload transfer
-  Download transfer
- Blue highlighting indicates selected items
- Green border shows the active pane

## SSH Configuration

The application reads SSH configuration from `~/.ssh/config`. Example configuration:

```
Host myserver
    HostName example.com
    User username
    Port 22
    IdentityFile ~/.ssh/id_rsa

Host another-server
    HostName 192.168.1.100
    User admin
    Port 2222
```

## Dependencies

- **crossterm**: Terminal handling
- **ratatui**: Terminal UI framework
- **ssh2**: SSH/SFTP protocol implementation
- **tokio**: Async runtime
- **anyhow**: Error handling
- **clap**: Command line argument parsing
- **dirs**: Directory utilities

## Key Bindings Summary

| Key | Action |
|-----|--------|
| Tab | Switch panes |
| ↑/↓ | Navigate |
| Enter | Change directory |
| Space | Select/deselect |
| / | Start search |
| t | Transfer files |
| c | Change connection |
| q | Quit |
| Esc | Cancel dialog/search |

## Transfer Workflow

1. Navigate to desired directories in both panes
2. Select files/directories using Space
3. Press 't' to open transfer dialog
4. Review the transfer queue (↑ = upload, ↓ = download)
5. Press Enter to confirm or Esc to cancel