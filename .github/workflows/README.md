# GitHub Actions Workflows

This directory contains automated build workflows for UVCAD installers.

## Available Workflows

### 1. Build and Release (`build-release.yml`)

**Triggers**: Automatically when you push a version tag (e.g., `v0.1.0`)

**What it does**:
- Builds Windows installer (MSI + NSIS)
- Builds macOS installer (DMG for Apple Silicon)
- Creates a GitHub Release with all installers attached
- Generates release notes

**How to use**:
```bash
# Create and push a version tag
git tag v0.1.0
git push origin v0.1.0

# GitHub Actions will automatically:
# 1. Build installers for Windows and macOS
# 2. Create a release at: https://github.com/YOUR_USERNAME/UVCAD/releases
# 3. Attach all installers to the release
```

**Build time**: ~15-20 minutes total (both platforms in parallel)

### 2. Manual Build (`manual-build.yml`)

**Triggers**: Manually from GitHub Actions tab

**What it does**:
- Lets you choose which platform to build (Windows, macOS, or both)
- Builds installer(s) without creating a release
- Artifacts available for 7 days

**How to use**:
1. Go to: `https://github.com/YOUR_USERNAME/UVCAD/actions`
2. Click "Manual Build" in the left sidebar
3. Click "Run workflow"
4. Select platform (windows/macos/both)
5. Click "Run workflow"
6. Wait for build to complete (~10 minutes per platform)
7. Download installers from the workflow run page

## Requirements

### For Your Repository
- Push the `.github/workflows/` directory to your repository
- No additional configuration needed - workflows work out of the box!

### For Private Repositories
- Workflows run using GitHub Actions minutes
- Free tier: 2000 minutes/month (plenty for this project)

### For Public Repositories
- Unlimited free GitHub Actions minutes!

## Workflow Features

✅ **Automatic dependency installation**
- Node.js, Rust, and build tools automatically installed
- No manual setup required

✅ **Parallel builds**
- Windows and macOS build simultaneously
- Faster than building sequentially

✅ **Artifact preservation**
- Installers saved as downloadable artifacts
- Available for 7 days (manual builds) or permanently (releases)

✅ **Release automation**
- Creates GitHub Release automatically
- Generates release notes
- Attaches all installers

## Build Output

### Windows Build Produces:
- `UVCAD_0.1.0_x64_en-US.msi` (~30-40 MB)
- `UVCAD_0.1.0_x64-setup.exe` (~30-40 MB)

### macOS Build Produces:
- `UVCAD_0.1.0_aarch64.dmg` (~6 MB)

## Customization

### Change Version Number
Edit both:
- `package.json` → `"version": "0.2.0"`
- `src-tauri/tauri.conf.json` → `"version": "0.2.0"`

Then create a new tag:
```bash
git tag v0.2.0
git push origin v0.2.0
```

### Build for Intel Macs Too
Edit `build-release.yml` and add:
```yaml
- name: Setup Rust
  uses: dtolnay/rust-toolchain@stable
  with:
    targets: aarch64-apple-darwin, x86_64-apple-darwin

- name: Build macOS installer (Universal)
  run: npm run build:mac
```

### Add Code Signing
For Windows, add to repository secrets:
- `WINDOWS_CERTIFICATE_THUMBPRINT`
- `WINDOWS_TIMESTAMP_URL`

Update workflow to use secrets in build step.

## Troubleshooting

### Build Fails on Windows
- Usually dependency/compilation errors
- Check the "Build Windows installer" step logs
- Common issues: Missing Visual Studio Build Tools components

### Build Fails on macOS
- Usually Rust target issues
- Check the "Build macOS installer" step logs
- Ensure correct targets are installed

### Release Not Created
- Ensure you pushed a tag (not just committed)
- Tag must start with 'v' (e.g., v0.1.0)
- Check workflow permissions in repo settings

### Artifacts Not Uploaded
- Check if build completed successfully
- Verify file paths in workflow match actual output
- Artifacts expire after 7 days for manual builds

## Local Development

To test locally before pushing:
```bash
# Install act (GitHub Actions local runner)
brew install act  # macOS
# or
choco install act-cli  # Windows

# Run workflow locally
act -j build-macos  # Test macOS build
act -j build-windows  # Test Windows build (requires Docker)
```

## Cost Estimation

### For Typical Usage (10 releases/month):
- Public repo: **FREE**
- Private repo: ~200 minutes/month (**FREE** with 2000 min/month allowance)

### Each Build Uses:
- Windows: ~10 minutes
- macOS: ~10 minutes
- Total per release: ~20 minutes

## Support

For GitHub Actions issues:
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Tauri CI/CD Guide](https://tauri.app/v1/guides/building/cross-platform)
