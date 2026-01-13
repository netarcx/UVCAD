# UVCAD Installation Guide

Easy installation instructions for end users.

## Download

Download the installer for your operating system:

- **macOS**: Download `UVCAD_0.1.0_universal.dmg`
- **Windows**: Download `UVCAD_0.1.0_x64_en-US.msi`

## Installation

### macOS Installation

1. **Open the downloaded DMG file**
   - Double-click `UVCAD_0.1.0_universal.dmg`

2. **Install the application**
   - Drag the UVCAD icon to the Applications folder

3. **Launch UVCAD**
   - Open Finder → Applications
   - Double-click UVCAD

4. **First Launch (Security)**
   - If you see "UVCAD cannot be opened because it is from an unidentified developer":
     - Click "OK"
     - Go to System Preferences → Security & Privacy
     - Click "Open Anyway" next to the UVCAD message
     - Click "Open" in the confirmation dialog

   **Alternative method**:
   - Right-click (or Control+click) on UVCAD in Applications
   - Select "Open" from the menu
   - Click "Open" in the dialog

   After the first launch, you can open UVCAD normally without these steps.

### Windows Installation

1. **Run the installer**
   - Double-click `UVCAD_0.1.0_x64_en-US.msi`

2. **Follow the installation wizard**
   - Click "Next" to begin
   - Choose installation location (default is recommended)
   - Click "Install"
   - Click "Finish" when complete

3. **Launch UVCAD**
   - Find UVCAD in your Start Menu
   - Or use the desktop shortcut (if created)

4. **First Launch (Security)**
   - If you see "Windows protected your PC":
     - Click "More info"
     - Click "Run anyway"

   After the first launch, Windows will remember your choice.

## System Requirements

### macOS
- **OS**: macOS 10.13 (High Sierra) or later
- **Architecture**: Intel or Apple Silicon (M1/M2/M3)
- **Disk Space**: 100 MB free space

### Windows
- **OS**: Windows 10 (version 1809+) or Windows 11
- **Architecture**: 64-bit (x64)
- **Disk Space**: 100 MB free space
- **WebView2**: Usually pre-installed on Windows 10/11

## First-Time Setup

After installation, you'll need to configure UVCAD:

1. **Set Local Folder**
   - Click "Settings" button
   - Browse and select your CAD files folder

2. **Connect Google Drive** (Optional)
   - Click "Settings" button
   - Import your Google API credentials JSON file, or
   - Enter Client ID and Client Secret manually
   - Click "Connect to Google Drive"
   - Enter the Google Drive folder ID

3. **Configure Samba Share** (Optional)
   - Click "Settings" button
   - Enter your Samba share path (e.g., `//server/share`)

4. **Start Syncing**
   - Click the "Start Sync" button
   - Monitor progress in the progress bar
   - View synced files in the Files section

## Uninstallation

### macOS
1. Open Finder → Applications
2. Drag UVCAD to the Trash
3. Empty the Trash

To remove app data (optional):
- Delete: `~/Library/Application Support/com.uvcad.UVCAD/`

### Windows
1. Open Settings → Apps → Installed apps
2. Find UVCAD in the list
3. Click the three dots → Uninstall
4. Follow the uninstallation wizard

To remove app data (optional):
- Delete: `%APPDATA%\com.uvcad.UVCAD\`

## Troubleshooting

### "App is damaged and can't be opened" (macOS)
This may happen if the DMG was corrupted during download.
- Solution: Re-download the DMG file and try again

### "Windows Installer package problem" (Windows)
- Solution: Right-click the MSI file → Run as Administrator

### Application won't start
- **macOS**: Check Console app for error messages
- **Windows**: Check Event Viewer → Windows Logs → Application

### Sync not working
- Verify internet connection
- Check that folders have read/write permissions
- Verify Google Drive authentication is valid

## Getting Help

For issues or questions:
- Check the Settings panel for deletion safety information
- Review the application logs
- Contact support or file an issue on the project repository

## Security Note

UVCAD includes deletion safety protection:
- Maximum 50 files deleted per sync
- Maximum 30% of files deleted per sync

If a sync would exceed these limits, it will be automatically blocked to prevent accidental data loss.
