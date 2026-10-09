#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod core;
mod i18n;
use core::{Mode, Sh, Shared, APPS};
use eframe::egui::{self, Color32, RichText};
use i18n::{tr, trf, Lang};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

const GREEN: Color32 = Color32::from_rgb(88, 190, 120);
const AMBER: Color32 = Color32::from_rgb(235, 170, 60);
const RED: Color32 = Color32::from_rgb(235, 100, 100);
const BLUE: Color32 = Color32::from_rgb(40, 100, 200);
const CARD: Color32 = Color32::from_rgb(33, 35, 40);
const CARD_ON: Color32 = Color32::from_rgb(28, 46, 78);
const LINE: Color32 = Color32::from_rgb(52, 55, 62);

const GROUPS: &[(&str, &[&str])] = &[
    ("g_img", &["photocraft", "vectorcraft", "lightcraft", "designcraft", "cadcraft"]),
    ("g_av", &["filmcraft", "effectcraft", "soundcraft"]),
    ("g_doc", &["wordcraft", "gridcraft", "deckcraft", "pdfcraft"]),
];

macro_rules! icons {
    ($($n:literal),*) => {
        fn icon_bytes(app: &str) -> &'static [u8] {
            match app { $($n => include_bytes!(concat!("../assets/", $n, ".png")),)* _ => &[] }
        }
    };
}
icons!("photocraft", "vectorcraft", "filmcraft", "lightcraft", "pdfcraft", "effectcraft", "designcraft", "wordcraft", "gridcraft", "deckcraft", "cadcraft", "soundcraft");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Apps,
    Method,
    Run,
}

struct App {
    sh: Sh,
    lang: Lang,
    step: Step,
    pick: BTreeMap<String, bool>,
    mode: Mode,
    uninstalling: bool,
    was_busy: bool,
}

fn add_font(f: &mut egui::FontDefinitions, name: &str, path: &str) -> bool {
    let Ok(bytes) = std::fs::read(path) else { return false };
    f.font_data.insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
    for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        f.families.get_mut(&fam).unwrap().push(name.into());
    }
    true
}

/// Font files that can draw one script, per OS (first existing one wins).
fn font_candidates(tag: &str) -> Vec<String> {
    let list = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    if cfg!(windows) {
        let w = |f: &str| format!("C:\\Windows\\Fonts\\{f}");
        match tag {
            "th" => vec![w("tahoma.ttf"), w("LeelawUI.ttf")],
            "zh" => vec![w("msyh.ttc")],
            "ja" => vec![w("YuGothM.ttc"), w("meiryo.ttc")],
            "ko" => vec![w("malgun.ttf")],
            _ => vec![],
        }
    } else if cfg!(target_os = "macos") {
        match tag {
            "th" => list(&["/System/Library/Fonts/Supplemental/Tahoma.ttf", "/System/Library/Fonts/Thonburi.ttc", "/System/Library/Fonts/Supplemental/Ayuthaya.ttf"]),
            "zh" => list(&["/System/Library/Fonts/PingFang.ttc", "/System/Library/Fonts/STHeiti Medium.ttc"]),
            "ja" => list(&["/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", "/System/Library/Fonts/Hiragino Sans GB.ttc"]),
            "ko" => list(&["/System/Library/Fonts/AppleSDGothicNeo.ttc"]),
            _ => vec![],
        }
    } else {
        // Linux: ask fontconfig which file covers the script
        let q = match tag { "zh" => "zh-cn", t => t };
        std::process::Command::new("fc-match")
            .args(["-f", "%{file}", &format!(":lang={q}")])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|f| !f.is_empty())
            .into_iter()
            .collect()
    }
}

/// egui's built-in font lacks Thai/CJK, so borrow system fonts. All scripts are loaded (the language
/// picker lists every language in its own script); the selected language's CJK font goes first so
/// shared Han characters use the right glyph style.
fn apply_fonts(ctx: &egui::Context, lang: Lang) {
    let mut f = egui::FontDefinitions::default();
    let mut order = vec!["th"];
    let first = match lang { Lang::Zh => Some("zh"), Lang::Ja => Some("ja"), Lang::Ko => Some("ko"), _ => None };
    if let Some(t) = first {
        order.push(t);
    }
    for t in ["zh", "ja", "ko"] {
        if Some(t) != first {
            order.push(t);
        }
    }
    for tag in order {
        for path in font_candidates(tag) {
            if add_font(&mut f, tag, &path) {
                break;
            }
        }
    }
    ctx.set_fonts(f);
}

fn setup_style(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
        s.spacing.button_padding = egui::vec2(14.0, 7.0);
        s.animation_time = 0.0; // no easing: nothing slides or breathes
        for (_, f) in s.text_styles.iter_mut() {
            f.size *= 1.12;
        }
        if let Some(f) = s.text_styles.get_mut(&egui::TextStyle::Small) {
            f.size = 13.5;
        }
        let v = &mut s.visuals;
        v.panel_fill = Color32::from_rgb(22, 23, 27);
        v.selection.bg_fill = BLUE;
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, LINE);
        // flat controls: colour change on hover only, never size or border changes
        for (w, fill) in [(&mut v.widgets.inactive, 46u8), (&mut v.widgets.hovered, 62), (&mut v.widgets.active, 74)] {
            w.weak_bg_fill = Color32::from_gray(fill);
            w.bg_fill = Color32::from_gray(fill);
            w.bg_stroke = egui::Stroke::NONE;
            w.fg_stroke.width = 1.0;
            w.expansion = 0.0;
            w.corner_radius = 8.0.into();
        }
    });
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let sh: Sh = Arc::new(Mutex::new(Shared { store: core::load_store(), ..Default::default() }));
        if sh.lock().unwrap().store.settings.lang.is_empty() {
            let l = core::detect_lang();
            sh.lock().unwrap().store.settings.lang = l.code().to_string();
            core::persist_settings(&sh);
        }
        let lang = core::lang(&sh);
        setup_style(&cc.egui_ctx);
        apply_fonts(&cc.egui_ctx, lang);
        core::refresh_latest(sh.clone());
        Self { sh, lang, step: Step::Apps, pick: BTreeMap::new(), mode: Mode::Portable, uninstalling: false, was_busy: false }
    }

    fn t(&self, k: &str) -> &'static str {
        tr(self.lang, k)
    }

    fn selected(&self) -> Vec<String> {
        APPS.iter().filter(|a| self.pick.get(a.0).copied().unwrap_or(false)).map(|a| a.0.to_string()).collect()
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("Craft Installer");
            ui.add_space(14.0);
            let steps = [(Step::Apps, "step1"), (Step::Method, "step2"), (Step::Run, "step3")];
            for (i, (s, key)) in steps.iter().enumerate() {
                let (cur, past) = (*s == self.step, (*s as u8) < self.step as u8);
                let col = if cur { BLUE } else if past { GREEN } else { ui.visuals().weak_text_color() };
                let label = format!("{}  {}", i + 1, self.t(key));
                ui.label(RichText::new(if past { format!("✔ {label}") } else { label }).color(col).strong());
                if i < 2 {
                    ui.label("›");
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let before = self.lang;
                egui::ComboBox::from_id_salt("lang").width(110.0).selected_text(self.lang.name()).show_ui(ui, |ui| {
                    for l in Lang::ALL {
                        ui.selectable_value(&mut self.lang, l, l.name());
                    }
                });
                if self.lang != before {
                    apply_fonts(ui.ctx(), self.lang);
                    self.sh.lock().unwrap().store.settings.lang = self.lang.code().to_string();
                    core::persist_settings(&self.sh);
                }
                let pinned = self.sh.lock().unwrap().store.settings.self_pinned;
                if pinned {
                    if ui.small_button(self.t("pin_rm")).clicked() {
                        core::unpin_self(&self.sh);
                    }
                } else if ui.small_button(self.t("pin_btn")).clicked() {
                    let sh = self.sh.clone();
                    std::thread::spawn(move || {
                        let _ = core::pin_self(&sh);
                    });
                }
            });
        });
        ui.separator();
    }

    // ---------------- step 1 ----------------
    fn pin_card(&mut self, ui: &mut egui::Ui) {
        let (pinned, asked) = {
            let g = self.sh.lock().unwrap();
            (g.store.settings.self_pinned, g.store.settings.pin_asked)
        };
        if pinned || asked {
            return;
        }
        egui::Frame::new().fill(CARD_ON).stroke(egui::Stroke::new(1.0, BLUE)).corner_radius(10.0).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(self.t("pin_t")).strong());
            ui.small(self.t("pin_b"));
            ui.horizontal(|ui| {
                if ui.button(self.t("pin_btn")).clicked() {
                    let sh = self.sh.clone();
                    std::thread::spawn(move || {
                        let _ = core::pin_self(&sh);
                    });
                }
                if ui.button(self.t("pin_no")).clicked() {
                    self.sh.lock().unwrap().store.settings.pin_asked = true;
                    core::persist_settings(&self.sh);
                }
            });
        });
    }

    fn step_apps(&mut self, ui: &mut egui::Ui, busy: bool) {
        let l = self.lang;
        let (installed, latest) = {
            let g = self.sh.lock().unwrap();
            (g.store.installed.clone(), g.latest.clone())
        };
        self.pin_card(ui);
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.t("pick_hint")).strong());
            if ui.small_button(self.t("all")).clicked() {
                for a in APPS {
                    self.pick.insert(a.0.to_string(), true);
                }
            }
            if ui.small_button(self.t("only_new")).clicked() {
                for a in APPS {
                    let up = matches!((installed.get(a.0), latest.get(a.0)), (Some(i), Some(v)) if &i.version != v);
                    self.pick.insert(a.0.to_string(), up);
                }
            }
            if ui.small_button(self.t("clear")).clicked() {
                self.pick.clear();
            }
            if ui.small_button(self.t("refresh")).clicked() {
                core::refresh_latest(self.sh.clone());
            }
        });
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            for (title, ids) in GROUPS {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(tr(l, title)).strong().size(16.0));
                    if ui.small_button(tr(l, "pick_group")).clicked() {
                        let all = ids.iter().all(|i| self.pick.get(*i).copied().unwrap_or(false));
                        for i in ids.iter() {
                            self.pick.insert(i.to_string(), !all);
                        }
                    }
                });
                for chunk in ids.chunks(5) {
                    let cw = ((ui.available_width() - ui.spacing().item_spacing.x * 4.0) / 5.0).floor();
                    ui.horizontal_top(|ui| {
                        for app in chunk.iter() {
                            let (_, name, _) = APPS.iter().find(|a| a.0 == *app).unwrap();
                            ui.allocate_ui_with_layout(egui::vec2(cw, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
                            let on = self.pick.get(*app).copied().unwrap_or(false);
                            // constant border width, only colours change -> layout never shifts on select/hover
                            egui::Frame::new()
                                .fill(if on { CARD_ON } else { CARD })
                                .stroke(egui::Stroke::new(1.0, if on { BLUE } else { LINE }))
                                .corner_radius(10.0)
                                .inner_margin(egui::Margin::same(10))
                                .show(ui, |ui| {
                                    ui.set_width(cw - 22.0);
                                    ui.set_min_height(122.0);
                                    ui.horizontal(|ui| {
                                        ui.add(egui::Image::from_bytes(format!("bytes://{app}.png"), icon_bytes(app)).fit_to_exact_size(egui::vec2(44.0, 44.0)).corner_radius(10.0));
                                        ui.vertical(|ui| {
                                            ui.checkbox(self.pick.entry(app.to_string()).or_default(), RichText::new(*name).strong().size(15.0));
                                            ui.small(tr(l, &format!("d_{app}")));
                                        });
                                    });
                                    let inst = installed.get(*app);
                                    match (inst, latest.get(*app)) {
                                        (Some(i), Some(v)) if &i.version != v => {
                                            ui.colored_label(AMBER, trf(l, "upd", &[v]));
                                        }
                                        (Some(i), _) => {
                                            ui.colored_label(GREEN, trf(l, "inst", &[&i.version]));
                                        }
                                        (None, v) => {
                                            ui.colored_label(ui.visuals().weak_text_color(), trf(l, "not_inst", &[v.map(String::as_str).unwrap_or("")]));
                                        }
                                    }
                                    if inst.is_some() {
                                        ui.horizontal(|ui| {
                                            ui.add_enabled_ui(!busy, |ui| {
                                                if ui.small_button(tr(l, "open")).clicked() {
                                                    core::open_app(&self.sh, app);
                                                }
                                                if ui.small_button(tr(l, "folder")).clicked() {
                                                    core::open_folder(&self.sh, app);
                                                }
                                            });
                                        });
                                    }
                                });
                            });
                        }
                    });
                }
            }
        });
    }

    // ---------------- step 2 ----------------
    fn method_card(&mut self, ui: &mut egui::Ui, m: Mode, title: &str, body: &str, badge: Option<&str>) {
        let on = self.mode == m;
        let resp = egui::Frame::new()
            .fill(if on { CARD_ON } else { CARD })
            .inner_margin(egui::Margin::symmetric(14, 10))
            .corner_radius(10.0)
            .stroke(egui::Stroke::new(1.0, if on { BLUE } else { LINE }))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(if on { "◉" } else { "○" }).color(if on { BLUE } else { ui.visuals().weak_text_color() }).size(18.0));
                    ui.label(RichText::new(title).strong().size(16.0));
                    if let Some(b) = badge {
                        ui.colored_label(GREEN, b);
                    }
                });
                ui.small(body);
            })
            .response
            .interact(egui::Sense::click());
        if resp.clicked() {
            self.mode = m;
        }
    }

    fn step_method(&mut self, ui: &mut egui::Ui) {
        let sel = self.selected();
        let names = sel.iter().map(|a| core::display(a)).collect::<Vec<_>>().join(", ");
        ui.label(RichText::new(trf(self.lang, "will", &[&sel.len().to_string(), &names])).strong());
        ui.add_space(4.0);
        self.method_card(ui, Mode::Portable, self.t("m_fast_t"), self.t("m_fast_b"), Some(self.t("recommended")));
        self.method_card(ui, Mode::Msi, self.t("m_msi_t"), self.t("m_msi_b"), None);
        self.method_card(ui, Mode::Build, self.t("m_build_t"), self.t("m_build_b"), None);
        ui.add_space(4.0);
        ui.label(RichText::new(self.t("opts_for")).strong());
        let l = self.lang;
        let mut g = self.sh.lock().unwrap();
        let s = &mut g.store.settings;
        let mut changed = false;
        match self.mode {
            Mode::Portable => {
                changed |= ui.checkbox(&mut s.desktop_shortcut, tr(l, "desk")).changed();
            }
            Mode::Msi => {
                ui.small(tr(l, "no_extra"));
            }
            Mode::Build => {
                changed |= ui.checkbox(&mut s.build_cli, tr(l, "cli")).changed();
                changed |= ui.checkbox(&mut s.use_fonts, tr(l, "fonts")).changed();
                changed |= ui.checkbox(&mut s.desktop_shortcut, tr(l, "desk")).changed();
                ui.horizontal(|ui| {
                    ui.label(tr(l, "local_src"));
                    changed |= ui.text_edit_singleline(&mut s.local_source_dir).changed();
                });
                ui.small(tr(l, "local_hint"));
            }
        }
        drop(g);
        if changed {
            core::persist_settings(&self.sh);
        }
    }

    // ---------------- step 3 ----------------
    fn step_run(&mut self, ui: &mut egui::Ui, busy: bool) {
        let l = self.lang;
        let (summary, results, progress, log) = {
            let g = self.sh.lock().unwrap();
            (g.summary.clone(), g.results.clone(), g.progress.clone(), g.log.clone())
        };
        if busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new(tr(l, if self.uninstalling { "working_u" } else { "working_i" })).strong());
                if self.mode == Mode::Msi && !self.uninstalling {
                    ui.small(tr(l, "uac"));
                }
            });
        } else if let Some(sm) = &summary {
            let good = sm.failed.is_empty();
            let col = if good { GREEN } else { RED };
            egui::Frame::new().fill(col.gamma_multiply(0.18)).stroke(egui::Stroke::new(1.0, col)).corner_radius(8.0).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(col, if good { "✔" } else { "✘" });
                    ui.label(core::summary_text(l, sm));
                });
            });
        }
        for app in self.selected() {
            ui.horizontal(|ui| {
                ui.set_min_height(26.0);
                ui.add_sized([120.0, 20.0], egui::Label::new(RichText::new(core::display(&app)).strong()));
                match results.get(&app) {
                    Some((true, _)) => {
                        ui.colored_label(GREEN, tr(l, if self.uninstalling { "done_u" } else { "done_i" }));
                        if !self.uninstalling && ui.small_button(tr(l, "open")).clicked() {
                            core::open_app(&self.sh, &app);
                        }
                        if !self.uninstalling && ui.small_button(tr(l, "folder")).clicked() {
                            core::open_folder(&self.sh, &app);
                        }
                    }
                    Some((false, er)) => {
                        ui.colored_label(RED, format!("✘ {er}"));
                    }
                    None => {
                        let p = progress.get(&app).copied().unwrap_or(0.0);
                        if p > 0.0 {
                            ui.add(egui::ProgressBar::new(p).desired_width(260.0));
                        } else {
                            ui.colored_label(ui.visuals().weak_text_color(), tr(l, "queued"));
                        }
                    }
                }
            });
        }
        ui.add_space(6.0);
        if busy {
            if let Some(last) = log.last() {
                ui.small(last);
            }
        }
        egui::CollapsingHeader::new(trf(l, "log", &[&log.len().to_string()])).id_salt("log").show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(150.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                for line in &log {
                    ui.monospace(line);
                }
            });
        });
    }

    fn start(&mut self, uninstall: bool) {
        self.uninstalling = uninstall;
        let jobs = self.selected().into_iter().map(|a| (a, if uninstall { None } else { Some(self.mode) })).collect();
        core::run_jobs(self.sh.clone(), jobs);
        self.step = Step::Run;
    }

    fn footer(&mut self, ui: &mut egui::Ui, busy: bool) {
        let n = self.selected().len();
        let l = self.lang;
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            match self.step {
                Step::Apps => {
                    ui.add_enabled_ui(n > 0, |ui| {
                        if ui.button(tr(l, "uninstall_sel")).clicked() {
                            self.start(true);
                        }
                    });
                }
                Step::Method => {
                    if ui.button(tr(l, "back")).clicked() {
                        self.step = Step::Apps;
                    }
                }
                Step::Run => {
                    ui.add_enabled_ui(!busy, |ui| {
                        if ui.button(tr(l, "more")).clicked() {
                            self.step = Step::Apps;
                            self.pick.clear();
                        }
                    });
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let big = |t: String| egui::Button::new(RichText::new(t).strong().size(16.0)).fill(BLUE).min_size(egui::vec2(240.0, 38.0));
                match self.step {
                    Step::Apps => {
                        ui.add_enabled_ui(n > 0, |ui| {
                            let label = if n > 0 { trf(l, "next", &[&n.to_string()]) } else { tr(l, "pick_first").to_string() };
                            if ui.add(big(label)).clicked() {
                                self.step = Step::Method;
                            }
                        });
                    }
                    Step::Method => {
                        if ui.add(big(trf(l, "start", &[&n.to_string()]))).clicked() {
                            self.start(false);
                        }
                    }
                    Step::Run => {
                        ui.add_enabled_ui(!busy, |ui| {
                            if ui.add(big(tr(l, "finish").to_string())).clicked() {
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    }
                }
            });
        });
        ui.add_space(6.0);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let busy = self.sh.lock().unwrap().busy;
        if self.was_busy && !busy {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Informational));
        }
        self.was_busy = busy;

        egui::Panel::bottom("nav").show(ui, |ui| self.footer(ui, busy));
        egui::CentralPanel::default().show(ui, |ui| {
            self.header(ui);
            match self.step {
                Step::Apps => self.step_apps(ui, busy),
                Step::Method => self.step_method(ui),
                Step::Run => self.step_run(ui, busy),
            }
        });
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(if busy { 200 } else { 700 }));
    }
}

/// Headless mode for automation:  CraftInstaller.exe --install portable|msi|build all|app1 app2 ...
///                                CraftInstaller.exe --uninstall all|app1 ...
/// Output goes to %USERPROFILE%\craft-apps\last-run.log (the GUI build has no console).
fn headless(args: &[String]) -> bool {
    if args.first().map(String::as_str) == Some("--pin") {
        let sh: Sh = Arc::new(Mutex::new(Shared { store: core::load_store(), ..Default::default() }));
        let r = core::pin_self(&sh);
        let _ = std::fs::write(core::root().join("last-run.log"), format!("{:?}
{}", r, sh.lock().unwrap().log.join("
")));
        return true;
    }
    let (uninstall, rest) = match args.first().map(String::as_str) {
        Some("--install") => (false, &args[1..]),
        Some("--uninstall") => (true, &args[1..]),
        _ => return false,
    };
    let (mode, names) = if uninstall {
        (None, rest)
    } else {
        let m = match rest.first().map(String::as_str) {
            Some("msi") => Mode::Msi,
            Some("build") => Mode::Build,
            _ => Mode::Portable,
        };
        (Some(m), &rest[1..])
    };
    let apps: Vec<String> = if names.iter().any(|n| n == "all") { APPS.iter().map(|a| a.0.to_string()).collect() } else { names.to_vec() };
    let sh: Sh = Arc::new(Mutex::new(Shared { store: core::load_store(), ..Default::default() }));
    core::run_jobs(sh.clone(), apps.into_iter().map(|a| (a, mode)).collect());
    loop {
        std::thread::sleep(std::time::Duration::from_millis(300));
        if !sh.lock().unwrap().busy {
            break;
        }
    }
    let g = sh.lock().unwrap();
    let mut out = g.log.join("\n");
    if let Some(sm) = &g.summary {
        out += &format!("\n{} {}\n", if sm.failed.is_empty() { "OK" } else { "FAIL" }, core::summary_text(Lang::En, sm));
    }
    let _ = std::fs::write(core::root().join("last-run.log"), out);
    true
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if headless(&args) {
        return Ok(());
    }
    let icon = image::load_from_memory(include_bytes!("../assets/icon.png")).ok().map(|i| {
        let i = i.to_rgba8();
        egui::IconData { width: i.width(), height: i.height(), rgba: i.into_raw() }
    });
    let mut vp = egui::ViewportBuilder::default().with_inner_size([1120.0, 720.0]).with_min_inner_size([980.0, 560.0]).with_title("Craft Installer");
    if let Some(i) = icon {
        vp = vp.with_icon(Arc::new(i));
    }
    let opts = eframe::NativeOptions { viewport: vp, ..Default::default() };
    eframe::run_native("Craft Installer", opts, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
