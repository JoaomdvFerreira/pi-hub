# Publishing a Pi-Hub release

Pi-Hub checks `https://github.com/JoaomdvFerreira/pi-hub/releases/latest/download/latest.json`
for updates (`src-tauri/tauri.conf.json`'s `plugins.updater.endpoints`). Nothing
publishes there automatically -- there is no CI/CD pipeline for this by
design (see WU025's scope). A release is built and pushed by hand, the same
way every other build in this project has been produced.

## Prerequisites

- The signing private key at `C:\Users\João\.tauri\pi-hub-updater.key`
  (never commit this file; its public counterpart is already embedded in
  `tauri.conf.json`'s `plugins.updater.pubkey`).
- `gh` authenticated against the `JoaomdvFerreira/pi-hub` repo.

## Steps

1. Bump the version in `src-tauri/tauri.conf.json` (`"version"`) and
   `package.json` so they match, and commit that change.

2. Build the signed bundle:

   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY_PATH = "C:\Users\João\.tauri\pi-hub-updater.key"
   npm run tauri build
   ```

   Because `bundle.createUpdaterArtifacts` is `true`, this produces (under
   `src-tauri/target/release/bundle/`) both the installer(s) (NSIS `.exe`
   and/or MSI) and updater artifacts: a `.sig` signature file next to a
   compressed update archive.

3. Create a GitHub Release tagged `vX.Y.Z` (matching the version bumped in
   step 1) and upload:
   - The installer (`.exe`/`.msi`) for people installing fresh.
   - The updater archive and its `.sig` file, for in-app updates.
   - A `latest.json` manifest (see below) -- this is the file the app's
     endpoint URL actually resolves to via GitHub's `.../releases/latest/...`
     alias, so it must be attached with exactly that filename.

   ```powershell
   gh release create vX.Y.Z `
     "src-tauri\target\release\bundle\nsis\Pi-Hub_X.Y.Z_x64-setup.exe" `
     "src-tauri\target\release\bundle\nsis\Pi-Hub_X.Y.Z_x64-setup.exe.sig" `
     "path\to\latest.json" `
     --title "vX.Y.Z" --notes "..."
   ```

4. `latest.json` shape (Tauri's updater format):

   ```json
   {
     "version": "X.Y.Z",
     "notes": "...",
     "pub_date": "2026-01-01T00:00:00Z",
     "platforms": {
       "windows-x86_64": {
         "signature": "<contents of the .sig file>",
         "url": "https://github.com/JoaomdvFerreira/pi-hub/releases/download/vX.Y.Z/Pi-Hub_X.Y.Z_x64-setup.exe"
       }
     }
   }
   ```

   `tauri build` prints the exact bundle paths and echoes the signature at
   the end of the build; the `.sig` file's contents go verbatim into the
   `signature` field.

5. Verify: from a machine running an older Pi-Hub build, open
   Settings -> Check for updates. It should find the new release, download
   it (signature-verified by the updater plugin against the embedded
   pubkey), and offer to restart into the new version.
