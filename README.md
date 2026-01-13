# UVCAD - CAD File Synchronization Tool

A GrabCAD Workbench-style application for synchronizing CAD files between local storage, Google Drive, and Samba shares. Built with Rust and Tauri for cross-platform support (Windows & macOS).

## Features

- **Cross-Platform**: Runs on Windows and macOS with native installers
- **Multiple Storage Providers**:
  - Local filesystem
  - Google Drive (with OAuth 2.0 authentication)
  - Samba/SMB shares
- **Intelligent Three-Way Sync**: Smart detection of changes with last known state tracking
- **Bidirectional Sync**: Files can be synced in both directions
- **Conflict Detection**: Automatic detection of file conflicts with user-driven resolution
- **File Integrity**: SHA-256 hashing ensures file integrity during transfer
- **Manual Sync**: User-triggered synchronization for full control
- **Real-Time Progress**: Live progress bar showing current file, percentage, and file count
- **Deletion Safety**: Automatic protection against accidental large-scale deletions
- **Credentials Import**: Easy Google Drive setup with JSON file import
- **File State Tracking**: View recently synced files with status and timestamps
- **Professional UI**: Clean, modern interface with status indicators and progress tracking

## Architecture

The application follows a layered architecture:

```
Frontend (React/TypeScript)
    ↕ Tauri IPC
Backend (Rust)
    ├── Commands Layer (Tauri commands)
    ├── Core Layer (Sync Engine, Auth Manager, Conflict Resolver)
    ├── Storage Layer (File State Tracker, Metadata Cache)
    └── Providers Layer (Local FS, Google Drive, Samba)
```

## Prerequisites

- **Node.js** (v18 or higher) and npm
- **Rust** (latest stable version)
- **Cargo** (comes with Rust)

## Installation

### For End Users

Download and install the pre-built installer for your platform:
- **macOS**: `UVCAD_0.1.0_universal.dmg`
- **Windows**: `UVCAD_0.1.0_x64_en-US.msi`

📖 See **[INSTALL.md](INSTALL.md)** for detailed installation instructions.

### For Developers

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd UVCAD
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

3. Run in development mode:
   ```bash
   npm run dev
   ```

## Building Installers

### Option 1: Automated Builds (Recommended)

Use GitHub Actions to build installers for both platforms automatically:

```bash
# Create and push a version tag
git tag v0.1.0
git push origin v0.1.0

# GitHub Actions will automatically:
# 1. Build Windows and macOS installers
# 2. Create a GitHub Release
# 3. Attach all installers
```

📖 See **[.github/workflows/README.md](.github/workflows/README.md)** for GitHub Actions documentation.

**Manual trigger**: Go to Actions → Manual Build → Run workflow

### Option 2: Local Build

```bash
# Build for current platform
npm run tauri:build

# Build for macOS (on Mac)
npm run build:mac

# Build for Windows (on Windows)
npm run build:windows
```

📖 See **[BUILD.md](BUILD.md)** for detailed build instructions and troubleshooting.
📖 See **[BUILD_WINDOWS.md](BUILD_WINDOWS.md)** for Windows-specific instructions.
📖 See **[QUICK_BUILD.md](QUICK_BUILD.md)** for a quick reference.

### Build Output Locations
- **macOS**: `src-tauri/target/release/bundle/dmg/UVCAD_0.1.0_aarch64.dmg`
- **Windows**: `src-tauri/target/release/bundle/msi/UVCAD_0.1.0_x64_en-US.msi`

## Project Structure

```
uvcad/
├── src/                         # React frontend
│   ├── components/              # React components
│   ├── hooks/                   # Custom React hooks
│   ├── services/                # API services
│   ├── types/                   # TypeScript types
│   └── styles/                  # CSS styles
│
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── commands/            # Tauri command handlers
│   │   ├── core/                # Business logic
│   │   │   ├── sync_engine.rs   # Main sync orchestration
│   │   │   ├── conflict_resolver.rs
│   │   │   ├── file_hasher.rs
│   │   │   └── auth_manager.rs
│   │   ├── providers/           # Storage providers
│   │   │   ├── traits.rs        # StorageProvider trait
│   │   │   ├── local_fs.rs
│   │   │   ├── google_drive.rs
│   │   │   └── samba.rs
│   │   ├── db/                  # Database layer (SQLite)
│   │   ├── models/              # Domain models
│   │   └── utils/               # Utilities
│   └── Cargo.toml
│
└── package.json
```

## Usage

### First-Time Setup

1. Launch the application
2. Click "Settings" in the top right
3. Configure your sync locations:
   - **Local Folder**: Select the folder containing your CAD files
   - **Google Drive**: Click "Connect" to authenticate and enter your Google Drive folder ID
   - **Samba Share**: Enter the path to your SMB share (e.g., `\\server\share` on Windows or `/Volumes/share` on macOS)
4. Click "Save Configuration"

### Syncing Files

1. Click the "Start Sync" button on the main screen
2. The app will scan all configured locations
3. If conflicts are detected, you'll be prompted to resolve them
4. Files will be synchronized according to your configuration

### Conflict Resolution

When the same file is modified in multiple locations, you'll be presented with options:
- **Keep Local Version**: Use the file from your local folder
- **Keep Google Drive Version**: Use the file from Google Drive
- **Keep Samba Version**: Use the file from the Samba share
- **Keep All**: Rename files to keep all versions

## Implementation Status

### ✅ Completed
- Project structure and build system
- Database schema (SQLite)
- Error handling framework
- Storage Provider trait and abstractions
- Local filesystem provider (fully functional)
- Samba provider (basic implementation)
- File hashing (SHA-256)
- React frontend with basic UI
- Tauri command handlers
- **Google Drive Integration** (FULLY IMPLEMENTED ✨)
  - OAuth 2.0 with PKCE flow
  - Local callback server (port 8080)
  - Automatic token refresh
  - Secure token storage (OS keyring)
  - File listing with pagination
  - File upload (new & update)
  - File download with verification
  - File deletion
  - Metadata retrieval
  - Connection testing
- **Three-Way Sync Engine** (FULLY IMPLEMENTED ✨)
  - Intelligent sync direction detection
  - Hash-based change detection
  - Last known state tracking in database
  - Only syncs changed files (no redundant transfers)
  - Conflict detection with detailed reporting
- **Progress Tracking** (FULLY IMPLEMENTED ✨)
  - Real-time progress bar for sync operations
  - File-by-file progress updates
  - Percentage and file count display
  - Event-driven updates via Tauri
- **Deletion Safety Protection** (FULLY IMPLEMENTED ✨)
  - Automatic blocking of large deletions (>50 files or >30%)
  - Prevents data loss from unmounted drives
  - Detailed error messages by location
  - Settings panel documentation
- **Professional Installers** (FULLY IMPLEMENTED ✨)
  - Universal macOS DMG (Intel + Apple Silicon)
  - Windows MSI installer
  - App icons for all platforms
  - Comprehensive build documentation
- **File State Tracking** (FULLY IMPLEMENTED ✨)
  - Database persistence of file states
  - Displays recently synced files in GUI
  - Shows sync status (synced/pending/conflict)
  - Sorted by modification date

### 🚧 Partial / TODO
- **Conflict Resolution UI**: Basic detection implemented, needs user interface with side-by-side comparison
- **Error Recovery**: Basic error handling, needs retry logic with exponential backoff
- **Unit Tests**: Core functionality works, needs expanded test coverage
- **Resumable Uploads**: Works for normal files, could add resume support for very large CAD files (>100MB)
- **Batch Operations**: Sync works efficiently, could optimize for very large file sets
- **Auto-Update**: Installers created, could add Tauri auto-updater for seamless updates

## Google Drive Setup

**See [GOOGLE_DRIVE_INTEGRATION.md](GOOGLE_DRIVE_INTEGRATION.md) for detailed instructions.**

Quick Setup:
1. Create a project in [Google Cloud Console](https://console.cloud.google.com/)
2. Enable the Google Drive API
3. Create OAuth 2.0 credentials (Desktop app)
4. Add redirect URI: `http://127.0.0.1:8080/oauth/callback`
5. Enter Client ID and Secret in UVCAD Settings
6. Click "Connect to Google Drive" and authorize

## Samba/SMB Setup

### macOS
1. Connect to your SMB share via Finder: `⌘K` → `smb://server/share`
2. The share will be mounted under `/Volumes/share_name`
3. Use the mount path in UVCAD settings

### Windows
1. Map a network drive to your SMB share
2. Use the UNC path (`\\server\share`) or mapped drive letter in UVCAD settings

## Contributing

This is a foundational implementation. Areas for contribution:
- Complete Google Drive API integration
- Enhance conflict resolution UI
- Add file versioning support
- Implement selective sync (exclude patterns)
- Add CAD file thumbnail previews
- Performance optimizations
- Additional tests

## Architecture Notes

### Sync Strategy
The application uses a three-way sync model:
1. Scan all three locations (Local, Google Drive, Samba)
2. Compare file hashes to detect changes
3. Determine sync direction or detect conflicts
4. Execute file transfers with integrity verification

### Security
- OAuth tokens stored in OS keyring (Keychain on macOS, Credential Manager on Windows)
- File hashes verified after every transfer
- HTTPS for all Google Drive communication

## License

MIT

## Acknowledgments

Inspired by GrabCAD Workbench's approach to CAD file management and collaboration.
