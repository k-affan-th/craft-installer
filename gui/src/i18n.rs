//! UI strings. Column order: en, th, zh, ja, ko, es, fr, de, pt, ru.
//! egui has no complex-script shaping, so Arabic/Hindi (which need it) are intentionally absent.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    Th,
    Zh,
    Ja,
    Ko,
    Es,
    Fr,
    De,
    Pt,
    Ru,
}

impl Lang {
    pub const ALL: [Lang; 10] = [Lang::En, Lang::Th, Lang::Zh, Lang::Ja, Lang::Ko, Lang::Es, Lang::Fr, Lang::De, Lang::Pt, Lang::Ru];
    pub fn code(self) -> &'static str {
        ["en", "th", "zh", "ja", "ko", "es", "fr", "de", "pt", "ru"][self as usize]
    }
    pub fn name(self) -> &'static str {
        ["English", "ไทย", "简体中文", "日本語", "한국어", "Español", "Français", "Deutsch", "Português", "Русский"][self as usize]
    }
    pub fn from_code(c: &str) -> Lang {
        Lang::ALL.into_iter().find(|l| l.code() == c.trim().to_lowercase()).unwrap_or(Lang::En)
    }
}

/// Look up `key`; falls back to English, then to the key itself.
pub fn tr(l: Lang, key: &str) -> &'static str {
    match TABLE.iter().find(|(k, _)| *k == key) {
        Some((_, v)) => v[l as usize],
        None => "?",
    }
}

/// tr + replace {0}, {1}, ... with args.
pub fn trf(l: Lang, key: &str, args: &[&str]) -> String {
    let mut s = tr(l, key).to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{i}}}"), a);
    }
    s
}

type Row = (&'static str, [&'static str; 10]);
static TABLE: &[Row] = &[
    ("step1", ["Choose apps", "เลือกแอป", "选择应用", "アプリを選択", "앱 선택", "Elegir apps", "Choisir les apps", "Apps wählen", "Escolher apps", "Выбор приложений"]),
    ("step2", ["Choose method", "เลือกวิธี", "选择方式", "方法を選択", "방식 선택", "Elegir método", "Choisir la méthode", "Methode wählen", "Escolher método", "Выбор способа"]),
    ("step3", ["Install", "ติดตั้ง", "安装", "インストール", "설치", "Instalar", "Installer", "Installieren", "Instalar", "Установка"]),
    ("pick_hint", ["Tick the apps you want", "ติ๊กแอปที่ต้องการ", "勾选需要的应用", "必要なアプリにチェック", "원하는 앱을 선택하세요", "Marca las apps que quieras", "Cochez les apps souhaitées", "Gewünschte Apps ankreuzen", "Marque os apps desejados", "Отметьте нужные приложения"]),
    ("all", ["All", "ทั้งหมด", "全选", "すべて", "전체", "Todas", "Toutes", "Alle", "Todas", "Все"]),
    ("only_new", ["Updates only", "เฉพาะที่มีเวอร์ชันใหม่", "仅有更新的", "更新ありのみ", "업데이트만", "Solo con novedades", "Mises à jour seulement", "Nur mit Updates", "Só com atualização", "Только с обновлением"]),
    ("clear", ["Clear", "ล้าง", "清除", "クリア", "지우기", "Limpiar", "Effacer", "Leeren", "Limpar", "Сбросить"]),
    ("refresh", ["↻ Check versions", "↻ ตรวจเวอร์ชัน", "↻ 检查版本", "↻ バージョン確認", "↻ 버전 확인", "↻ Comprobar versiones", "↻ Vérifier les versions", "↻ Versionen prüfen", "↻ Verificar versões", "↻ Проверить версии"]),
    ("g_img", ["Images & design", "ภาพและงานออกแบบ", "图像与设计", "画像・デザイン", "이미지 및 디자인", "Imagen y diseño", "Image et design", "Bild & Design", "Imagem e design", "Графика и дизайн"]),
    ("g_av", ["Video & audio", "วิดีโอและเสียง", "视频与音频", "動画・音声", "비디오 및 오디오", "Vídeo y audio", "Vidéo et audio", "Video & Audio", "Vídeo e áudio", "Видео и аудио"]),
    ("g_doc", ["Documents & office", "เอกสารและสำนักงาน", "文档与办公", "ドキュメント・オフィス", "문서 및 오피스", "Documentos y oficina", "Documents et bureautique", "Dokumente & Büro", "Documentos e escritório", "Документы и офис"]),
    ("pick_group", ["Select group", "เลือกกลุ่มนี้", "选择本组", "このグループを選択", "그룹 선택", "Elegir grupo", "Choisir le groupe", "Gruppe wählen", "Escolher grupo", "Выбрать группу"]),
    ("not_inst", ["○ Not installed {0}", "○ ยังไม่ติดตั้ง {0}", "○ 未安装 {0}", "○ 未インストール {0}", "○ 설치 안 됨 {0}", "○ No instalada {0}", "○ Non installée {0}", "○ Nicht installiert {0}", "○ Não instalado {0}", "○ Не установлено {0}"]),
    ("inst", ["● Installed {0}", "● ติดตั้งแล้ว {0}", "● 已安装 {0}", "● インストール済み {0}", "● 설치됨 {0}", "● Instalada {0}", "● Installée {0}", "● Installiert {0}", "● Instalado {0}", "● Установлено {0}"]),
    ("upd", ["● Update {0} available", "● มีใหม่ {0}", "● 有新版本 {0}", "● 更新あり {0}", "● 업데이트 {0}", "● Novedad {0}", "● Mise à jour {0}", "● Update {0}", "● Atualização {0}", "● Обновление {0}"]),
    ("open", ["▶ Open", "▶ เปิด", "▶ 打开", "▶ 開く", "▶ 열기", "▶ Abrir", "▶ Ouvrir", "▶ Öffnen", "▶ Abrir", "▶ Открыть"]),
    ("folder", ["Folder", "โฟลเดอร์", "文件夹", "フォルダー", "폴더", "Carpeta", "Dossier", "Ordner", "Pasta", "Папка"]),
    ("uninstall_sel", ["Uninstall selected", "ถอนการติดตั้งที่เลือก", "卸载所选", "選択をアンインストール", "선택 항목 제거", "Desinstalar selección", "Désinstaller la sélection", "Auswahl deinstallieren", "Desinstalar selecionados", "Удалить выбранное"]),
    ("next", ["Next: choose method ({0} apps) →", "ถัดไป: เลือกวิธี ({0} แอป) →", "下一步：选择方式（{0} 个应用）→", "次へ：方法を選択（{0} 個）→", "다음: 방식 선택 ({0}개) →", "Siguiente: método ({0} apps) →", "Suivant : méthode ({0} apps) →", "Weiter: Methode ({0} Apps) →", "Próximo: método ({0} apps) →", "Далее: способ ({0}) →"]),
    ("pick_first", ["Tick apps first", "ติ๊กแอปก่อน", "请先勾选应用", "先にアプリを選択", "먼저 앱을 선택", "Marca apps primero", "Cochez d'abord des apps", "Zuerst Apps wählen", "Marque apps primeiro", "Сначала выберите"]),
    ("back", ["← Back", "← ย้อนกลับ", "← 返回", "← 戻る", "← 뒤로", "← Atrás", "← Retour", "← Zurück", "← Voltar", "← Назад"]),
    ("will", ["Will install {0} apps: {1}", "จะติดตั้ง {0} แอป: {1}", "将安装 {0} 个应用：{1}", "{0} 個をインストール：{1}", "{0}개 앱 설치: {1}", "Se instalarán {0} apps: {1}", "{0} apps seront installées : {1}", "{0} Apps werden installiert: {1}", "Serão instalados {0} apps: {1}", "Будет установлено {0}: {1}"]),
    ("m_fast_t", ["Quick — unpack + shortcuts", "เร็ว — แตกไฟล์ + ทางลัด", "快速 — 解压 + 快捷方式", "クイック — 展開＋ショートカット", "빠름 — 압축 해제 + 바로가기", "Rápido — descomprimir + accesos", "Rapide — extraction + raccourcis", "Schnell — entpacken + Verknüpfungen", "Rápido — extrair + atalhos", "Быстро — распаковка + ярлыки"]),
    ("m_fast_b", [
        "Unpacks into your user folder (craft-apps) and adds app-menu shortcuts. No administrator prompt.",
        "แตกไฟล์ไว้ในโฟลเดอร์ผู้ใช้ (craft-apps) แล้วสร้างทางลัดในเมนูแอปให้ ไม่ต้องกดยืนยันสิทธิ์ผู้ดูแล",
        "解压到你的用户文件夹（craft-apps）并创建应用菜单快捷方式，无需管理员确认。",
        "ユーザーフォルダー（craft-apps）に展開し、アプリメニューにショートカットを作成します。管理者の確認は不要です。",
        "사용자 폴더(craft-apps)에 풀고 앱 메뉴 바로가기를 만듭니다. 관리자 확인이 필요 없습니다.",
        "Se descomprime en tu carpeta de usuario (craft-apps) y crea accesos en el menú de aplicaciones. Sin aviso de administrador.",
        "Extrait dans votre dossier utilisateur (craft-apps) et crée des raccourcis dans le menu des applications. Aucune invite administrateur.",
        "Entpackt in Ihren Benutzerordner (craft-apps) und legt Verknüpfungen im Anwendungsmenü an. Keine Administrator-Abfrage.",
        "Extrai na sua pasta de usuário (craft-apps) e cria atalhos no menu de aplicativos. Sem aviso de administrador.",
        "Распаковывает в вашу папку пользователя (craft-apps) и добавляет ярлыки в меню приложений. Без запроса администратора.",
    ]),
    ("recommended", ["Recommended", "แนะนำ", "推荐", "おすすめ", "추천", "Recomendado", "Recommandé", "Empfohlen", "Recomendado", "Рекомендуется"]),
    ("m_msi_t", [
        "System installer",
        "ตัวติดตั้งของระบบ",
        "系统安装包",
        "システムインストーラー",
        "시스템 설치 프로그램",
        "Instalador del sistema",
        "Installeur système",
        "System-Installer",
        "Instalador do sistema",
        "Системный установщик",
    ]),
    ("m_msi_b", [
        "Native package: .msi on Windows, .dmg to Applications on macOS, .deb/.rpm on Linux. One administrator prompt for the whole batch.",
        "แพ็กเกจของระบบ: .msi บน Windows, .dmg ลง Applications บน macOS, .deb/.rpm บน Linux  •  ขอสิทธิ์ผู้ดูแลครั้งเดียวต่อทั้งชุด",
        "系统原生安装包：Windows 用 .msi，macOS 将 .dmg 装入“应用程序”，Linux 用 .deb/.rpm。整批只需确认一次管理员权限。",
        "OS 標準のパッケージ：Windows は .msi、macOS は .dmg を「アプリケーション」へ、Linux は .deb/.rpm。管理者の確認は一括で 1 回です。",
        "OS 기본 패키지: Windows는 .msi, macOS는 .dmg를 응용 프로그램으로, Linux는 .deb/.rpm. 관리자 확인은 전체에 한 번뿐입니다.",
        "Paquete nativo: .msi en Windows, .dmg a Aplicaciones en macOS, .deb/.rpm en Linux. Un solo aviso de administrador para todo el lote.",
        "Paquet natif : .msi sous Windows, .dmg vers Applications sous macOS, .deb/.rpm sous Linux. Une seule invite administrateur pour tout le lot.",
        "Natives Paket: .msi unter Windows, .dmg nach Programme unter macOS, .deb/.rpm unter Linux. Nur eine Administrator-Abfrage für den ganzen Stapel.",
        "Pacote nativo: .msi no Windows, .dmg para Aplicativos no macOS, .deb/.rpm no Linux. Um único aviso de administrador para todo o lote.",
        "Нативный пакет: .msi в Windows, .dmg в «Программы» в macOS, .deb/.rpm в Linux. Один запрос прав администратора на весь набор.",
    ]),
    ("m_build_t", ["Build from source", "build จากซอร์ส", "从源码构建", "ソースからビルド", "소스에서 빌드", "Compilar desde el código", "Compiler depuis les sources", "Aus Quellcode bauen", "Compilar do código-fonte", "Сборка из исходников"]),
    ("m_build_b", [
        "Downloads the source and compiles it. Needs git and Rust. Slow (minutes per app), no prebuilt files.",
        "โหลดซอร์สแล้วคอมไพล์เอง ต้องมี git และ Rust  •  ช้า (หลายนาทีต่อแอป) แต่ไม่พึ่งไฟล์ติดตั้งสำเร็จรูป",
        "下载源码并自行编译，需要 git 和 Rust。较慢（每个应用数分钟），不依赖预编译文件。",
        "ソースを取得してコンパイルします。git と Rust が必要。遅い（1 アプリ数分）が、ビルド済みファイルに依存しません。",
        "소스를 받아 직접 컴파일합니다. git과 Rust 필요. 느리지만(앱당 수 분) 미리 빌드된 파일에 의존하지 않습니다.",
        "Descarga el código y lo compila. Requiere git y Rust. Lento (minutos por app), sin archivos precompilados.",
        "Télécharge les sources et les compile. Nécessite git et Rust. Lent (minutes par app), sans binaires précompilés.",
        "Lädt den Quellcode und kompiliert ihn. Benötigt git und Rust. Langsam (Minuten pro App), ohne Fertigdateien.",
        "Baixa o código-fonte e compila. Requer git e Rust. Lento (minutos por app), sem arquivos prontos.",
        "Скачивает исходники и компилирует. Нужны git и Rust. Медленно (минуты на приложение), без готовых файлов.",
    ]),
    ("opts_for", ["Options for this method", "ตัวเลือกสำหรับวิธีนี้", "此方式的选项", "この方法のオプション", "이 방식의 옵션", "Opciones de este método", "Options de cette méthode", "Optionen für diese Methode", "Opções deste método", "Параметры способа"]),
    ("desk", ["Also create a Desktop shortcut", "สร้างทางลัดบน Desktop ด้วย", "同时创建桌面快捷方式", "デスクトップにもショートカットを作成", "바탕 화면 바로가기도 만들기", "Crear también acceso en el escritorio", "Créer aussi un raccourci sur le Bureau", "Auch Desktop-Verknüpfung anlegen", "Criar também atalho na área de trabalho", "Создать ярлык на рабочем столе"]),
    ("no_extra", ["No extra options for this method", "วิธีนี้ไม่มีตัวเลือกเพิ่ม", "此方式没有额外选项", "この方法に追加オプションはありません", "이 방식에는 추가 옵션이 없습니다", "Sin opciones adicionales", "Pas d'option supplémentaire", "Keine zusätzlichen Optionen", "Sem opções extras", "Дополнительных параметров нет"]),
    ("cli", ["Also build the -cli binary", "build ตัว -cli ด้วย", "同时构建 -cli", "-cli もビルド", "-cli도 빌드", "Compilar también -cli", "Compiler aussi -cli", "Auch -cli bauen", "Compilar também -cli", "Собрать также -cli"]),
    ("fonts", ["Use craft-fonts (CJK fonts in the UI)", "ใช้ craft-fonts (ฟอนต์ CJK ใน UI)", "使用 craft-fonts（界面 CJK 字体）", "craft-fonts を使用（UI の CJK フォント）", "craft-fonts 사용 (UI의 CJK 글꼴)", "Usar craft-fonts (fuentes CJK en la UI)", "Utiliser craft-fonts (polices CJK dans l'UI)", "craft-fonts verwenden (CJK-Schriften in der UI)", "Usar craft-fonts (fontes CJK na UI)", "Использовать craft-fonts (CJK-шрифты в UI)"]),
    ("local_src", ["Local source folder (optional):", "โฟลเดอร์ซอร์สในเครื่อง (ไม่บังคับ):", "本地源码文件夹（可选）：", "ローカルのソースフォルダー（任意）：", "로컬 소스 폴더(선택):", "Carpeta local de código (opcional):", "Dossier source local (facultatif) :", "Lokaler Quellordner (optional):", "Pasta local do código (opcional):", "Локальная папка с исходниками (необязательно):"]),
    ("local_hint", [
        "e.g. D:\\GitHub\\craft-mirrors — empty = download from GitHub",
        "เช่น D:\\GitHub\\craft-mirrors — ว่างไว้ = โหลดจาก GitHub",
        "例如 D:\\GitHub\\craft-mirrors — 留空则从 GitHub 下载",
        "例: D:\\GitHub\\craft-mirrors — 空欄なら GitHub から取得",
        "예: D:\\GitHub\\craft-mirrors — 비우면 GitHub에서 받음",
        "p. ej. D:\\GitHub\\craft-mirrors — vacío = descargar de GitHub",
        "ex. D:\\GitHub\\craft-mirrors — vide = depuis GitHub",
        "z. B. D:\\GitHub\\craft-mirrors — leer = von GitHub laden",
        "ex.: D:\\GitHub\\craft-mirrors — vazio = baixar do GitHub",
        "напр. D:\\GitHub\\craft-mirrors — пусто = скачать с GitHub",
    ]),
    ("start", ["▶  Install {0} apps", "▶  เริ่มติดตั้ง {0} แอป", "▶  开始安装 {0} 个应用", "▶  {0} 個をインストール開始", "▶  {0}개 앱 설치 시작", "▶  Instalar {0} apps", "▶  Installer {0} apps", "▶  {0} Apps installieren", "▶  Instalar {0} apps", "▶  Установить ({0})"]),
    ("working_i", ["Installing…", "กำลังติดตั้ง…", "正在安装…", "インストール中…", "설치 중…", "Instalando…", "Installation…", "Installiere…", "Instalando…", "Установка…"]),
    ("working_u", ["Uninstalling…", "กำลังถอนการติดตั้ง…", "正在卸载…", "アンインストール中…", "제거 중…", "Desinstalando…", "Désinstallation…", "Deinstalliere…", "Desinstalando…", "Удаление…"]),
    ("uac", [
        "(If an administrator prompt appears, approve it once)",
        "(ถ้ามีหน้าต่างขอสิทธิ์ผู้ดูแล ให้กดอนุญาตครั้งเดียว)",
        "（如出现管理员权限提示，请批准一次）",
        "（管理者権限の確認が出たら 1 回許可してください）",
        "(관리자 권한 창이 뜨면 한 번 승인하세요)",
        "(Si aparece un aviso de administrador, apruébalo una vez)",
        "(Si une invite administrateur apparaît, validez-la une fois)",
        "(Erscheint eine Administrator-Abfrage, einmal bestätigen)",
        "(Se aparecer um aviso de administrador, aprove uma vez)",
        "(Если появится запрос прав администратора, подтвердите один раз)",
    ]),
    ("queued", ["Queued…", "รอคิว…", "排队中…", "待機中…", "대기 중…", "En cola…", "En attente…", "Wartet…", "Na fila…", "В очереди…"]),
    ("done_i", ["✔ Done", "✔ เรียบร้อย", "✔ 完成", "✔ 完了", "✔ 완료", "✔ Listo", "✔ Terminé", "✔ Fertig", "✔ Concluído", "✔ Готово"]),
    ("done_u", ["✔ Removed", "✔ ถอนแล้ว", "✔ 已卸载", "✔ 削除済み", "✔ 제거됨", "✔ Desinstalada", "✔ Désinstallée", "✔ Entfernt", "✔ Removido", "✔ Удалено"]),
    ("log", ["Details / log ({0})", "รายละเอียด / log ({0})", "详情 / 日志（{0}）", "詳細 / ログ（{0}）", "자세히 / 로그 ({0})", "Detalles / registro ({0})", "Détails / journal ({0})", "Details / Protokoll ({0})", "Detalhes / registro ({0})", "Подробности / журнал ({0})"]),
    ("more", ["← Do more", "← ทำเพิ่ม", "← 继续安装", "← 続ける", "← 더 하기", "← Hacer más", "← Continuer", "← Weitere", "← Fazer mais", "← Ещё"]),
    ("finish", ["Finish — close", "เสร็จสิ้น — ปิดโปรแกรม", "完成 — 关闭", "完了 — 閉じる", "완료 — 닫기", "Terminar — cerrar", "Terminer — fermer", "Fertig — schließen", "Concluir — fechar", "Готово — закрыть"]),
    ("sum_ok_i", [
        "Installed {0} of {1} apps — use Open or search the app menu",
        "ติดตั้งสำเร็จ {0} จาก {1} แอป — กดปุ่ม \"เปิด\" หรือค้นหาในเมนูแอป",
        "已安装 {0}/{1} 个应用 — 点击“打开”或在应用菜单中搜索",
        "{1} 個中 {0} 個をインストールしました — 「開く」またはアプリメニューで検索",
        "{1}개 중 {0}개 설치 완료 — '열기'를 누르거나 앱 메뉴에서 검색하세요",
        "Instaladas {0} de {1} apps — usa Abrir o busca en el menú de aplicaciones",
        "{0} app(s) sur {1} installée(s) — utilisez Ouvrir ou cherchez dans le menu des applications",
        "{0} von {1} Apps installiert — „Öffnen“ oder im Anwendungsmenü suchen",
        "{0} de {1} apps instalados — use Abrir ou pesquise no menu de aplicativos",
        "Установлено {0} из {1} — нажмите «Открыть» или найдите в меню приложений",
    ]),
    ("sum_ok_u", ["Removed {0} of {1} apps", "ถอนการติดตั้งสำเร็จ {0} จาก {1} แอป", "已卸载 {0}/{1} 个应用", "{1} 個中 {0} 個をアンインストールしました", "{1}개 중 {0}개 제거 완료", "Desinstaladas {0} de {1} apps", "{0} app(s) sur {1} désinstallée(s)", "{0} von {1} Apps entfernt", "{0} de {1} apps removidos", "Удалено {0} из {1}"]),
    ("sum_fail", ["Succeeded {0} • failed {1}: {2}", "สำเร็จ {0} • ล้มเหลว {1}: {2}", "成功 {0} • 失败 {1}：{2}", "成功 {0} • 失敗 {1}：{2}", "성공 {0} • 실패 {1}: {2}", "Correctas {0} • fallidas {1}: {2}", "Réussies {0} • échecs {1} : {2}", "Erfolgreich {0} • fehlgeschlagen {1}: {2}", "Sucesso {0} • falhas {1}: {2}", "Успешно {0} • ошибок {1}: {2}"]),
    ("t_done", ["Craft Installer: done", "Craft Installer: เสร็จแล้ว", "Craft Installer：已完成", "Craft Installer：完了", "Craft Installer: 완료", "Craft Installer: listo", "Craft Installer : terminé", "Craft Installer: fertig", "Craft Installer: concluído", "Craft Installer: готово"]),
    ("t_err", ["Craft Installer: errors", "Craft Installer: มีข้อผิดพลาด", "Craft Installer：出现错误", "Craft Installer：エラーあり", "Craft Installer: 오류 발생", "Craft Installer: errores", "Craft Installer : erreurs", "Craft Installer: Fehler", "Craft Installer: erros", "Craft Installer: ошибки"]),
    ("lang", ["Language", "ภาษา", "语言", "言語", "언어", "Idioma", "Langue", "Sprache", "Idioma", "Язык"]),
    ("pin_t", [
        "Add Craft Installer to the app menu",
        "เพิ่ม Craft Installer ลงในเมนูแอป",
        "将 Craft Installer 添加到应用菜单",
        "Craft Installer をアプリメニューに追加",
        "Craft Installer를 앱 메뉴에 추가",
        "Añadir Craft Installer al menú de aplicaciones",
        "Ajouter Craft Installer au menu des applications",
        "Craft Installer zum Anwendungsmenü hinzufügen",
        "Adicionar o Craft Installer ao menu de aplicativos",
        "Добавить Craft Installer в меню приложений",
    ]),
    ("pin_b", [
        "Open it later by searching “Craft Installer” in your system's app menu or search (Windows key, Launchpad/Spotlight, or Activities).",
        "เปิดครั้งต่อไปได้โดยค้นหา \"Craft Installer\" ในเมนูแอปหรือช่องค้นหาของระบบ (ปุ่ม Windows, Launchpad/Spotlight หรือ Activities)",
        "之后在系统的应用菜单或搜索中查找“Craft Installer”即可打开（Windows 键、启动台/聚焦搜索或 Activities）。",
        "次回はシステムのアプリメニューや検索（Windows キー、Launchpad/Spotlight、Activities）で「Craft Installer」を探して開けます。",
        "나중에 시스템 앱 메뉴나 검색(Windows 키, Launchpad/Spotlight, Activities)에서 'Craft Installer'를 찾아 열 수 있습니다.",
        "Ábrelo después buscando «Craft Installer» en el menú de aplicaciones o la búsqueda del sistema (tecla Windows, Launchpad/Spotlight o Actividades).",
        "Ouvrez-le ensuite en cherchant « Craft Installer » dans le menu des applications ou la recherche du système (touche Windows, Launchpad/Spotlight ou Activités).",
        "Später über das Anwendungsmenü oder die Systemsuche öffnen („Craft Installer“; Windows-Taste, Launchpad/Spotlight oder Aktivitäten).",
        "Abra depois pesquisando “Craft Installer” no menu de aplicativos ou na busca do sistema (tecla Windows, Launchpad/Spotlight ou Atividades).",
        "Потом найдите «Craft Installer» в меню приложений или поиске системы (клавиша Windows, Launchpad/Spotlight или «Обзор»).",
    ]),
    ("pin_btn", [
        "Add to app menu",
        "เพิ่มลงเมนูแอป",
        "添加到应用菜单",
        "アプリメニューに追加",
        "앱 메뉴에 추가",
        "Añadir al menú de aplicaciones",
        "Ajouter au menu des applications",
        "Zum Anwendungsmenü hinzufügen",
        "Adicionar ao menu de aplicativos",
        "Добавить в меню приложений",
    ]),
    ("pin_no", ["Not now", "ไว้ทีหลัง", "以后再说", "今はしない", "나중에", "Ahora no", "Plus tard", "Nicht jetzt", "Agora não", "Не сейчас"]),
    ("pin_done", [
        "✔ In the app menu",
        "✔ อยู่ในเมนูแอปแล้ว",
        "✔ 已在应用菜单",
        "✔ アプリメニューに追加済み",
        "✔ 앱 메뉴에 있음",
        "✔ En el menú de aplicaciones",
        "✔ Dans le menu des applications",
        "✔ Im Anwendungsmenü",
        "✔ No menu de aplicativos",
        "✔ В меню приложений",
    ]),
    ("pin_rm", [
        "Remove from app menu",
        "เอาออกจากเมนูแอป",
        "从应用菜单移除",
        "アプリメニューから削除",
        "앱 메뉴에서 제거",
        "Quitar del menú de aplicaciones",
        "Retirer du menu des applications",
        "Aus Anwendungsmenü entfernen",
        "Remover do menu de aplicativos",
        "Убрать из меню приложений",
    ]),
    ("d_photocraft", ["Image editor", "ตัวแก้ไขภาพ", "图像编辑器", "画像エディター", "이미지 편집기", "Editor de imágenes", "Éditeur d'images", "Bildeditor", "Editor de imagens", "Редактор изображений"]),
    ("d_vectorcraft", ["Vector graphics", "งานเวกเตอร์", "矢量图形", "ベクターグラフィック", "벡터 그래픽", "Gráficos vectoriales", "Graphisme vectoriel", "Vektorgrafik", "Gráficos vetoriais", "Векторная графика"]),
    ("d_filmcraft", ["Video editor", "ตัวตัดต่อวิดีโอ", "视频编辑器", "動画エディター", "동영상 편집기", "Editor de vídeo", "Éditeur vidéo", "Videoeditor", "Editor de vídeo", "Видеоредактор"]),
    ("d_lightcraft", ["Photo library & RAW", "คลังภาพ / RAW", "照片库与 RAW", "フォトライブラリ・RAW", "사진 라이브러리·RAW", "Fototeca y RAW", "Photothèque et RAW", "Fotobibliothek & RAW", "Biblioteca de fotos e RAW", "Фотоархив и RAW"]),
    ("d_pdfcraft", ["PDF tools", "เครื่องมือ PDF", "PDF 工具", "PDF ツール", "PDF 도구", "Herramientas PDF", "Outils PDF", "PDF-Werkzeuge", "Ferramentas PDF", "Инструменты PDF"]),
    ("d_effectcraft", ["Motion graphics / VFX", "motion graphics / VFX", "动态图形 / 视觉特效", "モーショングラフィックス / VFX", "모션 그래픽 / VFX", "Motion graphics / VFX", "Motion design / VFX", "Motion Graphics / VFX", "Motion graphics / VFX", "Моушн-графика / VFX"]),
    ("d_designcraft", ["Page layout", "จัดหน้า / สิ่งพิมพ์", "页面排版", "ページレイアウト", "페이지 레이아웃", "Maquetación", "Mise en page", "Seitenlayout", "Diagramação", "Верстка"]),
    ("d_wordcraft", ["Word processor", "ประมวลผลคำ", "文字处理", "ワープロ", "워드 프로세서", "Procesador de textos", "Traitement de texte", "Textverarbeitung", "Processador de texto", "Текстовый редактор"]),
    ("d_gridcraft", ["Spreadsheet", "สเปรดชีต", "电子表格", "表計算", "스프레드시트", "Hoja de cálculo", "Tableur", "Tabellenkalkulation", "Planilhas", "Электронные таблицы"]),
    ("d_deckcraft", ["Presentations", "งานนำเสนอ", "演示文稿", "プレゼンテーション", "프레젠테이션", "Presentaciones", "Présentations", "Präsentationen", "Apresentações", "Презентации"]),
    ("d_cadcraft", ["CAD drafting", "ออกแบบ / เขียนแบบ CAD", "CAD 制图", "CAD 製図", "CAD 제도", "Dibujo CAD", "Dessin CAO", "CAD-Zeichnen", "Desenho CAD", "CAD-черчение"]),
    ("d_soundcraft", ["Audio editing", "ตัดต่อเสียง", "音频编辑", "オーディオ編集", "오디오 편집", "Edición de audio", "Montage audio", "Audiobearbeitung", "Edição de áudio", "Работа со звуком"]),
];
