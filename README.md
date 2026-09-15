# duster

Disk cleanup CLI for developers. Finds and removes build artifacts, caches, and other space hogs.

## Install

**macOS / Linux:**
```bash
curl -fsSL https://raw.githubusercontent.com/Jon2G/duster/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Jon2G/duster/main/install.ps1 | iex
```

<details>
<summary>Other install methods</summary>

**With Cargo:**
```bash
cargo install --git https://github.com/Jon2G/duster
```

**Manual download:**
- [macOS Apple Silicon](https://github.com/Jon2G/duster/releases/latest/download/duster-macos-arm64.tar.gz)
- [macOS Intel](https://github.com/Jon2G/duster/releases/latest/download/duster-macos-x86_64.tar.gz)
- [Linux x86_64](https://github.com/Jon2G/duster/releases/latest/download/duster-linux-x86_64.tar.gz)
- [Windows x86_64](https://github.com/Jon2G/duster/releases/latest/download/duster-windows-x86_64.zip)

</details>

## Commands

```bash
duster scan              # Find cleanable files (dry-run)
duster clean             # Select categories, then delete (with confirmation)
duster clean -y          # Delete without confirmation (skips sensitive items)
duster analyze           # Detailed breakdown by category
duster space             # Total / free disk space (default: home fs)
duster space --path /tmp # For a specific path's filesystem
duster space --json      # Machine-readable output
duster tui               # Interactive TUI to visualize cleanable disk usage
duster config            # Show current settings
```

## Categories

```bash
--cache       # App/system caches (~/.cache, ~/Library/Caches, known Windows caches)
--trash       # Trash / Recycle Bin
--temp        # Temp files older than 1 day (%TEMP% / /tmp)
--downloads   # Old files in ~/Downloads
--build       # Build artifacts from inactive projects (node_modules, target/, obj/, etc.)
--large       # Files over 100MB
--duplicates  # Duplicate files (by hash)
--old         # Files not accessed in 30+ days
--all, -a     # All categories (default if none specified)
```

## Options

```bash
--min-age <DAYS>         # Age threshold for old files (default: 30)
--min-size <SIZE>        # Size threshold for large files (default: 100MB)
--project-age <DAYS>     # Projects inactive for this long are cleanable (default: 14)
--path <PATH>            # Scan path (default: home directory)
--exclude <PATTERN>      # Exclude matching paths (repeatable)
--include-sensitive      # Opt into broad AppData (Windows); results tagged sensitive
--force-sensitive        # With clean -y, allow deleting sensitive items (requires --include-sensitive)
--json                   # Output as JSON
```

## Examples

```bash
# Quick cache cleanup
duster clean --cache --trash -y

# Find build artifacts from old projects
duster scan --build --project-age 30

# Large files over 500MB
duster scan --large --min-size 500MB

# Windows: include Local/Roaming AppData (risky)
duster scan --cache --include-sensitive
duster clean --cache --include-sensitive --force-sensitive -y

# Everything as JSON
duster scan --json
```

## Windows notes

- Default cache scan uses known-safe paths (npm, NuGet, Cargo, browser caches, etc.), not all of AppData.
- Broad AppData discovery requires `--include-sensitive` (alias: `--include-appdata`). Items are tagged `risk: sensitive`.
- `duster clean -y` never deletes sensitive items unless you also pass `--include-sensitive --force-sensitive`.
- Recycle Bin scanning may return nothing without permission to read `$Recycle.Bin`.
- Config file: `%APPDATA%\duster\config.toml`

## TUI Mode

Run `duster tui` for an interactive terminal visualization of cleanable disk space by category.
Use `up/down` (or `j/k`) to move between categories and `q` to exit.

## Config File

Optional: `~/.config/duster/config.toml` (Windows: `%APPDATA%\duster\config.toml`)

```toml
min_age_days = 30
min_large_size_mb = 100
project_recent_days = 14
download_age_days = 30
include_sensitive = false
excluded_paths = ["important-project/node_modules"]
custom_paths = [
  { path = "~/Library/Application Support/Cursor Nightly", category = "cache", description = "Cursor Nightly app data" },
  { path = "~/Library/Caches/co.anysphere.cursor.nightly", category = "cache", description = "Cursor Nightly cache" },
  { path = "~/Library/Caches/co.anysphere.cursor.nightly.ShipIt", category = "cache", description = "Cursor Nightly updater cache" },
  { path = "~/dev/everysphere/anyrun/target", category = "build", description = "Anyrun build artifacts" }
]
```

### Custom Clean Paths

Use `custom_paths` to include specific directories or files that duster doesn't
discover automatically. Each entry supports:

- `path`: Absolute or `~/`-relative path.
- `category`: One of `cache`, `build`, `trash`, `temp`, `downloads`, `large`, `duplicates`, `old`.
- `description`: Optional text shown in reports.
- `min_size_mb`: Optional size threshold (defaults to 1MB).

## How Build Detection Works

Build artifacts (`node_modules`, `target/`, `.gradle`, `obj/`, `.vs`, etc.) are only flagged if the parent project hasn't been modified within `--project-age` days. This protects active projects.
