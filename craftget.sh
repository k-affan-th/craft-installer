#!/usr/bin/env bash
# craftget.sh - pick Craft apps (storytold/*) and install them, or build them from source.
# Same behaviour as craftget.py, for Linux / macOS / Git Bash. Needs only bash + curl (+ git, cargo for build).
#
#   ./craftget.sh                              interactive (up/down, space = tick, a = all, enter = go)
#   ./craftget.sh -a "photocraft pdfcraft" -m install
#   ./craftget.sh --all -m build --cli --fonts
#   ./craftget.sh --list
#
# Modes: install (.dmg/.deb/.rpm/AppImage/.msi) | portable (unpack archive) | build (git clone + cargo build --release)
# Set GITHUB_TOKEN to avoid API rate limits.
set -uo pipefail

ORG=storytold
APPS=(photocraft vectorcraft filmcraft lightcraft pdfcraft effectcraft designcraft wordcraft gridcraft deckcraft cadcraft soundcraft)
DESC=("image editor" "vector illustration" "video editor" "photo library / raw developer" "PDF workbench" "motion graphics / VFX" "page layout / publishing" "word processor" "spreadsheet" "presentations" "CAD / drafting" "audio workstation")
HOME_DIR="$HOME/craft-apps"
SEL_APPS=""; MODE=""; CLI=0; FONTS=0; REF=""; ALL=0; LIST=0

case "$(uname -s)" in
  Darwin) OS=mac ;;
  MINGW*|MSYS*|CYGWIN*) OS=win ;;
  *) OS=linux ;;
esac
case "$(uname -m)" in
  arm64|aarch64) WA=arm64; LA=aarch64 ;;
  i386|i686|x86) WA=x86; LA=x86_64 ;;
  *) WA=x64; LA=x86_64 ;;
esac

die() { echo "error: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "'$1' not found in PATH"; }
need curl

api() { # $1 = path ; prints JSON
  local auth=(); [ -n "${GITHUB_TOKEN:-}" ] && auth=(-H "Authorization: Bearer $GITHUB_TOKEN")
  curl -fsSL "${auth[@]}" -H "Accept: application/vnd.github+json" "https://api.github.com/$1"
}
sha() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }
tag_of() { local j; j=$(api "repos/$ORG/$1/releases/latest") || return 1; printf '%s' "$j" | grep -m1 '"tag_name"' | sed -E 's/.*: *"([^"]+)".*/\1/'; }
run() { echo "  \$ $*"; "$@"; }

do_release() { # app mode
  local app=$1 mode=$2 json tag ver want url dest sums
  json=$(api "repos/$ORG/$app/releases/latest") || { echo "  cannot read release info"; return 1; }
  tag=$(printf '%s' "$json" | grep -m1 '"tag_name"' | sed -E 's/.*: *"([^"]+)".*/\1/')
  ver=${tag#v}
  case "$OS:$mode" in
    win:portable)   want="$app-$ver-windows-$WA-portable.zip" ;;
    win:*)          want="$app-$ver-windows-$WA.msi" ;;
    mac:*)          want="$app-$ver-macos-universal.dmg" ;;
    linux:portable) want="$app-$ver-linux-$LA.tar.gz" ;;
    linux:*)
      if   command -v dpkg >/dev/null; then want="$app-$ver-linux-$LA.deb"
      elif command -v rpm  >/dev/null; then want="$app-$ver-linux-$LA.rpm"
      else want="$app-$ver-linux-$LA.AppImage"; fi ;;
  esac
  [ "$OS:$mode" = "mac:portable" ] && want="$app-$ver-macos-universal.dmg"   # no portable archive on macOS
  url=$(printf '%s' "$json" | grep -o '"browser_download_url": *"[^"]*"' | sed -E 's/.*"(https[^"]+)"/\1/' | grep "/$want\$" | head -1)
  [ -n "$url" ] || { echo "  no asset '$want' in $tag"; return 1; }
  echo "  $app $tag: $want"
  mkdir -p "$HOME_DIR/downloads"; dest="$HOME_DIR/downloads/$want"
  curl -fL --progress-bar -o "$dest" "$url" || return 1
  sums=$(printf '%s' "$json" | grep -o '"browser_download_url": *"[^"]*SHA256SUMS.txt"' | sed -E 's/.*"(https[^"]+)"/\1/' | head -1)
  if [ -n "$sums" ]; then
    local expect; expect=$(curl -fsSL "$sums" | grep " \*\?$want\$" | awk '{print $1}')
    if [ -n "$expect" ]; then
      [ "$expect" = "$(sha "$dest")" ] || { rm -f "$dest"; echo "  SHA256 mismatch - download deleted"; return 1; }
      echo "  sha256 OK"
    fi
  fi
  case "$want" in
    *portable.zip) need unzip; rm -rf "$HOME_DIR/portable/$app"; mkdir -p "$HOME_DIR/portable/$app"; unzip -q "$dest" -d "$HOME_DIR/portable/$app"; echo "  unpacked to $HOME_DIR/portable/$app" ;;
    *.tar.gz)      rm -rf "$HOME_DIR/portable/$app"; mkdir -p "$HOME_DIR/portable/$app"; tar -xzf "$dest" -C "$HOME_DIR/portable/$app"; echo "  unpacked to $HOME_DIR/portable/$app" ;;
    *.msi)         run msiexec //i "$(cygpath -w "$dest" 2>/dev/null || echo "$dest")" //passive ;;
    *.dmg)         run open "$dest"; echo "  drag the app into Applications in the window that opened" ;;
    *.deb)         run sudo apt-get install -y "$dest" ;;
    *.rpm)         run sudo rpm -U "$dest" ;;
    *.AppImage)    mkdir -p "$HOME/.local/bin"; cp "$dest" "$HOME/.local/bin/$app"; chmod +x "$HOME/.local/bin/$app"; echo "  installed to $HOME/.local/bin/$app" ;;
  esac
}

do_build() { # app
  local app=$1 src="$HOME_DIR/src/$1" tag pkgs ext=""
  command -v git >/dev/null && command -v cargo >/dev/null || { echo "  build needs git and cargo (https://rustup.rs)"; return 1; }
  tag=${REF:-$(tag_of "$app")}
  [ -n "$tag" ] || { echo "  cannot determine release tag"; return 1; }
  if [ ! -d "$src/.git" ]; then
    mkdir -p "$HOME_DIR/src"
    run git clone --depth 1 --branch "$tag" "https://github.com/$ORG/$app" "$src" || return 1
  else
    run git -C "$src" fetch --depth 1 origin tag "$tag" --no-tags && run git -C "$src" checkout -f "$tag" || return 1
  fi
  if [ "$FONTS" = 1 ]; then
    [ -d "$HOME_DIR/src/craft-fonts" ] || run git clone --depth 1 "https://github.com/$ORG/craft-fonts" "$HOME_DIR/src/craft-fonts" || return 1
    export CRAFT_FONTS_DIR="$HOME_DIR/src/craft-fonts"
  fi
  pkgs=(-p "$app"); [ "$CLI" = 1 ] && pkgs+=(-p "$app-cli")
  (cd "$src" && run cargo build --release "${pkgs[@]}") || return 1
  [ "$OS" = win ] && ext=.exe
  mkdir -p "$HOME_DIR/bin"
  cp "$src/target/release/$app$ext" "$HOME_DIR/bin/"
  [ "$CLI" = 1 ] && cp "$src/target/release/$app-cli$ext" "$HOME_DIR/bin/"
  echo "  built $tag -> $HOME_DIR/bin"
}

# ---- menu (needs a terminal) ----
menu() { # title multi(1|0) items... ; sets MENU_SEL (space separated indexes)
  local title=$1 multi=$2; shift 2
  local items=("$@") n=$# cur=0 i k rest first=1 sel=()
  for ((i=0;i<n;i++)); do sel[i]=0; done
  while :; do
    [ $first = 1 ] || printf '\033[%dA' $((n+2))
    first=0
    printf '\033[2K%s\n' "$title"
    for ((i=0;i<n;i++)); do
      local box mark=" "
      [ $i = $cur ] && mark=">"
      if [ "$multi" = 1 ]; then [ "${sel[i]}" = 1 ] && box="[x]" || box="[ ]"
      else [ $i = $cur ] && box="(*)" || box="( )"; fi
      printf '\033[2K %s %s %s\n' "$mark" "$box" "${items[i]}"
    done
    printf '\033[2K\n'
    IFS= read -rsn1 k
    if [ "$k" = $'\033' ]; then read -rsn2 rest; k=$rest; fi
    case "$k" in
      '[A') cur=$(( (cur+n-1) % n )) ;;
      '[B') cur=$(( (cur+1) % n )) ;;
      ' ') [ "$multi" = 1 ] && sel[cur]=$(( 1 - sel[cur] )) ;;
      a|A) if [ "$multi" = 1 ]; then local v=1; [[ " ${sel[*]} " != *" 0 "* ]] && v=0; for ((i=0;i<n;i++)); do sel[i]=$v; done; fi ;;
      '') break ;;
      q|Q) echo; exit 1 ;;
    esac
  done
  MENU_SEL=""
  if [ "$multi" = 1 ]; then for ((i=0;i<n;i++)); do [ "${sel[i]}" = 1 ] && MENU_SEL+="$i "; done; else MENU_SEL=$cur; fi
}

while [ $# -gt 0 ]; do
  case "$1" in
    -a|--apps) SEL_APPS=$2; shift ;;
    -m|--mode) MODE=$2; shift ;;
    --all) ALL=1 ;;
    --cli) CLI=1 ;;
    --fonts) FONTS=1 ;;
    --ref) REF=$2; shift ;;
    --list) LIST=1 ;;
    -h|--help) sed -n '2,11p' "$0"; exit 0 ;;
    *) die "unknown option $1 (try --help)" ;;
  esac
  shift
done

if [ $LIST = 1 ]; then
  for i in "${!APPS[@]}"; do
    t=$(tag_of "${APPS[i]}")
    printf '%-12s %-8s %s\n' "${APPS[i]}" "${t:-?}" "${DESC[i]}"
  done
  exit 0
fi

[ $ALL = 1 ] && SEL_APPS="${APPS[*]}"
if [ -z "$SEL_APPS" ]; then
  [ -t 0 ] || die "no terminal; pass -a \"app ...\" or --all"
  echo "Craft apps  (up/down move, space tick, a = all, enter confirm, q quit)"; echo
  lines=(); for i in "${!APPS[@]}"; do lines+=("$(printf '%-12s %s' "${APPS[i]}" "${DESC[i]}")"); done
  menu "Apps" 1 "${lines[@]}"
  [ -n "$MENU_SEL" ] || die "nothing selected"
  for i in $MENU_SEL; do SEL_APPS+="${APPS[i]} "; done
fi
if [ -z "$MODE" ]; then
  echo
  menu "Mode" 0 "install   - download the OS package and install it" "portable  - download and unpack, no installer" "build     - clone source + cargo build --release"
  MODE=(install portable build); MODE=${MODE[$MENU_SEL]}
  if [ "$MODE" = build ] && [ $CLI = 0 ] && [ $FONTS = 0 ]; then
    echo
    menu "Build options" 1 "also build the -cli binary" "use craft-fonts (CJK fonts in UI)"
    for i in $MENU_SEL; do [ "$i" = 0 ] && CLI=1; [ "$i" = 1 ] && FONTS=1; done
  fi
fi
case "$MODE" in install|portable|build) ;; *) die "mode must be install, portable or build" ;; esac

failed=""
for a in $SEL_APPS; do
  [[ " ${APPS[*]} " == *" $a "* ]] || { echo "unknown app: $a"; failed+="$a "; continue; }
  echo; echo "== $a ($MODE)"
  if [ "$MODE" = build ]; then do_build "$a" || failed+="$a "; else do_release "$a" "$MODE" || failed+="$a "; fi
done
echo; echo "Done.${failed:+ Failed: $failed}"
[ -z "$failed" ]
