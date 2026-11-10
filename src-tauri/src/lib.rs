#[macro_use]
mod i18n;
mod models;
mod ports;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use models::{ExcludedRange, KillOutcome, PortEntry, TreeMember};
use ports::ShieldState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};

/// Ports TCP en écoute et sockets UDP liés, avec processus, mémoire et provenance.
#[tauri::command]
async fn get_active_ports() -> Result<Vec<PortEntry>, String> {
    ports::scan()
}

/// Arrête (de force) le processus qui détient un port, après les garde-fous du backend.
#[tauri::command]
async fn kill_port_process(pid: u32) -> Result<KillOutcome, String> {
    ports::kill_listener(pid, std::process::id())
}

/// Smart Kill : arrête le processus d'un port, ses descendants et, si `include_parents`,
/// ses lanceurs npm/yarn/nodemon. Mêmes garde-fous sur chaque processus de l'arbre.
#[tauri::command]
async fn kill_process_tree(pid: u32, include_parents: bool) -> Result<bool, String> {
    ports::kill_tree(pid, std::process::id(), include_parents)
}

/// Aperçu de l'arbre qu'un Smart Kill arrêterait (lecture seule).
#[tauri::command]
async fn preview_kill_tree(pid: u32) -> Result<Vec<TreeMember>, String> {
    ports::preview_tree(pid, std::process::id())
}

/// Plages de ports TCP exclues par Windows (Hyper-V / WinNAT), lecture seule.
#[tauri::command]
async fn get_excluded_ranges() -> Result<Vec<ExcludedRange>, String> {
    ports::excluded_ranges()
}

/// Contexte fixe du diagnostic copié pour un assistant IA : version de l'appli et de Windows.
#[derive(serde::Serialize)]
struct DiagContext {
    version: &'static str,
    os: String,
}

#[tauri::command]
fn get_diag_context() -> DiagContext {
    DiagContext {
        version: env!("CARGO_PKG_VERSION"),
        os: sysinfo::System::long_os_version().unwrap_or_else(|| "Windows".into()),
    }
}

/// Mémoire du backend PortDoctor (hors WebView), en octets.
#[tauri::command]
async fn get_backend_memory() -> u64 {
    ports::own_memory(std::process::id())
}

/// Un port surveillé par le Port Shield et le nom du processus qui devrait le tenir.
#[derive(serde::Deserialize, Clone)]
struct ShieldEntry {
    port: u16,
    name: String,
}

/// Liste des ports surveillés, partagée avec le thread de surveillance.
#[derive(Default)]
struct ShieldWatch {
    entries: Mutex<Vec<ShieldEntry>>,
}

/// Remplace la liste des ports surveillés (l'interface la garde et la renvoie à chaque changement).
#[tauri::command]
fn set_shield(watch: State<ShieldWatch>, entries: Vec<ShieldEntry>) {
    if let Ok(mut e) = watch.entries.lock() {
        *e = entries;
    }
}

/// Surveille les ports du Shield toutes les 2 s (seulement s'il y en a). Alerte uniquement : aucun arrêt automatique.
/// La première observation d'un port est silencieuse ; ensuite on n'alerte qu'au changement d'état.
fn spawn_shield_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: HashMap<u16, ShieldState> = HashMap::new();
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let entries = match app.state::<ShieldWatch>().entries.lock() {
                Ok(e) => e.clone(),
                Err(_) => continue,
            };
            if entries.is_empty() {
                last.clear();
                continue;
            }
            let Ok(ports) = ports::scan() else { continue };
            let wanted: Vec<(u16, String)> = entries.iter().map(|e| (e.port, e.name.clone())).collect();
            for alert in ports::shield_step(&wanted, &ports, &mut last) {
                let _ = app.emit("shield-alert", alert);
                // Fenêtre masquée (tray) : on la montre ; sinon on fait clignoter la barre des tâches.
                match app.get_webview_window("main") {
                    Some(w) if w.is_visible().unwrap_or(true) => {
                        let _ = w.request_user_attention(Some(tauri::UserAttentionType::Informational));
                    }
                    _ => show_main_window(&app),
                }
            }
        }
    });
}

/// Éléments du menu du tray, gardés pour pouvoir changer leur texte quand la langue change.
struct TrayItems {
    show: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

fn tray_labels() -> (&'static str, &'static str) {
    (tr!("Afficher PortDoctor", "Show PortDoctor"), tr!("Quitter", "Quit"))
}

/// Choisit la langue des messages du backend et du menu du tray ("fr" ou "en").
#[tauri::command]
fn set_language(app: AppHandle, lang: String) {
    i18n::set_english(lang.eq_ignore_ascii_case("en"));
    if let Some(items) = app.try_state::<TrayItems>() {
        let (show, quit) = tray_labels();
        let _ = items.show.set_text(show);
        let _ = items.quit.set_text(quit);
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Branche menu et clics sur l'icône du tray (créée depuis `app.trayIcon` de tauri.conf.json,
/// sinon construite ici).
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let (show_label, quit_label) = tray_labels();
    let show = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_label, true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    app.manage(TrayItems { show, quit });

    let tray: TrayIcon = match app.tray_by_id("main") {
        Some(t) => t,
        None => {
            let mut b = TrayIconBuilder::with_id("main").tooltip("PortDoctor");
            if let Some(icon) = app.default_window_icon() {
                b = b.icon(icon.clone());
            }
            b.build(app)?
        }
    };
    tray.set_menu(Some(menu))?;
    tray.set_show_menu_on_left_click(false)?;
    tray.on_menu_event(|app, event| match event.id.as_ref() {
        "show" => show_main_window(app),
        "quit" => app.exit(0),
        _ => {}
    });
    tray.on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            show_main_window(tray.app_handle());
        }
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ShieldWatch::default())
        .setup(|app| {
            setup_tray(app.handle())?;
            spawn_shield_watcher(app.handle().clone());
            Ok(())
        })
        // Fermer la fenêtre la masque : l'app reste dans la zone de notification.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_active_ports,
            kill_port_process,
            kill_process_tree,
            preview_kill_tree,
            get_excluded_ranges,
            get_backend_memory,
            get_diag_context,
            set_language,
            set_shield
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de PortDoctor");
}
