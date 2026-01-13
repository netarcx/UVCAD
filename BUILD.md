# Building UVCAD Installers

This guide explains how to build UVCAD installers for Windows and macOS.

## Prerequisites

### All Platforms
- **Node.js** (v18 or later): [Download here](https://nodejs.org/)
- **Rust** (latest stable): [Download here](https://rustup.rs/)
- **Git**: For cloning the repository

### macOS Specific
- **Xcode Command Line Tools**: Install with `xcode-select --install`
- For code signing (optional): Apple Developer account

### Windows Specific
- **WebView2**: Usually pre-installed on Windows 10/11
- **Visual Studio Build Tools**: [Download here](https://visualstudio.microsoft.com/downloads/)
  - Select "Desktop development with C++" workload
- For code signing (optional): Code signing certificate

## Setup

1. **Clone the repository** (if not already done):
   ```bash
   git clone <repository-url>
   cd UVCAD
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Verify Rust installation**:
   ```bash
   rustc --version
   cargo --version
   ```

## Building Installers

### macOS (on Mac)

To build a universal macOS app bundle (.app) and DMG installer:

```bash
npm run build:mac
```

**Output location**: `src-tauri/target/release/bundle/`

**Artifacts created**:
- **UVCAD.app** - The application bundle (drag to Applications folder)
- **dmg/UVCAD_0.1.0_universal.dmg** - Disk image installer (recommended for distribution)

**Installation for end users**:
1. Download the `.dmg` file
2. Double-click to open
3. Drag UVCAD.app to Applications folder
4. Launch from Applications

**Note**: First launch may show "unidentified developer" warning. Users can:
- Right-click → Open → Click "Open" button, OR
- System Preferences → Security & Privacy → Click "Open Anyway"

To avoid this warning, you need to code sign the app (requires Apple Developer account).

### Windows (on Windows)

To build a Windows installer:

```bash
npm run build:windows
```

**Output location**: `src-tauri/target/release/bundle/`

**Artifacts created**:
- **msi/UVCAD_0.1.0_x64_en-US.msi** - Windows Installer package (recommended)
- **nsis/UVCAD_0.1.0_x64-setup.exe** - NSIS installer (alternative)

**Installation for end users**:
1. Download the `.msi` or `.exe` file
2. Double-click to run the installer
3. Follow the installation wizard
4. Launch from Start Menu or Desktop shortcut

**Note**: Windows may show "Windows protected your PC" warning for unsigned executables. Users can click "More info" → "Run anyway"

To avoid this warning, you need to code sign the installer (requires code signing certificate).

## Build for Current Platform

To build for your current platform (automatically detects Windows/Mac):

```bash
npm run tauri:build
```

## Development Build

For testing during development (no installer, just runs the app):

```bash
npm run dev
```

## Build Configuration

The build configuration is in `src-tauri/tauri.conf.json`:

- **App name**: Line 10 - `"productName": "UVCAD"`
- **Version**: Line 11 - `"version": "0.1.0"`
- **Bundle identifier**: Line 48 - `"identifier": "com.uvcad.UVCAD"`
- **Window settings**: Lines 53-62
- **Platform-specific settings**: Lines 65-100

## Code Signing (Optional but Recommended)

### macOS Code Signing

1. Get an Apple Developer account ($99/year)
2. Create a Developer ID Application certificate in Xcode
3. Update `tauri.conf.json`:
   ```json
   "macOS": {
     "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
   }
   ```

### Windows Code Signing

1. Obtain a code signing certificate (from DigiCert, Sectigo, etc.)
2. Update `tauri.conf.json`:
   ```json
   "windows": {
     "certificateThumbprint": "YOUR_CERT_THUMBPRINT",
     "timestampUrl": "http://timestamp.digicert.com"
   }
   ```

## Troubleshooting

### "command not found: cargo" or "command not found: rustc"
- Install Rust from https://rustup.rs/
- Restart your terminal after installation

### "WebView2 runtime not found" (Windows)
- Install WebView2 Runtime: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

### "No valid code signing identity found" (macOS)
- Either get a Developer ID certificate, or
- Remove `signingIdentity` from `tauri.conf.json` (users will see security warning)

### Build fails with "linker error" (Windows)
- Install Visual Studio Build Tools with C++ workload
- Restart terminal after installation

### App bundle is huge (macOS)
- This is normal for universal builds (Intel + Apple Silicon)
- The DMG will be compressed automatically

## File Sizes

Expected installer sizes:
- **macOS DMG**: ~50-70 MB (universal binary)
- **Windows MSI**: ~30-40 MB
- **Windows NSIS**: ~30-40 MB

## Distribution

### macOS
- Distribute the `.dmg` file
- Users can drag the app to Applications folder
- Optional: Distribute via Mac App Store (requires additional setup)

### Windows
- Distribute the `.msi` file (recommended) or `.exe` file
- Users run the installer and follow the wizard
- Optional: Use Windows Store (requires Microsoft Partner account)

## Updating the App

To release a new version:

1. Update version in `package.json` and `src-tauri/tauri.conf.json`
2. Rebuild installers
3. Distribute new installers to users

Future enhancement: Implement auto-update using Tauri's updater feature.

## Support

For build issues:
- Check the [Tauri documentation](https://tauri.app/v1/guides/building/)
- Check the [Tauri Discord](https://discord.com/invite/tauri)
- Review Rust/Cargo errors carefully - they usually indicate missing dependencies
