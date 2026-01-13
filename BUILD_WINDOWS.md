# Building UVCAD on Windows

## Prerequisites

### 1. Install Node.js
Download and install from: https://nodejs.org/
- Version 18 or later
- Verify: `node --version` and `npm --version`

### 2. Install Rust
Download and install from: https://rustup.rs/
- Run `rustup-init.exe`
- Follow the prompts (default options are fine)
- Restart your terminal after installation
- Verify: `rustc --version` and `cargo --version`

### 3. Install Visual Studio Build Tools
Download from: https://visualstudio.microsoft.com/downloads/
- Scroll down to "Tools for Visual Studio"
- Download "Build Tools for Visual Studio 2022"
- Run the installer
- Select "Desktop development with C++"
- Install (this will take 5-10 minutes)

### 4. Install WebView2 (Usually Already Installed)
WebView2 is pre-installed on Windows 10/11. If needed:
- Download from: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

## Building the Installer

### 1. Clone and Setup
```powershell
# Clone the repository
git clone <repository-url>
cd UVCAD

# Install dependencies
npm install
```

### 2. Build the Installer
```powershell
# Build Windows installer
npm run build:windows
```

Or build for current platform:
```powershell
npm run tauri:build
```

### 3. Build Output

The build will create:
- **MSI Installer**: `src-tauri\target\release\bundle\msi\UVCAD_0.1.0_x64_en-US.msi`
- **NSIS Installer**: `src-tauri\target\release\bundle\nsis\UVCAD_0.1.0_x64-setup.exe`

Both installers work great - MSI is more "official" and integrates better with Windows.

### Expected Build Time
- First build: 5-10 minutes (downloads and compiles dependencies)
- Subsequent builds: 1-3 minutes

### Expected File Sizes
- **MSI installer**: ~30-40 MB
- **NSIS installer**: ~30-40 MB

## Testing the Installer

### Install Locally
1. Double-click the MSI file
2. Follow the installation wizard
3. Launch from Start Menu

### Verify Installation
- Check: `C:\Program Files\UVCAD\UVCAD.exe`
- Start Menu shortcut should be created
- App appears in "Add or Remove Programs"

## Troubleshooting

### Error: "rustc not found" or "cargo not found"
- Close and reopen PowerShell/Command Prompt
- If still failing, add to PATH manually:
  - Open "Edit system environment variables"
  - Add: `%USERPROFILE%\.cargo\bin`

### Error: "link.exe not found"
- Visual Studio Build Tools not installed correctly
- Reinstall and ensure "Desktop development with C++" is selected
- Restart terminal after installation

### Error: "WebView2 not found"
- Download WebView2 Runtime: https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- Install the "Evergreen Standalone Installer"

### Error: "failed to run custom build command for `openssl-sys`"
- Install Perl: https://strawberryperl.com/
- Or use pre-built OpenSSL: `vcpkg install openssl:x64-windows`

### Build Warnings
- Warnings are normal and safe to ignore
- The installer will still work perfectly

## Distribution

1. Share the MSI file with users
2. Users double-click to install
3. For unsigned installers, users may see "Windows protected your PC"
   - Click "More info" → "Run anyway"

## Code Signing (Optional)

To remove security warnings:

1. Obtain a code signing certificate (~$100-300/year)
   - Providers: DigiCert, Sectigo, SSL.com

2. Update `tauri.conf.json`:
   ```json
   "windows": {
     "certificateThumbprint": "YOUR_CERT_THUMBPRINT",
     "digestAlgorithm": "sha256",
     "timestampUrl": "http://timestamp.digicert.com"
   }
   ```

3. Rebuild - installer will be signed automatically

## Automated Builds

See `.github/workflows/build.yml` for GitHub Actions setup to build automatically on every release.
