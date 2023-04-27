#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod eapi;
mod ncm;
mod rc4;

use std::sync::Mutex;

use attohttpc::Session;
use tauri::*;

#[derive(Debug)]
pub struct AppState {
    pub cookie: Mutex<String>,
    pub session: Mutex<Session>,
}

impl Default for AppState {
    fn default() -> Self {
        let mut session = Session::new();
        session.header("origin", "orpheus://orpheus");
        session.header("user-agent", "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Safari/537.36 Chrome/91.0.4472.164 NeteaseMusicDesktop/2.10.7.200791");
        Self {
            cookie: Mutex::new("".into()),
            session: Mutex::new(session),
        }
    }
}

fn recreate_window(app: &AppHandle) {
    if let Some(win) = app.get_window("main") {
        let _ = win.show();
        return;
    }
    #[cfg(debug_assertions)]
    tauri::WindowBuilder::new(
        app,
        "main",
        tauri::WindowUrl::External(app.config().build.dev_path.to_string().parse().unwrap()),
    )
    .inner_size(800., 600.)
    .min_inner_size(800., 600.)
    .title("MRBNCM App")
    .visible(true)
    .theme(Some(Theme::Dark))
    .build()
    .expect("can't show original window");
    #[cfg(not(debug_assertions))]
    tauri::WindowBuilder::new(app, "main", tauri::WindowUrl::App("index.html".into()))
        .inner_size(800., 600.)
        .min_inner_size(800., 600.)
        .title("MRBNCM App")
        .visible(false)
        .theme(Some(Theme::Dark))
        .build()
        .expect("can't show original window");
}

fn exit(app: &AppHandle) {
    audio::stop_audio_thread();
    app.exit(0);
}

fn main() {
    let tray = SystemTray::new().with_menu(
        SystemTrayMenu::new()
            .add_item(CustomMenuItem::new("show", "显示主页面"))
            .add_item(CustomMenuItem::new("quit", "退出 MRBNCM App")),
    );

    tauri::Builder::default()
        .system_tray(tray)
        .manage(AppState::default())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            eapi::tauri_eapi_encrypt,
            eapi::tauri_eapi_decrypt,
            eapi::tauri_eapi_request,
            eapi::tauri_eapi_encrypt_for_request,
            audio::init_audio_thread,
            audio::send_msg_to_audio_thread,
        ])
        .on_system_tray_event(|app, event| match event {
            tauri::SystemTrayEvent::DoubleClick { .. } => {
                recreate_window(app);
            }
            tauri::SystemTrayEvent::LeftClick { .. } => {
                #[cfg(target_os = "macos")]
                {
                    recreate_window(app);
                }
            }
            tauri::SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "show" => {
                    recreate_window(app);
                }
                "quit" => {
                    exit(app);
                }
                _ => {}
            },
            _ => {}
        })
        .setup(|app| {
            recreate_window(&app.handle());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
