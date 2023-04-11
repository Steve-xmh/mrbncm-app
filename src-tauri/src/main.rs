#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod eapi;
mod ncm;
mod rc4;
mod audio;

use tauri::*;

fn recreate_window(app: &AppHandle) {
    #[cfg(debug_assertions)]
    tauri::WindowBuilder::new(
        app,
        "main",
        tauri::WindowUrl::External(app.config().build.dev_path.to_string().parse().unwrap()),
    )
    .inner_size(800., 600.)
    .min_inner_size(800., 600.)
    .title("MRBNCM App")
    .build()
    .expect("can't show original window");
    #[cfg(not(debug_assertions))]
    tauri::WindowBuilder::new(app, "main", tauri::WindowUrl::App("index.html".into()))
        .inner_size(800., 600.)
        .min_inner_size(800., 600.)
        .title("MRBNCM App")
        .build()
        .expect("can't show original window");
}

fn main() {
    let tray = SystemTray::new().with_menu(
        SystemTrayMenu::new()
            .add_item(CustomMenuItem::new("show", "显示主页面"))
            .add_item(CustomMenuItem::new("quit", "退出 MRBNCM App")),
    );

    tauri::Builder::default()
        .system_tray(tray)
        .invoke_handler(tauri::generate_handler![
            eapi::tauri_eapi_encrypt,
            eapi::tauri_eapi_decrypt,
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
                    app.exit(0);
                }
                _ => {}
            },
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
