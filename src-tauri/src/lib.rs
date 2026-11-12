mod avisos;
mod bandeja;
mod cache;
mod comandos;
mod compartir;
mod descargas;
mod edicion;
mod enlaces;
mod entrada;
mod formatos;
mod marcador;
mod motor;
mod sesiones;
mod spotify;

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use tauri_plugin_deep_link::DeepLinkExt;

use descargas::Descargas;
use entrada::Entrada;
use motor::Motor;

/// Argumento con el que Windows abre Klippo al iniciar sesión (Ajustes → Iniciar con Windows)
const ARG_OCULTO: &str = "--oculto";

/// Cómo arrancó esta vez Klippo
pub struct Arranque {
    /// Con Windows: se queda en la bandeja sin enseñar la ventana
    pub oculto: bool,
}

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

/// Los pocos textos que salen de Rust (bandeja, avisos y algún error)
pub fn traducir(es: &str, en: &str) -> String {
    if *EN_INGLES { en } else { es }.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let oculto = std::env::args().any(|a| a == ARG_OCULTO);
    let app = tauri::Builder::default()
        // Tiene que ir el primero: si Klippo ya está abierto, la segunda copia se cierra y
        // la que estaba abierta pasa al frente. Si la segunda traía un klippo://, el plugin
        // deep-link se lo entrega a esta (on_open_url).
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if !args.iter().any(|a| a.starts_with("klippo:")) {
                mostrar_ventana(app);
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().arg(ARG_OCULTO).build())
        .manage(Arranque { oculto })
        .manage(Entrada::default())
        .setup(move |app| {
            bandeja::crear(app.handle())?;
            let motor = Motor::preparar(app.handle());
            app.manage(Descargas::new(app.handle().clone(), motor));
            entrada::vigilar_portapapeles(app.handle().clone());

            // klippo://descargar?url=… (marcador "Enviar a Klippo"). El instalador registra el
            // protocolo; en desarrollo se registra aquí, apuntando al ejecutable de pruebas.
            #[cfg(debug_assertions)]
            let _ = app.deep_link().register_all();
            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |evento| {
                for enlace in evento.urls() {
                    entrada::abrir_enlace_klippo(&handle, &enlace);
                }
            });
            if let Ok(Some(enlaces)) = app.deep_link().get_current() {
                for enlace in enlaces {
                    entrada::abrir_enlace_klippo(app.handle(), &enlace);
                }
            }

            // La ventana nace oculta y la interfaz la muestra cuando ya tiene los colores
            // puestos (sin destello blanco en el tema oscuro). Por si la interfaz fallara,
            // se muestra igualmente a los pocos segundos (salvo al arrancar con Windows).
            if !oculto {
                let app = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(3));
                    if let Some(ventana) = app.get_webview_window("main") {
                        if !ventana.is_visible().unwrap_or(true) {
                            let _ = ventana.show();
                        }
                    }
                });
            }
            Ok(())
        })
        // Cerrar la ventana no cierra Klippo: sigue en la bandeja ("Salir" lo cierra del todo)
        .on_window_event(|ventana, evento| {
            // (las ventanas de inicio de sesión sí se cierran)
            if ventana.label() != "main" {
                return;
            }
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
            comandos::enlace_del_portapapeles,
            comandos::tomar_enlace_pendiente,
            comandos::instalar_marcador,
            comandos::codigo_marcador,
            comandos::arranque_oculto,
            comandos::guardar_descarga,
            comandos::mostrar_guardado,
            comandos::copiar_descarga,
            comandos::whatsapp_descarga,
            comandos::compartir_descarga,
            comandos::exportar,
            comandos::cancelar_exportacion,
            comandos::guardar_archivo,
            comandos::copiar_archivo,
            comandos::whatsapp_archivo,
            comandos::compartir_archivo,
            comandos::reproducir_archivo,
            comandos::mostrar_archivo,
            comandos::estado_sesiones,
            comandos::iniciar_sesion,
            comandos::cerrar_sesion,
            comandos::plataforma_de,
            comandos::guardar_lista,
            comandos::copiar_lista,
            comandos::whatsapp_lista,
            comandos::compartir_lista,
            comandos::eliminar_lista,
            comandos::abrir_carpeta,
            comandos::leer_spotify,
            comandos::buscar_spotify,
            comandos::cancelar_busqueda_spotify,
            comandos::descargar_spotify,
            comandos::cancelar_lista,
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
