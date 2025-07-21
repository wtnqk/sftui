# sftui - Terminal UI SFTP Client

[![Crates.io](https://img.shields.io/crates/v/sftui.svg)](https://crates.io/crates/sftui)
[![Documentation](https://docs.rs/sftui/badge.svg)](https://docs.rs/sftui)
[![CI](https://github.com/wtnqk/sftui/workflows/CI/badge.svg)](https://github.com/wtnqk/sftui/actions)
[![License](https://img.shields.io/crates/l/sftui.svg)](LICENSE)

A terminal-based SFTP client with dual panes for local and remote file browsing, built with Rust and Ratatui.

## Features

- **Dual Pane Interface**: Side-by-side local and remote file browsing
- **SSH Config Integration**: Automatically imports connection settings from `~/.ssh/config`
- **Runtime Connection Switching**: Change SFTP destinations without restarting
- **File Selection & Staging**: Select multiple files/directories for transfer
- **Confirmation Dialogs**: Review transfers before execution
- **Directory Navigation**: Browse local and remote directories

## Platform Support

This application has been tested on macOS. It should also work on Linux and Windows if you can successfully build it, as it uses cross-platform Rust libraries.

## Requirements

- **SSH Agent** (Strongly Recommended): For SSH key authentication, you must have ssh-agent running:

  ```bash
  # Start ssh-agent
  eval "$(ssh-agent -s)"

  # Add your SSH key
  ssh-add ~/.ssh/id_rsa
  ```

  Without ssh-agent, only password authentication or unencrypted SSH keys will work.

## Installation

### From crates.io

```bash
cargo install sftui
```

### From source

```bash
git clone https://github.com/wtnqk/sftui.git
cd sftui
cargo install --path .
```

## Usage

### Basic Navigation

- **Tab**: Switch between local and remote panes
- **↑/↓** or **j/k**: Navigate file list (vim-style navigation supported)
- **Enter**: Enter directory (when on a directory)
- **Space**: Select/deselect files for transfer
- **q** or **Q**: Quit application

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

- **c** or **C**: Open connection dialog to switch SFTP destinations
- The application reads SSH hosts from `~/.ssh/config`
- You can specify a host at startup: `sftui -H hostname`
- In connection dialog:
  - **↑/↓**: Navigate host list
  - **Enter**: Connect to selected host
  - **Esc**: Cancel

### File Transfers

- **Space**: Select/deselect individual files
- **t** or **T**: Open transfer dialog with selected files
- Selected files appear with blue background
- In transfer dialog:
  - **Enter**: Confirm and execute transfers
  - **Esc**: Cancel transfers

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

| Key        | Action                            |
| ---------- | --------------------------------- |
| Tab        | Switch panes                      |
| ↑/↓ or j/k | Navigate up/down                  |
| Enter      | Enter directory                   |
| Space      | Select/deselect                   |
| /          | Start search                      |
| t or T     | Transfer dialog                   |
| c or C     | Connection dialog                 |
| q or Q     | Quit                              |
| Esc        | Cancel dialog/search              |
| Backspace  | Delete character (in search mode) |

## Transfer Workflow

1. Navigate to desired directories in both panes
2. Select files/directories using Space
3. Press 't' or 'T' to open transfer dialog
4. Review the transfer queue (arrows indicate direction)
5. Press Enter to confirm or Esc to cancel

## Building from Source

### Requirements

- Rust 1.70 or higher
- OpenSSL development libraries (required by the ssh2 crate)

The specific installation method for OpenSSL depends on your platform:
- **Ubuntu/Debian**: `sudo apt-get install libssl-dev pkg-config`
- **macOS**: `brew install openssl@3`
- **Windows**: Install OpenSSL via vcpkg or from https://slproweb.com/products/Win32OpenSSL.html

## License

This project is licensed under the MIT License - see the LICENSE file for details.

