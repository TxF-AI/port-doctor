mod models;
mod ports;

use models::{ExcludedRange, KillOutcome, PortEntry, TreeMember};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

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

/// Mémoire du backend PortDoctor (hors WebView), en octets.
#[tauri::command]
async fn get_backend_memory() -> u64 {
    ports::own_memory(std::process::id())
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
    let show = MenuItem::with_id(app, "show", "Afficher PortDoctor", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

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
        .setup(|app| {
            setup_tray(app.handle())?;
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
            get_backend_memory
        ])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de PortDoctor");
}
