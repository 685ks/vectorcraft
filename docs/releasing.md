# Releasing VectorCraft

> **Last updated:** 2026-10-21 · **Change:** releases go through the Arthavan Projects release workflow (685ks/craft-actions)

`.github/workflows/release.yml` calls the shared Arthavan Projects release workflow,
`685ks/craft-actions/.github/workflows/release-rust-app.yml`, which builds, packages and signs
VectorCraft and creates the GitHub Release `v<version>` on this repository. It runs only when started
by hand. The Arthavan Projects launcher installs and updates VectorCraft from these releases, so a
launcher-started VectorCraft (`ARTHAVAN_LAUNCHER=1`) runs no updater of its own and its About box says
"Updates are managed by Arthavan Projects". Packaging metadata the shared workflow reads (app id,
display name, bundle id `app.freedev.ainz.ks.VectorCraft`, publisher, categories, icons, license)
is in [`packaging/app.toml`](../packaging/app.toml).

User-facing names say **VectorCraft**. Files, binaries and ids stay lowercase
(`vectorcraft-<version>-<platform>-<arch>.<ext>`, `app.freedev.ainz.ks.VectorCraft`).

## Cutting a release

1. **Bump the version** on `main`. It lives in exactly one place, `[workspace.package] version`
   in the root `Cargo.toml`; every crate inherits it and the packaging scripts read it from there:

   ```sh
   cargo xtask version                 # prints the current version, e.g. 0.3.1
   cargo xtask version set 0.4.0       # or 0.4.0-rc.1; updates Cargo.toml and Cargo.lock
   ```

   Commit the change (`Cargo.toml` + `Cargo.lock`) through the normal review flow.
2. **Start the release**: *Actions › Release › Run workflow* on `main`, with `version` set to that
   version (without the `v`). Leave `publish` off for a draft you check first; set `prerelease` for
   a `-rc` build.
3. **Check the draft.** Download an installer or two and read the job summaries.
4. **Publish** the draft in the GitHub UI (or run the workflow again with `publish` on).

The platform notes below describe the packaging scripts in `packaging/`, which the release builds
use; which platforms a release covers is decided by the shared workflow.

## What gets built

| Platform | Artifacts | Built on |
|---|---|---|
| Windows x64 | `vectorcraft-<v>-windows-x64.msi`, `vectorcraft-<v>-windows-x64-portable.zip` | `windows-latest` |
| Windows x86 (32-bit) | `vectorcraft-<v>-windows-x86.msi`, `vectorcraft-<v>-windows-x86-portable.zip` | `windows-latest` |
| Windows on ARM64 | `vectorcraft-<v>-windows-arm64.msi`, `vectorcraft-<v>-windows-arm64-portable.zip` | `windows-latest` (cross-compiled) |
| Web | `vectorcraft-web-<v>.zip`, a static site (see [`packaging/web/README.md`](../packaging/web/README.md)) | `ubuntu-latest` |

`vectorcraft --version` and `vectorcraft-cli --version` print the version from `Cargo.toml`.

**Fonts.** Every build job checks out [craft-fonts](https://github.com/685ks/craft-fonts) at a
pinned commit (the `ref:` of its craft-fonts checkout step) and builds with `CRAFT_FONTS_DIR` and
`CRAFT_FONTS_REQUIRED=1`, so releases embed its Japanese fonts and fail rather than ship without
them (the web build embeds only the UI font, BIZ UDPGothic Regular, to stay small). The packages carry each embedded font's licence as `OFL-<family>.txt`
(`copy_font_licences` in `packaging/env.sh`). Bump the pins deliberately, in every job at once.
The rules are in craftrules `standards/fonts.md`, the build option in
[`docs/development.md`](development.md) › Fonts.

### Windows

`packaging/windows/package.ps1 -Arch x64|x86|arm64` builds with `-C target-feature=+crt-static`,
so neither the MSI nor the portable zip needs the Visual C++ redistributable. The flag is scoped
to the target triple, so host build scripts aren't affected.

- Before packaging, the script checks each binary's PE machine type against `-Arch` (an x64 build
  can never ship labelled arm64), that `vectorcraft.exe` is a GUI program (no console window) and
  that `vectorcraft-cli.exe` is a console program.
- **Portable zip:** it ships `packaging/windows/portable.txt` beside `vectorcraft.exe`. That
  marker (or a `VectorCraft.portable` file) switches on portable mode, as in PhotoCraft: `ui.json`
  and the folders next to it (Swatches, Graphic Styles, Fonts, Data Recovery, logs…) go to
  `VectorCraftData\` next to the exe instead of `%APPDATA%\VectorCraft`. The first time
  `VectorCraftData` is created, the settings already in `%APPDATA%\VectorCraft` are copied into it
  (the originals stay), so someone updating a portable zip from before the marker keeps their
  settings. If that folder isn't writable the app logs a warning and uses `%APPDATA%`. Deleting
  `portable.txt` returns settings to the user profile (the `VectorCraftData` folder is left as it
  is). The MSI has no marker. `VECTORCRAFT_CONFIG_DIR` still wins, `VECTORCRAFT_NO_PREFS` never
  looks for the marker, and a portable copy skips the one-time read of the old DrawCraft settings.
  The logic is in `apps/vectorcraft/src/profile.rs`. On the other platforms the marker goes where
  the user sees the app, never inside it: on macOS next to `VectorCraft.app` (nothing is written
  inside the signed bundle), for an AppImage next to the `.AppImage` file (`$APPIMAGE`; the mounted
  executable's own folder is read-only). Flatpak isn't supported: `/app` is read-only and the
  sandbox has no folder "beside the app", so a Flatpak install always uses its own config folder.
- `vectorcraft.wxs` (WiX) is a per-machine install into Program Files with a Start Menu shortcut
  and an App Paths entry (Win+R `vectorcraft`). Same-version upgrades are allowed, so release
  candidates replace each other.
- Double-clicking the MSI opens a setup wizard (WixUI_InstallDir without the licence page):
  Welcome, install folder (remembered for upgrades in `HKLM\Software\VectorCraft\InstallDir`),
  Shortcuts (a ticked "Create a desktop shortcut" box), Ready, a progress page, and a Finish page
  that says where to find the app, with a ticked "Launch VectorCraft" box. The pages name the full
  version (`vectorcraft.en-us.wxl`). `package.ps1` draws the wizard's banner and side bitmaps from
  the app icon, so no WiX stock art ships. `msiexec /qn` still installs silently, with the desktop
  shortcut unless `INSTALLDESKTOPSHORTCUT=""` is passed. Building needs the WiX UI and Util
  extensions: `wix extension add -g WixToolset.UI.wixext/5.0.2 WixToolset.Util.wixext/5.0.2`.
- The ARM64 build is cross-compiled on the x64 runner. `.github/workflows/windows-arm64.yml`
  installs that MSI on a Windows 11 ARM64 runner, checks that both installed programs are ARM64,
  runs `vectorcraft-cli --version` natively and uninstalls.
- **Signing:** `packaging/windows/sign.ps1` signs both `.exe` files and then the `.msi` with
  `signtool` (SHA-256, RFC 3161 timestamp), using whichever material is present:
  1. a `.pfx` certificate (`WINDOWS_CERTIFICATE`, base64, and `WINDOWS_CERTIFICATE_PASSWORD`), or
  2. Azure Trusted Signing (`AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`,
     `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT`, `AZURE_CERT_PROFILE`).

  With neither, it warns and leaves the files unsigned, so test builds still produce installers.
  It is the one place to adapt when the signing setup changes; `WINDOWS_TIMESTAMP_URL` and
  `SIGNTOOL` override the timestamp server and the `signtool.exe` path.

### Web

The web job builds `apps/vectorcraft-web` with trunk and zips the static site
(`packaging/web/package.sh`). [`packaging/web/README.md`](../packaging/web/README.md) covers
hosting: any path works, `.wasm` must be served as `application/wasm`, and the sample header
files for common hosts.

## The draft release

The last job waits for every build, downloads their artifacts, writes `SHA256SUMS.txt` and creates
the draft `VectorCraft v<version>` with notes generated from the merged pull requests. If the draft
already exists, it replaces its assets and keeps it a draft. If that version is already published,
the job fails and asks for a version bump (`cargo xtask version set`).

## Secrets

The jobs that sign (Windows) and the draft-release job run in the `release` environment,
which only the `release` branch can use and which holds the signing secrets. The other jobs get no
secrets. Every secret is optional: a missing one produces unsigned artifacts and a
warning, never a failed build.

| Secret | Used for |
|---|---|
| `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `KEYCHAIN_PASSWORD` | the Developer ID certificate (base64 `.p12`), imported into a temporary keychain by `packaging/macos/import-cert.sh` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | notarization (`APPLE_PASSWORD` is an app-specific password) |
| `WINDOWS_CERTIFICATE`, `WINDOWS_CERTIFICATE_PASSWORD` | Windows signing with a `.pfx` |
| `AZURE_TENANT_ID`, `AZURE_CLIENT_ID`, `AZURE_CLIENT_SECRET`, `AZURE_SIGNING_ENDPOINT`, `AZURE_SIGNING_ACCOUNT`, `AZURE_CERT_PROFILE` | Windows signing with Azure Trusted Signing |

## Revision history

| Date | Change | Summary |
|---|---|---|
| 2026-10-14 | minor | RPM glibc floor names libm's `GLIBC_2.35`: RHEL 9's glibc 2.34 provides libc's through backports |
| 2026-10-12 | minor | RPM dependencies use 64-bit runtime capabilities and enforce glibc 2.35 (#1275); real distro install/start-up checks remain pending |
| 2026-10-12 | minor | Portable mode (#1366): the Windows portable zip ships `portable.txt`; where the marker goes on macOS and Linux, and what Flatpak can't do |
