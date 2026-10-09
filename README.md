# Craft Installer

Installs the Craft apps from `github.com/storytold` (PhotoCraft, VectorCraft, LightCraft, DesignCraft, CADCraft,
FilmCraft, EffectCraft, SoundCraft, WordCraft, GridCraft, DeckCraft, PdfCraft).

| Tool | Use |
|---|---|
| `CraftInstaller.exe` / `gui/` | Rust window app (3 steps, 10 languages). Windows build is tested. |
| `craftget.py`, `craftget.cmd` | Command-line menu, Python 3 only |
| `craftget.sh` | Same for Linux / macOS / Git Bash |

Headless: `CraftInstaller --install portable|msi|build all|app1 app2 ...`, `--uninstall ...`, `--pin`
(log goes to `~/craft-apps/last-run.log`).

## Build the window app

```
cd gui
cargo build --release        # Windows, macOS, Linux
```
Linux needs `libxkbcommon`, `libwayland`, `libx11`, `libGL` (runtime) and a C compiler. `.github/workflows/build.yml`
builds all three OSes on GitHub Actions.

## What each mode does per OS

| Mode | Windows | macOS | Linux |
|---|---|---|---|
| Quick | unzip + Start Menu shortcut | `.dmg` -> `~/Applications` | `.tar.gz` -> `~/craft-apps/apps`, `.desktop` entry |
| System installer | `.msi`, one UAC for the batch | `.dmg` -> `/Applications` | `.deb`/`.rpm` via one `pkexec` prompt (AppImage if neither) |
| Build | git + cargo | git + cargo | git + cargo |

Hindi and Arabic are not offered: egui cannot shape those scripts.
