//! Install / update / uninstall logic. No UI code here; progress goes through `Shared`.
use crate::i18n::{self, Lang};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

pub const ORG: &str = "storytold";
/// (repo name, display name, description)
pub const APPS: &[(&str, &str, &str)] = &[
    ("photocraft", "PhotoCraft", "ตัวแก้ไขภาพ"),
    ("vectorcraft", "VectorCraft", "งานเวกเตอร์"),
    ("filmcraft", "FilmCraft", "ตัวตัดต่อวิดีโอ"),
    ("lightcraft", "LightCraft", "คลังภาพ / raw developer"),
    ("pdfcraft", "PdfCraft", "เครื่องมือ PDF"),
    ("effectcraft", "EffectCraft", "motion graphics / VFX"),
    ("designcraft", "DesignCraft", "จัดหน้า / สิ่งพิมพ์"),
    ("wordcraft", "WordCraft", "ประมวลผลคำ (แนว Word)"),
    ("gridcraft", "GridCraft", "สเปรดชีต (แนว Excel)"),
    ("deckcraft", "DeckCraft", "งานนำเสนอ (แนว PowerPoint)"),
    ("cadcraft", "CADCraft", "ออกแบบ / เขียนแบบ CAD"),
    ("soundcraft", "SoundCraft", "ตัดต่อเสียง (แนว Pro Tools)"),
];
pub fn display(app: &str) -> &str {
    APPS.iter().find(|a| a.0 == app).map(|a| a.1).unwrap_or(app)
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Mode {
    Msi,
    Portable,
    Build,
}
impl Mode {
    #[allow(dead_code)]
    pub const ALL: [Mode; 3] = [Mode::Portable, Mode::Msi, Mode::Build];
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Installed {
    pub version: String,
    pub mode: Mode,
    #[serde(default)]
    pub exe: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    /// when non-empty, build mode clones from <dir>/<app> instead of GitHub
    pub local_source_dir: String,
    pub build_cli: bool,
    pub use_fonts: bool,
    pub desktop_shortcut: bool,
    /// UI language code (en, th, zh, ...); empty = detect on first run
    pub lang: String,
    pub self_pinned: bool,
    pub pin_asked: bool,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Store {
    pub installed: BTreeMap<String, Installed>,
    pub settings: Settings,
}

#[derive(Default)]
pub struct Shared {
    pub latest: BTreeMap<String, String>,
    pub log: Vec<String>,
    pub progress: BTreeMap<String, f32>,
    pub busy: bool,
    pub store: Store,
    /// outcome of the last batch, rendered by the UI in the chosen language
    pub summary: Option<Summary>,
    /// per-app outcome of the current batch: (success, error text)
    pub results: BTreeMap<String, (bool, String)>,
}
#[derive(Clone)]
pub struct Summary {
    pub ok: usize,
    pub total: usize,
    pub failed: Vec<(String, String)>,
    pub uninstall: bool,
}
pub fn summary_text(l: Lang, s: &Summary) -> String {
    if s.failed.is_empty() {
        i18n::trf(l, if s.uninstall { "sum_ok_u" } else { "sum_ok_i" }, &[&s.ok.to_string(), &s.total.to_string()])
    } else {
        let list = s.failed.iter().map(|(a, e)| format!("{a} ({e})")).collect::<Vec<_>>().join(", ");
        i18n::trf(l, "sum_fail", &[&s.ok.to_string(), &s.failed.len().to_string(), &list])
    }
}
pub fn lang(sh: &Sh) -> Lang {
    Lang::from_code(&sh.lock().unwrap().store.settings.lang)
}
pub type Sh = Arc<Mutex<Shared>>;

pub fn root() -> PathBuf {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("craft-apps")
}
fn state_file() -> PathBuf {
    root().join("state.json")
}
pub fn load_store() -> Store {
    std::fs::read_to_string(state_file()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
fn save(sh: &Sh) {
    let g = sh.lock().unwrap();
    let _ = std::fs::create_dir_all(root());
    if let Ok(s) = serde_json::to_string_pretty(&g.store) {
        let _ = std::fs::write(state_file(), s);
    }
}
pub fn persist_settings(sh: &Sh) {
    save(sh);
}
pub fn log(sh: &Sh, msg: impl Into<String>) {
    sh.lock().unwrap().log.push(msg.into());
}
fn set_progress(sh: &Sh, app: &str, p: f32) {
    sh.lock().unwrap().progress.insert(app.into(), p);
}

type R<T> = Result<T, String>;
fn e<E: std::fmt::Display>(x: E) -> String {
    x.to_string()
}

fn get(url: &str) -> R<ureq::http::Response<ureq::Body>> {
    let mut req = ureq::get(url).header("User-Agent", "craftget-gui").header("Accept", "application/vnd.github+json");
    if url.contains("api.github.com") {
        if let Ok(t) = std::env::var("GITHUB_TOKEN") {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
    }
    req.call().map_err(e)
}
/// Latest release tag, read from the github.com redirect (the REST API caps anonymous use at 60/hour).
pub fn latest_tag(app: &str) -> R<String> {
    use ureq::ResponseExt;
    let r = ureq::head(&format!("https://github.com/{ORG}/{app}/releases/latest")).header("User-Agent", "craftget-gui").call().map_err(e)?;
    let uri = r.get_uri().to_string();
    uri.split("/releases/tag/").nth(1).map(|t| t.trim_end_matches('/').to_string()).ok_or_else(|| format!("cannot read version ({uri})"))
}
fn asset_url(app: &str, tag: &str, name: &str) -> String {
    format!("https://github.com/{ORG}/{app}/releases/download/{tag}/{name}")
}

/// Refresh the "latest version" column in the background.
pub fn refresh_latest(sh: Sh) {
    std::thread::spawn(move || {
        for (app, _, _) in APPS {
            match latest_tag(app) {
                Ok(t) => {
                    sh.lock().unwrap().latest.insert(app.to_string(), t.trim_start_matches('v').to_string());
                }
                Err(er) => log(&sh, format!("{app}: cannot read latest version ({er})")),
            }
        }
    });
}

fn win_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86" => "x86",
        _ => "x64",
    }
}

fn download(sh: &Sh, app: &str, url: &str, dest: &Path) -> R<()> {
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(e)?;
    let resp = get(url)?;
    let total: u64 = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut rd = resp.into_body().into_reader();
    let mut f = std::fs::File::create(dest).map_err(e)?;
    let (mut buf, mut got) = (vec![0u8; 256 * 1024], 0u64);
    loop {
        let n = rd.read(&mut buf).map_err(e)?;
        if n == 0 {
            break;
        }
        f.write_all(&buf[..n]).map_err(e)?;
        got += n as u64;
        if total > 0 {
            set_progress(sh, app, got as f32 / total as f32 * 0.8);
        }
    }
    Ok(())
}

fn sha256_file(p: &Path) -> R<String> {
    let mut f = std::fs::File::open(p).map_err(e)?;
    let (mut h, mut buf) = (Sha256::new(), vec![0u8; 1 << 20]);
    loop {
        let n = f.read(&mut buf).map_err(e)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn quiet(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

/// Run a child process, streaming stdout+stderr into the log. Returns Err on non-zero exit.
fn run_logged(sh: &Sh, mut cmd: Command) -> R<()> {
    quiet(&mut cmd);
    log(sh, format!("$ {:?}", cmd).replace('"', ""));
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|x| format!("cannot start process: {x}"))?;
    let (out, err) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let sh2 = sh.clone();
    let t = std::thread::spawn(move || {
        for l in BufReader::new(err).lines().map_while(Result::ok) {
            log(&sh2, l);
        }
    });
    for l in BufReader::new(out).lines().map_while(Result::ok) {
        log(sh, l);
    }
    let _ = t.join();
    let st = child.wait().map_err(e)?;
    if st.success() { Ok(()) } else { Err(format!("exit {st}")) }
}

fn exe_name(n: &str) -> String {
    if cfg!(windows) { format!("{n}.exe") } else { n.to_string() }
}

fn find_exe(dir: &Path, app: &str, depth: u32) -> Option<PathBuf> {
    let want = exe_name(app).to_lowercase();
    for ent in std::fs::read_dir(dir).ok()?.flatten() {
        let p = ent.path();
        if p.is_file() && p.file_name()?.to_string_lossy().to_lowercase() == want {
            return Some(p);
        }
    }
    if depth > 0 {
        for ent in std::fs::read_dir(dir).ok()?.flatten() {
            if ent.path().is_dir() {
                if let Some(x) = find_exe(&ent.path(), app, depth - 1) {
                    return Some(x);
                }
            }
        }
    }
    None
}

/// Best effort: where did the .msi put the exe?
fn find_msi_exe(app: &str) -> Option<PathBuf> {
    if !cfg!(windows) {
        return which_path(app);
    }
    let mut bases = vec![];
    for v in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(p) = std::env::var(v) {
            bases.push(PathBuf::from(p));
        }
    }
    if let Ok(p) = std::env::var("LOCALAPPDATA") {
        bases.push(PathBuf::from(p).join("Programs"));
    }
    for b in bases {
        for ent in std::fs::read_dir(&b).ok()?.flatten() {
            let n = ent.path().file_name()?.to_string_lossy().to_lowercase();
            if n.contains(app) || n.contains("storyteller") || n.contains("craft") {
                if let Some(x) = find_exe(&ent.path(), app, 2) {
                    return Some(x);
                }
            }
        }
    }
    None
}

fn powershell(sh: &Sh, script: &str, envs: &[(&str, String)]) -> R<()> {
    let mut c = Command::new("powershell");
    c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]);
    for (k, v) in envs {
        c.env(k, v);
    }
    quiet(&mut c);
    let out = c.output().map_err(e)?;
    if !out.status.success() {
        log(sh, String::from_utf8_lossy(&out.stderr).trim().to_string());
        return Err("powershell failed".into());
    }
    Ok(())
}

pub fn notify(title: &str, msg: &str) {
    if !cfg!(windows) {
        return unix_notify(title, msg);
    }
    let script = "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType=WindowsRuntime] > $null;\
        $t=[Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent('ToastText02');\
        $n=$t.GetElementsByTagName('text');\
        $n.Item(0).AppendChild($t.CreateTextNode($env:CRAFT_T)) > $null;\
        $n.Item(1).AppendChild($t.CreateTextNode($env:CRAFT_M)) > $null;\
        [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe').Show([Windows.UI.Notifications.ToastNotification]::new($t))";
    let mut c = Command::new("powershell");
    c.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script]).env("CRAFT_T", title).env("CRAFT_M", msg);
    quiet(&mut c);
    let _ = c.output();
}

const SHORTCUT_PS: &str = "$w=New-Object -ComObject WScript.Shell;\
    function L($p){$s=$w.CreateShortcut($p);$s.TargetPath=$env:CRAFT_TARGET;$s.WorkingDirectory=$env:CRAFT_WD;$s.Save()};\
    $sm=Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs\\Craft Apps';\
    New-Item -ItemType Directory -Force $sm | Out-Null;\
    L (Join-Path $sm ($env:CRAFT_NAME+'.lnk'));\
    if($env:CRAFT_DESKTOP -eq '1'){L (Join-Path ([Environment]::GetFolderPath('Desktop')) ($env:CRAFT_NAME+'.lnk'))}";
const UNSHORTCUT_PS: &str = "Remove-Item -Force -ErrorAction SilentlyContinue (Join-Path $env:APPDATA ('Microsoft\\Windows\\Start Menu\\Programs\\Craft Apps\\'+$env:CRAFT_NAME+'.lnk')),(Join-Path ([Environment]::GetFolderPath('Desktop')) ($env:CRAFT_NAME+'.lnk'))";

fn make_shortcut(sh: &Sh, name: &str, exe: &Path) {
    if !cfg!(windows) {
        return unix_shortcut(sh, name, exe);
    }
    let desktop = sh.lock().unwrap().store.settings.desktop_shortcut;
    let envs = [
        ("CRAFT_TARGET", exe.to_string_lossy().into_owned()),
        ("CRAFT_WD", exe.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()),
        ("CRAFT_NAME", name.to_string()),
        ("CRAFT_DESKTOP", if desktop { "1" } else { "0" }.into()),
    ];
    match powershell(sh, SHORTCUT_PS, &envs) {
        Ok(()) => log(sh, format!("Start Menu shortcut created: {name}")),
        Err(er) => log(sh, format!("cannot create shortcut: {er}")),
    }
}
fn remove_shortcut(sh: &Sh, name: &str) {
    if !cfg!(windows) {
        return unix_unshortcut(name);
    }
    let _ = powershell(sh, UNSHORTCUT_PS, &[("CRAFT_NAME", name.to_string())]);
}

/// Copy this program to a stable place and add it to the Start Menu.
pub fn pin_self(sh: &Sh) -> R<()> {
    let me = std::env::current_exe().map_err(e)?;
    if cfg!(target_os = "macos") {
        return mac_pin_self(sh, &me);
    }
    let target = root().join(if cfg!(windows) { "CraftInstaller.exe" } else { "CraftInstaller" });
    std::fs::create_dir_all(root()).map_err(e)?;
    if me != target {
        std::fs::copy(&me, &target).map_err(e)?;
    }
    // the Start Menu entry only, never the Desktop, for the installer itself
    let desk = std::mem::replace(&mut sh.lock().unwrap().store.settings.desktop_shortcut, false);
    make_shortcut(sh, "Craft Installer", &target);
    sh.lock().unwrap().store.settings.desktop_shortcut = desk;
    sh.lock().unwrap().store.settings.self_pinned = true;
    save(sh);
    Ok(())
}
pub fn unpin_self(sh: &Sh) {
    if cfg!(target_os = "macos") {
        let _ = std::fs::remove_dir_all(home().join("Applications/Craft Installer.app"));
    }
    remove_shortcut(sh, "Craft Installer");
    sh.lock().unwrap().store.settings.self_pinned = false;
    save(sh);
}

/// Download the release asset for `mode` and verify its checksum. Returns (version, local path).
fn fetch_asset(sh: &Sh, app: &str, mode: Mode) -> R<(String, PathBuf)> {
    let tag = latest_tag(app)?;
    let ver = tag.trim_start_matches('v').to_string();
    let want = asset_name(app, &ver, mode);
    let dest = root().join("downloads").join(&want);
    log(sh, format!("{}: downloading {want}", display(app)));
    download(sh, app, &asset_url(app, &tag, &want), &dest).map_err(|x| format!("cannot download {want}: {x}"))?;
    match get(&asset_url(app, &tag, "SHA256SUMS.txt")).and_then(|mut r| r.body_mut().read_to_string().map_err(e)) {
        Ok(txt) => {
            let expect = txt
                .lines()
                .filter_map(|l| l.split_once(char::is_whitespace))
                .find(|(_, n)| n.trim().trim_start_matches('*') == want)
                .map(|(h, _)| h.to_lowercase());
            if let Some(x) = expect {
                if x != sha256_file(&dest)? {
                    let _ = std::fs::remove_file(&dest);
                    return Err("SHA256 mismatch - download deleted".into());
                }
                log(sh, "sha256 OK");
            }
        }
        Err(er) => log(sh, format!("sha256 check skipped (cannot fetch list: {er})")),
    }
    Ok((ver, dest))
}

fn finish(sh: &Sh, app: &str, version: String, mode: Mode, exe: Option<PathBuf>) {
    sh.lock().unwrap().store.installed.insert(app.into(), Installed { version: version.clone(), mode, exe: exe.map(|p| p.to_string_lossy().into_owned()) });
    save(sh);
    set_progress(sh, app, 1.0);
    log(sh, format!("✔ {} {version} done", display(app)));
}

fn install_portable(sh: &Sh, app: &str) -> R<()> {
    if cfg!(windows) { install_portable_win(sh, app) } else { install_unix(sh, app, Mode::Portable) }
}

fn install_portable_win(sh: &Sh, app: &str) -> R<()> {
    let (ver, zip) = fetch_asset(sh, app, Mode::Portable)?;
    let out = root().join("apps").join(app);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).map_err(e)?;
    let mut c = Command::new("tar");
    c.arg("-xf").arg(&zip).arg("-C").arg(&out);
    run_logged(sh, c)?;
    set_progress(sh, app, 0.9);
    // the zip wraps everything in one folder; lift it so the app sits directly in apps\<app>
    let entries: Vec<_> = std::fs::read_dir(&out).map_err(e)?.flatten().collect();
    if let [only] = entries.as_slice() {
        if only.path().is_dir() {
            for ent in std::fs::read_dir(only.path()).map_err(e)?.flatten() {
                std::fs::rename(ent.path(), out.join(ent.file_name())).map_err(e)?;
            }
            let _ = std::fs::remove_dir(only.path());
        }
    }
    let exe = find_exe(&out, app, 3).ok_or("no .exe found after unpacking")?;
    make_shortcut(sh, display(app), &exe);
    finish(sh, app, ver, Mode::Portable, Some(exe));
    Ok(())
}

fn install_build(sh: &Sh, app: &str) -> R<()> {
    let st = sh.lock().unwrap().store.settings.clone();
    let tag = latest_tag(app)?;
    let src = root().join("src").join(app);
    let local = st.local_source_dir.trim().to_string();
    let origin_of = |name: &str| {
        if local.is_empty() {
            format!("https://github.com/{ORG}/{name}")
        } else {
            PathBuf::from(&local).join(name).to_string_lossy().into_owned()
        }
    };
    std::fs::create_dir_all(root().join("src")).map_err(e)?;
    let git = |args: &[&str], dir: Option<&Path>| {
        let mut c = Command::new("git");
        if let Some(d) = dir {
            c.arg("-C").arg(d);
        }
        c.args(args);
        run_logged(sh, c)
    };
    let origin = origin_of(app);
    if !src.join(".git").exists() {
        git(&["clone", &origin, &src.to_string_lossy()], None)?;
    } else {
        git(&["fetch", "--tags", "--force", &origin], Some(&src))?;
    }
    git(&["checkout", "-f", &tag], Some(&src))?;
    let mut pkgs = vec!["-p".to_string(), app.to_string()];
    if st.build_cli {
        pkgs.extend(["-p".into(), format!("{app}-cli")]);
    }
    let mut c = Command::new("cargo");
    c.current_dir(&src).args(["build", "--release"]).args(&pkgs);
    if st.use_fonts {
        let fonts = root().join("src").join("craft-fonts");
        if !fonts.exists() {
            git(&["clone", "--depth", "1", &origin_of("craft-fonts"), &fonts.to_string_lossy()], None)?;
        }
        c.env("CRAFT_FONTS_DIR", &fonts);
    }
    log(sh, format!("{}: cargo build --release (takes minutes)", display(app)));
    set_progress(sh, app, 0.4);
    run_logged(sh, c)?;
    let bin = root().join("bin");
    std::fs::create_dir_all(&bin).map_err(e)?;
    let mut names = vec![app.to_string()];
    if st.build_cli {
        names.push(format!("{app}-cli"));
    }
    for n in names {
        std::fs::copy(src.join("target/release").join(exe_name(&n)), bin.join(exe_name(&n))).map_err(e)?;
    }
    let exe = bin.join(exe_name(app));
    make_shortcut(sh, display(app), &exe);
    finish(sh, app, tag.trim_start_matches('v').to_string(), Mode::Build, Some(exe));
    Ok(())
}

fn uninstall_local(sh: &Sh, app: &str, inst: &Installed) -> R<()> {
    match inst.mode {
        Mode::Portable if cfg!(target_os = "macos") => remove_installed_path(inst)?,
        Mode::Portable => {
            std::fs::remove_dir_all(root().join("apps").join(app)).map_err(e)?;
            let _ = std::fs::remove_file(home().join(".local/bin").join(app));
        }
        Mode::Build => {
            for n in [app.to_string(), format!("{app}-cli")] {
                let _ = std::fs::remove_file(root().join("bin").join(exe_name(&n)));
            }
        }
        Mode::Msi => remove_installed_path(inst)?, // macOS .app / Linux AppImage (no batch)
    }
    remove_shortcut(sh, display(app));
    sh.lock().unwrap().store.installed.remove(app);
    save(sh);
    log(sh, format!("removed {}", display(app)));
    Ok(())
}

struct MsiOp {
    app: String,
    version: String,
    path: PathBuf,
    install: bool,
}

/// One elevated PowerShell runs every queued .msi, so Windows asks for UAC exactly once.
fn run_msi_batch(sh: &Sh, ops: &[MsiOp]) -> Vec<(String, R<()>)> {
    if !cfg!(windows) {
        return run_pkg_batch_unix(sh, ops);
    }
    let dir = root().join("tmp");
    let _ = std::fs::create_dir_all(&dir);
    let (script, result) = (dir.join("msi-batch.ps1"), dir.join("msi-result.txt"));
    let _ = std::fs::remove_file(&result);
    let q = |s: &str| s.replace('\'', "''");
    let mut ps = format!("$out='{}'\nfunction Run($app,$flag,$path){{$p=Start-Process msiexec -ArgumentList @($flag,('\"'+$path+'\"'),'/qn','/norestart') -Wait -PassThru;Add-Content $out ($app+' '+$p.ExitCode)}}\n", q(&result.to_string_lossy()));
    for o in ops {
        ps += &format!("Run '{}' '{}' '{}'\n", o.app, if o.install { "/i" } else { "/x" }, q(&o.path.to_string_lossy()));
    }
    let _ = std::fs::write(&script, ps);
    log(sh, format!("asking for administrator rights once for {} item(s) - click Yes in the UAC window", ops.len()));
    let launch = "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File',('\"'+$env:CRAFT_SCRIPT+'\"')";
    let launched = powershell(sh, launch, &[("CRAFT_SCRIPT", script.to_string_lossy().into_owned())]);
    let txt = std::fs::read_to_string(&result).unwrap_or_default();
    ops.iter()
        .map(|o| {
            let code = txt.lines().find_map(|l| l.strip_prefix(&format!("{} ", o.app))).and_then(|c| c.trim().parse::<i32>().ok());
            let r = match (code, &launched) {
                (Some(0 | 3010 | 1641), _) => Ok(()),
                (Some(c), _) => Err(format!("msiexec exit code {c}")),
                (None, _) => Err("cancelled (UAC) or the installer did not run".to_string()),
            };
            (o.app.clone(), r)
        })
        .collect()
}

/// Run a batch of jobs in a worker thread. `Some(mode)` = install/update, `None` = uninstall.
pub fn run_jobs(sh: Sh, jobs: Vec<(String, Option<Mode>)>) {
    {
        let mut g = sh.lock().unwrap();
        if g.busy {
            return;
        }
        g.busy = true;
        g.summary = None;
        g.progress.clear();
        g.results.clear();
    }
    std::thread::spawn(move || {
        let (mut ok, mut failed): (Vec<String>, Vec<(String, String)>) = (vec![], vec![]);
        let mut msi: Vec<MsiOp> = vec![];
        let total = jobs.len();
        let mut uninstalling = false;
        for (app, mode) in jobs {
            set_progress(&sh, &app, 0.02);
            let inst = sh.lock().unwrap().store.installed.get(&app).cloned();
            let r: R<()> = match mode {
                Some(Mode::Portable) => install_portable(&sh, &app),
                Some(Mode::Build) => install_build(&sh, &app),
                Some(Mode::Msi) if !batches_pkg() => install_unix(&sh, &app, Mode::Msi),
                Some(Mode::Msi) => fetch_asset(&sh, &app, Mode::Msi).map(|(version, path)| {
                    msi.push(MsiOp { app: app.clone(), version, path, install: true });
                }),
                None => {
                    uninstalling = true;
                    match inst {
                        None => Err("not installed through this program".to_string()),
                        Some(i) if i.mode == Mode::Msi && batches_pkg() => {
                            let f = root().join("downloads").join(asset_name(&app, &i.version, Mode::Msi));
                            if f.exists() {
                                msi.push(MsiOp { app: app.clone(), version: i.version, path: f, install: false });
                                Ok(())
                            } else {
                                Err("original .msi not found - remove it in Windows Settings > Apps".to_string())
                            }
                        }
                        Some(i) => uninstall_local(&sh, &app, &i),
                    }
                }
            };
            let queued = msi.iter().any(|o| o.app == app);
            match r {
                Ok(()) if !queued => {
                    sh.lock().unwrap().results.insert(app.clone(), (true, String::new()));
                    ok.push(app)
                }
                Ok(()) => {}
                Err(er) => {
                    log(&sh, format!("✘ {}: {er}", display(&app)));
                    sh.lock().unwrap().results.insert(app.clone(), (false, er.clone()));
                    failed.push((display(&app).to_string(), er.clone()));
                }
            }
        }
        if !msi.is_empty() {
            for (app, r) in run_msi_batch(&sh, &msi) {
                let op = msi.iter().find(|o| o.app == app).unwrap();
                match r {
                    Ok(()) if op.install => {
                        let exe = find_msi_exe(&app);
                        finish(&sh, &app, op.version.clone(), Mode::Msi, exe);
                        sh.lock().unwrap().results.insert(app.clone(), (true, String::new()));
                        ok.push(app);
                    }
                    Ok(()) => {
                        sh.lock().unwrap().store.installed.remove(&app);
                        save(&sh);
                        log(&sh, format!("removed {}", display(&app)));
                        sh.lock().unwrap().results.insert(app.clone(), (true, String::new()));
                        ok.push(app);
                    }
                    Err(er) => {
                        log(&sh, format!("✘ {}: {er}", display(&app)));
                        sh.lock().unwrap().results.insert(app.clone(), (false, er.clone()));
                        failed.push((display(&app).to_string(), er.clone()));
                    }
                }
            }
        }
        let summary = Summary { ok: ok.len(), total, failed, uninstall: uninstalling };
        let l = lang(&sh);
        notify(i18n::tr(l, if summary.failed.is_empty() { "t_done" } else { "t_err" }), &summary_text(l, &summary));
        let mut g = sh.lock().unwrap();
        g.summary = Some(summary);
        g.busy = false;
    });
}

/// Launch an installed app (detached).
pub fn open_app(sh: &Sh, app: &str) {
    let exe = sh.lock().unwrap().store.installed.get(app).and_then(|i| i.exe.clone());
    match exe {
        Some(p) if Path::new(&p).exists() => {
            let mut c = if cfg!(target_os = "macos") { let mut o = Command::new("open"); o.arg(&p); o } else { Command::new(&p) };
            if let Some(d) = Path::new(&p).parent() {
                c.current_dir(d);
            }
            if let Err(er) = c.spawn() {
                log(sh, format!("cannot open {}: {er}", display(app)));
            }
        }
        _ => log(sh, format!("files of {} not found - try the app menu search", display(app))),
    }
}

pub fn open_folder(sh: &Sh, app: &str) {
    let exe = sh.lock().unwrap().store.installed.get(app).and_then(|i| i.exe.clone());
    if let Some(dir) = exe.as_deref().and_then(|p| Path::new(p).parent()) {
        let _ = Command::new(opener()).arg(dir).spawn();
    }
}

pub fn open_path(p: &Path) {
    let _ = std::fs::create_dir_all(p);
    let _ = Command::new(opener()).arg(p).spawn();
}

/// First-run language guess from the Windows UI culture.
pub fn detect_lang() -> Lang {
    if !cfg!(windows) {
        return unix_detect_lang();
    }
    let mut c = Command::new("powershell");
    c.args(["-NoProfile", "-Command", "(Get-UICulture).TwoLetterISOLanguageName"]);
    quiet(&mut c);
    c.output().ok().map(|o| Lang::from_code(String::from_utf8_lossy(&o.stdout).trim())).unwrap_or(Lang::En)
}


// ======================= macOS / Linux support =======================
// Everything below compiles on every OS (only std APIs); it is simply never reached on Windows.

fn opener() -> &'static str {
    if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" }
}
fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into()))
}
fn which(cmd: &str) -> bool {
    Command::new("sh").args(["-c", &format!("command -v {cmd}")]).output().map(|o| o.status.success()).unwrap_or(false)
}
fn which_path(cmd: &str) -> Option<PathBuf> {
    let o = Command::new("sh").args(["-c", &format!("command -v {cmd}")]).output().ok()?;
    let t = String::from_utf8_lossy(&o.stdout).trim().to_string();
    (o.status.success() && !t.is_empty()).then(|| PathBuf::from(t))
}
fn unix_arch() -> &'static str {
    if std::env::consts::ARCH == "aarch64" { "aarch64" } else { "x86_64" }
}

/// Release asset to download for this OS / arch / mode.
fn asset_name(app: &str, ver: &str, mode: Mode) -> String {
    if cfg!(windows) {
        match mode {
            Mode::Portable => format!("{app}-{ver}-windows-{}-portable.zip", win_arch()),
            _ => format!("{app}-{ver}-windows-{}.msi", win_arch()),
        }
    } else if cfg!(target_os = "macos") {
        format!("{app}-{ver}-macos-universal.dmg")
    } else {
        let a = unix_arch();
        match mode {
            Mode::Portable => format!("{app}-{ver}-linux-{a}.tar.gz"),
            _ if which("dpkg") => format!("{app}-{ver}-linux-{a}.deb"),
            _ if which("rpm") => format!("{app}-{ver}-linux-{a}.rpm"),
            _ => format!("{app}-{ver}-linux-{a}.AppImage"),
        }
    }
}

/// True when the system installer needs one elevated batch (Windows .msi, Linux .deb/.rpm).
fn batches_pkg() -> bool {
    cfg!(windows) || (cfg!(target_os = "linux") && (which("dpkg") || which("rpm")))
}

#[cfg(unix)]
fn make_exec(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(m) = std::fs::metadata(p) {
        let mut perm = m.permissions();
        perm.set_mode(0o755);
        let _ = std::fs::set_permissions(p, perm);
    }
}
#[cfg(not(unix))]
fn make_exec(_: &Path) {}
#[cfg(unix)]
fn symlink(target: &Path, link: &Path) {
    let _ = std::fs::remove_file(link);
    let _ = std::os::unix::fs::symlink(target, link);
}
#[cfg(not(unix))]
fn symlink(_: &Path, _: &Path) {}

fn slug(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// Linux: a .desktop entry (+ icon) so the app shows up in the menu; macOS needs nothing (the .app is indexed).
fn unix_shortcut(sh: &Sh, name: &str, exe: &Path) {
    if cfg!(target_os = "macos") {
        return;
    }
    let s = slug(name);
    let icon_dir = home().join(".local/share/icons/hicolor/128x128/apps");
    let apps_dir = home().join(".local/share/applications");
    let _ = std::fs::create_dir_all(&icon_dir);
    let _ = std::fs::create_dir_all(&apps_dir);
    let bytes: &[u8] = APPS.iter().find(|a| a.1 == name).map(|a| crate::icon_bytes(a.0)).unwrap_or(include_bytes!("../assets/icon.png"));
    let _ = std::fs::write(icon_dir.join(format!("craft-{s}.png")), bytes);
    let entry = format!("[Desktop Entry]\nType=Application\nName={name}\nExec=\"{}\"\nIcon=craft-{s}\nTerminal=false\nCategories=Utility;\n", exe.display());
    let file = apps_dir.join(format!("craft-{s}.desktop"));
    if std::fs::write(&file, &entry).is_ok() {
        make_exec(&file);
        log(sh, format!("menu entry created: {name}"));
        if sh.lock().unwrap().store.settings.desktop_shortcut && name != "Craft Installer" {
            let d = home().join("Desktop");
            if d.is_dir() {
                let _ = std::fs::write(d.join(format!("{name}.desktop")), &entry);
                make_exec(&d.join(format!("{name}.desktop")));
            }
        }
    }
}
fn unix_unshortcut(name: &str) {
    let s = slug(name);
    let _ = std::fs::remove_file(home().join(".local/share/applications").join(format!("craft-{s}.desktop")));
    let _ = std::fs::remove_file(home().join(".local/share/icons/hicolor/128x128/apps").join(format!("craft-{s}.png")));
    let _ = std::fs::remove_file(home().join("Desktop").join(format!("{name}.desktop")));
}

fn unix_notify(title: &str, msg: &str) {
    if cfg!(target_os = "macos") {
        let q = |t: &str| t.replace('\\', "\\\\").replace('"', "\\\"");
        let _ = Command::new("osascript").args(["-e", &format!("display notification \"{}\" with title \"{}\"", q(msg), q(title))]).output();
    } else if which("notify-send") {
        let _ = Command::new("notify-send").args([title, msg]).output();
    }
}

fn unix_detect_lang() -> Lang {
    let from_env = ["LC_ALL", "LC_MESSAGES", "LANG"].iter().filter_map(|k| std::env::var(k).ok()).find(|v| !v.is_empty() && v != "C" && v != "POSIX");
    let code = if cfg!(target_os = "macos") {
        Command::new("defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).split('"').nth(1).map(|s| s.to_string()))
            .or(from_env)
    } else {
        from_env
    };
    code.map(|c| Lang::from_code(&c.chars().take(2).collect::<String>())).unwrap_or(Lang::En)
}

fn mac_pin_self(sh: &Sh, me: &Path) -> R<()> {
    let app = home().join("Applications/Craft Installer.app");
    let bin_dir = app.join("Contents/MacOS");
    std::fs::create_dir_all(&bin_dir).map_err(e)?;
    let exe = bin_dir.join("CraftInstaller");
    std::fs::copy(me, &exe).map_err(e)?;
    make_exec(&exe);
    let plist = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\"><dict><key>CFBundleName</key><string>Craft Installer</string><key>CFBundleExecutable</key><string>CraftInstaller</string><key>CFBundleIdentifier</key><string>local.craftinstaller</string><key>CFBundlePackageType</key><string>APPL</string><key>NSHighResolutionCapable</key><true/></dict></plist>\n";
    std::fs::write(app.join("Contents/Info.plist"), plist).map_err(e)?;
    sh.lock().unwrap().store.settings.self_pinned = true;
    save(sh);
    log(sh, "Craft Installer.app created in ~/Applications");
    Ok(())
}

/// macOS (.dmg -> .app) and Linux (.tar.gz or AppImage) installs that need no administrator batch.
fn install_unix(sh: &Sh, app: &str, mode: Mode) -> R<()> {
    let (ver, file) = fetch_asset(sh, app, mode)?;
    set_progress(sh, app, 0.85);
    if cfg!(target_os = "macos") {
        let mnt = root().join("tmp").join(format!("mnt-{app}"));
        std::fs::create_dir_all(&mnt).map_err(e)?;
        let mut a = Command::new("hdiutil");
        a.args(["attach", "-nobrowse", "-readonly", "-mountpoint"]).arg(&mnt).arg(&file);
        run_logged(sh, a)?;
        let res = (|| -> R<PathBuf> {
            let bundle = std::fs::read_dir(&mnt).map_err(e)?.flatten().map(|x| x.path()).find(|p| p.extension().map(|x| x == "app").unwrap_or(false)).ok_or("no .app inside the .dmg")?;
            let name = bundle.file_name().unwrap().to_owned();
            let mut dests = vec![];
            if mode == Mode::Msi {
                dests.push(PathBuf::from("/Applications"));
            }
            dests.push(home().join("Applications"));
            for d in dests {
                let _ = std::fs::create_dir_all(&d);
                let target = d.join(&name);
                let _ = std::fs::remove_dir_all(&target);
                let mut c = Command::new("cp");
                c.arg("-R").arg(&bundle).arg(&target);
                if run_logged(sh, c).is_ok() {
                    return Ok(target);
                }
            }
            Err("cannot copy the .app (permissions?)".to_string())
        })();
        let _ = Command::new("hdiutil").arg("detach").arg("-force").arg(&mnt).output();
        finish(sh, app, ver, mode, Some(res?));
        return Ok(());
    }
    // Linux
    let exe = if file.to_string_lossy().ends_with(".tar.gz") {
        let out = root().join("apps").join(app);
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).map_err(e)?;
        let mut c = Command::new("tar");
        c.arg("-xzf").arg(&file).arg("-C").arg(&out);
        run_logged(sh, c)?;
        let entries: Vec<_> = std::fs::read_dir(&out).map_err(e)?.flatten().collect();
        if let [only] = entries.as_slice() {
            if only.path().is_dir() {
                for ent in std::fs::read_dir(only.path()).map_err(e)?.flatten() {
                    std::fs::rename(ent.path(), out.join(ent.file_name())).map_err(e)?;
                }
                let _ = std::fs::remove_dir(only.path());
            }
        }
        let exe = find_exe(&out, app, 3).ok_or("no executable found after unpacking")?;
        make_exec(&exe);
        exe
    } else {
        let exe = home().join(".local/bin").join(app); // AppImage fallback
        std::fs::create_dir_all(exe.parent().unwrap()).map_err(e)?;
        std::fs::copy(&file, &exe).map_err(e)?;
        make_exec(&exe);
        exe
    };
    let bin = home().join(".local/bin");
    let _ = std::fs::create_dir_all(&bin);
    if exe != bin.join(app) {
        symlink(&exe, &bin.join(app));
    }
    make_shortcut(sh, display(app), &exe);
    finish(sh, app, ver, mode, Some(exe));
    Ok(())
}

fn remove_installed_path(inst: &Installed) -> R<()> {
    let p = PathBuf::from(inst.exe.clone().ok_or("install location unknown")?);
    if p.is_dir() { std::fs::remove_dir_all(&p).map_err(e) } else { std::fs::remove_file(&p).map_err(e) }
}

/// Linux: one pkexec (polkit) prompt runs apt/rpm for every queued package.
fn run_pkg_batch_unix(sh: &Sh, ops: &[MsiOp]) -> Vec<(String, R<()>)> {
    let dir = root().join("tmp");
    let _ = std::fs::create_dir_all(&dir);
    let (script, result) = (dir.join("pkg-batch.sh"), dir.join("pkg-result.txt"));
    let _ = std::fs::remove_file(&result);
    let mut sc = String::from("#!/bin/sh\n");
    for o in ops {
        let p = o.path.to_string_lossy().to_string();
        let deb = p.ends_with(".deb");
        let cmd = match (o.install, deb) {
            (true, true) => format!("apt-get install -y '{p}'"),
            (true, false) => format!("rpm -U --replacepkgs '{p}'"),
            (false, true) => format!("apt-get remove -y {}", o.app),
            (false, false) => format!("rpm -e {}", o.app),
        };
        sc += &format!("{cmd}; echo \"{} $?\" >> '{}'\n", o.app, result.display());
    }
    let _ = std::fs::write(&script, sc);
    log(sh, format!("asking for administrator rights once for {} item(s) - approve the prompt", ops.len()));
    let mut c = Command::new("pkexec");
    c.arg("sh").arg(&script);
    let launched = run_logged(sh, c);
    let txt = std::fs::read_to_string(&result).unwrap_or_default();
    ops.iter()
        .map(|o| {
            let code = txt.lines().find_map(|l| l.strip_prefix(&format!("{} ", o.app))).and_then(|c| c.trim().parse::<i32>().ok());
            let r = match (code, &launched) {
                (Some(0), _) => Ok(()),
                (Some(c), _) => Err(format!("package manager exit code {c}")),
                (None, _) => Err("cancelled (administrator prompt) or the installer did not run".to_string()),
            };
            (o.app.clone(), r)
        })
        .collect()
}
