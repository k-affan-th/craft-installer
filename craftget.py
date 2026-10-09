#!/usr/bin/env python3
"""craftget - pick Craft apps (storytold/*) and install them, or build them from source.

Modes
  install  download the latest GitHub release (.msi / .dmg / .deb / .rpm / AppImage) and install it
  portable download the portable .zip (Windows) / .tar.gz (Linux) and unpack it, no installer
  build    git clone the latest release tag, `cargo build --release`, copy the binary

Usage
  python craftget.py                              interactive (arrows, space = tick, enter = go)
  python craftget.py -a photocraft pdfcraft -m install
  python craftget.py --all -m build --cli --fonts
  python craftget.py --list

Needs only Python 3.8+. `build` additionally needs git and cargo; `install` needs nothing else.
Set GITHUB_TOKEN to avoid API rate limits.
"""
import argparse, hashlib, json, os, platform, shutil, subprocess, sys, tarfile, urllib.request, zipfile
from pathlib import Path

ORG = "storytold"
APPS = {
    "photocraft":  "image editor",
    "vectorcraft": "vector illustration",
    "filmcraft":   "video editor",
    "lightcraft":  "photo library / raw developer",
    "pdfcraft":    "PDF workbench",
    "effectcraft": "motion graphics / VFX",
    "designcraft": "page layout / publishing",
    "wordcraft":   "word processor",
    "gridcraft":   "spreadsheet",
    "deckcraft":   "presentations",
    "cadcraft":    "CAD / drafting",
    "soundcraft":  "audio workstation",
}
HOME = Path.home() / "craft-apps"          # downloads, sources, portable installs, built binaries
IS_WIN, IS_MAC = os.name == "nt", sys.platform == "darwin"


def arch_tags():
    m = platform.machine().lower()
    if m in ("arm64", "aarch64"):
        return "arm64", "aarch64"           # (windows tag, linux tag)
    if m in ("x86", "i386", "i686"):
        return "x86", "x86_64"
    return "x64", "x86_64"


def api(path):
    req = urllib.request.Request(f"https://api.github.com/{path}", headers={"Accept": "application/vnd.github+json", "User-Agent": "craftget"})
    if os.environ.get("GITHUB_TOKEN"):
        req.add_header("Authorization", "Bearer " + os.environ["GITHUB_TOKEN"])
    with urllib.request.urlopen(req, timeout=30) as r:
        return json.load(r)


def download(url, dest, label):
    dest.parent.mkdir(parents=True, exist_ok=True)
    req = urllib.request.Request(url, headers={"User-Agent": "craftget"})
    with urllib.request.urlopen(req, timeout=60) as r, open(dest, "wb") as f:
        total, got = int(r.headers.get("Content-Length") or 0), 0
        while chunk := r.read(1 << 20):
            f.write(chunk)
            got += len(chunk)
            if total:
                print(f"\r  {label}: {got >> 20}/{total >> 20} MB", end="", flush=True)
    print()


def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        while b := f.read(1 << 20):
            h.update(b)
    return h.hexdigest()


def pick_asset(assets, app, ver, mode):
    """Return the asset name that fits this OS/arch/mode, or None."""
    wa, la = arch_tags()
    names = {a["name"]: a for a in assets}
    p = f"{app}-{ver}-"
    if IS_WIN:
        want = f"{p}windows-{wa}-portable.zip" if mode == "portable" else f"{p}windows-{wa}.msi"
    elif IS_MAC:
        want = f"{p}macos-universal.dmg"
    elif mode == "portable":
        want = f"{p}linux-{la}.tar.gz"
    else:
        if shutil.which("dpkg"):
            want = f"{p}linux-{la}.deb"
        elif shutil.which("rpm"):
            want = f"{p}linux-{la}.rpm"
        else:
            want = f"{p}linux-{la}.AppImage"
    return names.get(want)


def run(cmd, **kw):
    print("  $", " ".join(map(str, cmd)))
    subprocess.run(cmd, check=True, **kw)


def do_release(app, mode, args):
    rel = api(f"repos/{ORG}/{app}/releases/latest")
    tag = rel["tag_name"]
    ver = tag.lstrip("v")
    asset = pick_asset(rel["assets"], app, ver, mode)
    if not asset:
        raise SystemExit(f"  no matching asset in {tag} for this platform/mode")
    print(f"  {app} {tag}: {asset['name']}")
    dest = HOME / "downloads" / asset["name"]
    download(asset["browser_download_url"], dest, asset["name"])
    sums = next((a for a in rel["assets"] if a["name"] == "SHA256SUMS.txt"), None)
    if sums:
        sf = HOME / "downloads" / f"{app}-{ver}-SHA256SUMS.txt"
        download(sums["browser_download_url"], sf, "SHA256SUMS.txt")
        want = {l.split()[1].lstrip("*"): l.split()[0] for l in sf.read_text().splitlines() if len(l.split()) == 2}
        if asset["name"] in want:
            if want[asset["name"]].lower() != sha256(dest):
                dest.unlink()
                raise SystemExit("  SHA256 mismatch - download deleted")
            print("  sha256 OK")
    n = asset["name"]
    if mode == "portable":
        out = HOME / "portable" / app
        shutil.rmtree(out, ignore_errors=True)
        out.mkdir(parents=True)
        if n.endswith(".zip"):
            zipfile.ZipFile(dest).extractall(out)
        else:
            tarfile.open(dest).extractall(out)
        print(f"  unpacked to {out}")
    elif n.endswith(".msi"):
        run(["msiexec", "/i", str(dest), "/passive"] + ([] if not args.quiet else ["/qn"]))
    elif n.endswith(".dmg"):
        run(["open", str(dest)])
        print("  drag the app into Applications in the window that opened")
    elif n.endswith(".deb"):
        run(["sudo", "apt-get", "install", "-y", str(dest)])
    elif n.endswith(".rpm"):
        run(["sudo", "rpm", "-U", str(dest)])
    elif n.endswith(".AppImage"):
        bindir = Path.home() / ".local" / "bin"
        bindir.mkdir(parents=True, exist_ok=True)
        target = bindir / app
        shutil.copy2(dest, target)
        target.chmod(0o755)
        print(f"  installed to {target}")


def do_build(app, args):
    for tool in ("git", "cargo"):
        if not shutil.which(tool):
            raise SystemExit(f"  '{tool}' not found in PATH (build mode needs git + Rust: https://rustup.rs)")
    tag = args.ref or api(f"repos/{ORG}/{app}/releases/latest")["tag_name"]
    src = HOME / "src" / app
    if not (src / ".git").exists():
        src.parent.mkdir(parents=True, exist_ok=True)
        run(["git", "clone", "--depth", "1", "--branch", tag, f"https://github.com/{ORG}/{app}", str(src)])
    else:
        run(["git", "-C", str(src), "fetch", "--depth", "1", "origin", "tag", tag, "--no-tags"])
        run(["git", "-C", str(src), "checkout", "-f", tag])
    env = dict(os.environ)
    if args.fonts:
        fonts = HOME / "src" / "craft-fonts"
        if not fonts.exists():
            run(["git", "clone", "--depth", "1", f"https://github.com/{ORG}/craft-fonts", str(fonts)])
        env["CRAFT_FONTS_DIR"] = str(fonts)
    pkgs = [app] + ([f"{app}-cli"] if args.cli else [])
    run(["cargo", "build", "--release"] + [x for p in pkgs for x in ("-p", p)], cwd=src, env=env)
    bindir = HOME / "bin"
    bindir.mkdir(parents=True, exist_ok=True)
    ext = ".exe" if IS_WIN else ""
    for p in pkgs:
        shutil.copy2(src / "target" / "release" / (p + ext), bindir / (p + ext))
    print(f"  built {tag} -> {bindir}")


# ---------- interactive checkbox menu (stdlib only) ----------
def getkey():
    if IS_WIN:
        import msvcrt
        c = msvcrt.getwch()
        if c in ("\x00", "\xe0"):
            return {"H": "up", "P": "down"}.get(msvcrt.getwch(), "")
        return {"\r": "enter", " ": "space", "\x03": "quit"}.get(c, c)
    import termios, tty
    fd = sys.stdin.fileno()
    old = termios.tcgetattr(fd)
    try:
        tty.setraw(fd)
        c = sys.stdin.read(1)
        if c == "\x1b":
            c += sys.stdin.read(2)
    finally:
        termios.tcsetattr(fd, termios.TCSADRAIN, old)
    return {"\x1b[A": "up", "\x1b[B": "down", "\r": "enter", "\n": "enter", " ": "space", "\x03": "quit"}.get(c, c)


def menu(title, items, multi, hint=""):
    """items: list of (label). returns list of selected indexes."""
    cur, sel = 0, set()
    if IS_WIN:
        os.system("")  # enable ANSI
    first = True
    while True:
        if not first:
            sys.stdout.write(f"\x1b[{len(items) + 2}A")
        first = False
        sys.stdout.write(f"\x1b[2K{title}  {hint}\n")
        for i, it in enumerate(items):
            box = ("[x]" if i in sel else "[ ]") if multi else ("(*)" if i == cur else "( )")
            sys.stdout.write(f"\x1b[2K {'>' if i == cur else ' '} {box} {it}\n")
        sys.stdout.write("\x1b[2K\n")
        sys.stdout.flush()
        k = getkey()
        if k == "up":
            cur = (cur - 1) % len(items)
        elif k == "down":
            cur = (cur + 1) % len(items)
        elif k == "space" and multi:
            sel ^= {cur}
        elif k in ("a", "A") and multi:
            sel = set(range(len(items))) if len(sel) < len(items) else set()
        elif k == "enter":
            return sorted(sel) if multi else [cur]
        elif k in ("quit", "q"):
            raise SystemExit(1)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("-a", "--apps", nargs="+", choices=APPS, help="apps to get (omit for interactive menu)")
    ap.add_argument("--all", action="store_true", help="all 12 apps")
    ap.add_argument("-m", "--mode", choices=["install", "portable", "build"])
    ap.add_argument("--cli", action="store_true", help="build mode: also build <app>-cli")
    ap.add_argument("--fonts", action="store_true", help="build mode: use craft-fonts (CJK UI fonts)")
    ap.add_argument("--ref", help="build mode: git tag/branch instead of latest release")
    ap.add_argument("--quiet", action="store_true", help="msi: silent install (no UI)")
    ap.add_argument("--list", action="store_true", help="show apps and latest versions, then exit")
    args = ap.parse_args()

    if args.list:
        for a, d in APPS.items():
            try:
                print(f"{a:12} {api(f'repos/{ORG}/{a}/releases/latest')['tag_name']:8} {d}")
            except Exception as e:
                print(f"{a:12} ?        {d} ({e})")
        return

    apps = list(APPS) if args.all else args.apps
    if not apps:
        print("Craft apps  (up/down move, space tick, a = all, enter confirm, q quit)\n")
        idx = menu("Apps", [f"{a:12} {d}" for a, d in APPS.items()], True)
        if not idx:
            raise SystemExit("nothing selected")
        apps = [list(APPS)[i] for i in idx]
    mode = args.mode
    if not mode:
        print()
        win_note = "download .msi and run installer" if IS_WIN else "download the OS package and install it"
        i = menu("Mode", [f"install   - {win_note}", "portable  - download and unpack, no installer", "build     - clone source + cargo build --release"], False)[0]
        mode = ["install", "portable", "build"][i]
        if mode == "build" and not (args.cli or args.fonts):
            opts = menu("Build options", ["also build the -cli binary", "use craft-fonts (CJK fonts in UI)"], True)
            args.cli, args.fonts = 0 in opts, 1 in opts

    failed = []
    for a in apps:
        print(f"\n== {a} ({mode})")
        try:
            do_build(a, args) if mode == "build" else do_release(a, mode, args)
        except (subprocess.CalledProcessError, OSError, SystemExit, Exception) as e:
            print(f"  FAILED: {e}")
            failed.append(a)
    print("\nDone." + (f" Failed: {', '.join(failed)}" if failed else ""))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
