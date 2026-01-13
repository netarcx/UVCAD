# Quick Build Reference

## For Developers - Building Installers

### One-Time Setup
```bash
# Install dependencies
npm install

# Verify Rust is installed
rustc --version
cargo --version
```

### Build Commands

#### Build for Current Platform (Easiest)
```bash
npm run tauri:build
```

#### Build for macOS (Universal - Intel + Apple Silicon)
```bash
npm run build:mac
```
Output: `src-tauri/target/release/bundle/dmg/UVCAD_0.1.0_universal.dmg`

#### Build for Windows
```bash
npm run build:windows
```
Output: `src-tauri/target/release/bundle/msi/UVCAD_0.1.0_x64_en-US.msi`

### Build Time
- First build: 5-10 minutes (downloads dependencies)
- Subsequent builds: 1-3 minutes

### What Gets Built

#### macOS
- **UVCAD.app** - Application bundle
- **UVCAD_0.1.0_universal.dmg** - Installer (distribute this)

#### Windows
- **UVCAD_0.1.0_x64_en-US.msi** - Windows Installer (distribute this)
- **UVCAD_0.1.0_x64-setup.exe** - NSIS installer (alternative)

### Development Mode
```bash
npm run dev
```
Runs the app without building an installer. Hot-reload enabled.

### Regenerate Icons
If you change the icon:
```bash
# Update app-icon.png (must be 1024x1024px PNG)
npm run tauri icon app-icon.png
```

### Version Bump
Update version in two places:
1. `package.json` → `"version": "0.2.0"`
2. `src-tauri/tauri.conf.json` → `"version": "0.2.0"`

Then rebuild installers.

---

See **BUILD.md** for detailed instructions and troubleshooting.
See **INSTALL.md** for end-user installation instructions.
