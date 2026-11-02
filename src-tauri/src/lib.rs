mod bandeja;

use std::time::Duration;

use tauri::{AppHandle, Manager, WindowEvent};

/// Trae al frente la ventana principal (desde la bandeja o al abrir Klippo otra vez)
pub fn mostrar_ventana(app: &AppHandle) {
    if let Some(ventana) = app.get_webview_window("main") {
        let _ = ventana.show();
        let _ = ventana.unminimize();
        let _ = ventana.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Tiene que ir el primero: si Klippo ya está abierto, la segunda copia se cierra
        // y la que estaba abierta pasa al frente
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            mostrar_ventana(app)
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            bandeja::crear(app.handle())?;

            // La ventana nace oculta y la interfaz la muestra cuando ya tiene los colores
            // puestos (sin destello blanco en el tema oscuro). Por si la interfaz fallara,
            // se muestra igualmente a los pocos segundos.
            let app = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(3));
                if let Some(ventana) = app.get_webview_window("main") {
                    if !ventana.is_visible().unwrap_or(true) {
                        let _ = ventana.show();
                    }
                }
            });
            Ok(())
        })
        // Cerrar la ventana no cierra Klippo: sigue en la bandeja ("Salir" lo cierra del todo)
        .on_window_event(|ventana, evento| {
            if let WindowEvent::CloseRequested { api, .. } = evento {
                api.prevent_close();
                let _ = ventana.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar Klippo");
}
