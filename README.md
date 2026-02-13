# rtree

High-performance Rust CLI for analyzing disk usage of files and directories on Windows.

`rtree` scans a target path, computes sizes, and prints a clean table of items sorted by size or name.

## Features

- Multi-threaded scanning with `rayon`
- Fast traversal with `walkdir`
- Real-time scan progress spinner via `indicatif`
- Sort results by size (`--sort size`) or name (`--sort name`)
- Limit output rows with `--limit` or `-n`
- Human-readable sizes (`B`, `KB`, `MB`, `GB`, `TB`)
- Graceful handling of `PermissionDenied` during traversal

## Requirements

- Windows (optimized for NTFS workloads)
- Rust stable toolchain

Install Rust from: https://www.rust-lang.org/tools/install

## Build

Debug build:

```powershell
cargo build
```

Optimized release build:

```powershell
cargo build --release
```

Binary path:

```text
target\release\rtree.exe
```

## Usage

```powershell
rtree.exe [PATH] [--sort <size|name>] [--limit <N>]
```

### Arguments

- `PATH` (optional): target path to scan. Defaults to `.`.

### Options

- `--sort <size|name>`: sort output by `size` (default) or `name`.
- `--limit <N>`, `-n <N>`: show only top `N` items.
- `--help`: show full CLI help.
- `--version`: show version.

## Examples

Scan `C:\Windows`, sort by size, show top 20:

```powershell
.\target\release\rtree.exe C:\Windows --sort size --limit 20
```

Scan current directory, show top 10:

```powershell
.\target\release\rtree.exe . -n 10
```

Sort by name:

```powershell
.\target\release\rtree.exe C:\Users\admin --sort name
```

## Output

`rtree` prints an aligned table:

- `Size`: human-readable size
- `Type`: `DIR` or `FILE`
- `Path`: full or relative item path

During scanning, a live spinner displays elapsed time and files scanned.

## Error Handling Notes

- Protected folders may produce permission warnings and are skipped.
- The tool continues scanning instead of crashing on inaccessible entries.
- Paths with special characters are handled using `PathBuf` and lossy display formatting.

## Development

Check code compiles:

```powershell
cargo check
```

Run formatter:

```powershell
cargo fmt
```
