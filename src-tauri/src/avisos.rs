// Avisos de Windows (notificaciones): enlace copiado, descarga empezada, terminada o fallida.
// Solo salen cuando Klippo no está delante; al pulsarlos, Klippo se abre.
use tauri::{AppHandle, Emitter};
use tauri_winrt_notification::{IconCrop, Toast};

use crate::descargas::Descarga;
use crate::entrada::{pedir_descarga, ventana_delante};
use crate::{mostrar_ventana, traducir};

/// Identificador con el que Windows agrupa los avisos: el del acceso directo que crea el
/// instalador. En desarrollo no hay acceso directo y se usa el de PowerShell.
fn id_app(app: &AppHandle) -> String {
    if cfg!(debug_assertions) {
        Toast::POWERSHELL_APP_ID.to_string()
    } else {
        app.config().identifier.clone()
    }
}

/// "¿Descargar con Klippo?" con el botón Descargar. Pulsar el aviso en sí abre Klippo con el
/// enlace en el campo.
pub fn ofrecer_enlace(app: &AppHandle, url: String) {
    let app_aviso = app.clone();
    let _ = Toast::new(&id_app(app))
        .title(&traducir("¿Descargar con Klippo?", "Download with Klippo?"))
        .text1(&url)
        .add_button(&traducir("Descargar", "Download"), "descargar")
        .on_activated(move |accion| {
            if accion.as_deref() == Some("descargar") {
                pedir_descarga(&app_aviso, url.clone(), false);
            } else {
                mostrar_ventana(&app_aviso);
                let _ = app_aviso.emit("enlace-copiado", url.clone());
            }
            Ok(())
        })
        .show();
}

/// Lo que se ve al pulsar el marcador sin que Klippo se ponga delante
pub fn descargando(app: &AppHandle, url: &str) {
    mostrar(app, &traducir("Descargando en Klippo…", "Downloading in Klippo…"), url, None);
}

pub fn descarga_terminada(app: &AppHandle, descarga: &Descarga) {
    if ventana_delante(app) {
        return;
    }
    let titulo = if descarga.es_audio {
        traducir("Audio descargado", "Audio downloaded")
    } else {
        traducir("Vídeo descargado", "Video downloaded")
    };
    let texto = descarga.titulo.clone().unwrap_or_else(|| descarga.url.clone());
    mostrar(app, &titulo, &texto, descarga.miniatura.as_deref());
}

pub fn descarga_fallida(app: &AppHandle, url: &str, mensaje: &str) {
    if ventana_delante(app) {
        return;
    }
    let mensaje: String = mensaje.chars().take(160).collect();
    let texto = format!("{url}\n{mensaje}");
    mostrar(app, &traducir("No se pudo descargar", "Download failed"), &texto, None);
}

fn mostrar(app: &AppHandle, titulo: &str, texto: &str, imagen: Option<&std::path::Path>) {
    let app_aviso = app.clone();
    let mut aviso = Toast::new(&id_app(app)).title(titulo).text1(texto).on_activated(move |_| {
        mostrar_ventana(&app_aviso);
        Ok(())
    });
    if let Some(imagen) = imagen {
        aviso = aviso.icon(imagen, IconCrop::Square, "");
    }
    let _ = aviso.show();
}
