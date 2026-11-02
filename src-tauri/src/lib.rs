mod bandeja;
mod cache;
mod comandos;
mod descargas;
mod formatos;
mod motor;

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

use descargas::Descargas;
use motor::Motor;

/// Trae al frente la ventana principal (desde la bandeja o al abrir Klippo otra vez)
pub fn mostrar_ventana(app: &AppHandle) {
    if let Some(ventana) = app.get_webview_window("main") {
        let _ = ventana.show();
        let _ = ventana.unminimize();
        let _ = ventana.set_focus();
    }
}

/// Igual que en Android: inglés si el sistema está en inglés, español en cualquier otro caso
static EN_INGLES: LazyLock<bool> = LazyLock::new(|| {
    sys_locale::get_locale().is_some_and(|idioma| idioma.to_lowercase().starts_with("en"))
});

/// Los pocos textos que salen de Rust (bandeja y algún error)
pub fn traducir(es: &str, en: &str) -> String {
    if *EN_INGLES { en } else { es }.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
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
            let motor = Motor::preparar(app.handle());
            app.manage(Descargas::new(app.handle().clone(), motor));

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
        .invoke_handler(tauri::generate_handler![
            comandos::listar_descargas,
            comandos::descargar,
            comandos::cancelar_descarga,
            comandos::reintentar_descarga,
            comandos::eliminar_descarga,
            comandos::reproducir,
            comandos::mostrar_en_carpeta,
            comandos::info_motor,
            comandos::actualizar_motor,
            comandos::tamano_cache,
            comandos::vaciar_cache,
        ])
        .build(tauri::generate_context!())
        .expect("no se pudo iniciar Klippo");

    app.run(|app, evento| {
        if let RunEvent::Exit = evento {
            if let Some(descargas) = app.try_state::<Arc<Descargas>>() {
                descargas.cancelar_todas();
            }
        }
    });
}
