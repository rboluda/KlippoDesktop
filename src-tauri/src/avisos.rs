// Avisos de Windows (notificaciones): enlace copiado, descarga empezada, terminada o fallida.
// Solo salen cuando Klippo no está delante; al pulsarlos, Klippo se abre.
use std::path::Path;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_opener::OpenerExt;
use tauri_winrt_notification::{IconCrop, Toast};

use crate::compartir;
use crate::descargas::{Descarga, Descargas};
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

/// Como en Android: WhatsApp · Copiar · Guardar en el propio aviso
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
    let app_aviso = app.clone();
    let id = descarga.id.clone();
    let mut aviso = Toast::new(&id_app(app))
        .title(&titulo)
        .text1(&texto)
        .add_button("WhatsApp", "whatsapp")
        .add_button(&traducir("Copiar", "Copy"), "copiar")
        .add_button(&traducir("Guardar", "Save"), "guardar")
        .on_activated(move |accion| {
            accion_sobre_descarga(&app_aviso, &id, accion.as_deref());
            Ok(())
        });
    if let Some(miniatura) = descarga.miniatura.as_deref() {
        aviso = aviso.icon(miniatura, IconCrop::Square, "");
    }
    let _ = aviso.show();
}

fn accion_sobre_descarga(app: &AppHandle, id: &str, accion: Option<&str>) {
    let descargas = app.state::<Arc<Descargas>>();
    let archivo = descargas.archivo(id);
    let resultado = match (accion, archivo) {
        (Some("whatsapp"), Some(archivo)) => compartir::whatsapp(app, std::slice::from_ref(&archivo)).map(|_| {
            mostrar(
                app,
                &traducir("Vídeo copiado", "Video copied"),
                &traducir("Elige el chat en WhatsApp y pulsa Ctrl+V", "Pick the chat in WhatsApp and press Ctrl+V"),
                None,
            )
        }),
        (Some("copiar"), Some(archivo)) => compartir::copiar(std::slice::from_ref(&archivo)).map(|_| {
            mostrar(
                app,
                &traducir("Vídeo copiado", "Video copied"),
                &traducir("Pégalo con Ctrl+V donde quieras", "Paste it anywhere with Ctrl+V"),
                None,
            )
        }),
        (Some("guardar"), Some(_)) => descargas.guardar(id).map(|ruta| guardado(app, &ruta)),
        // El aviso en sí: Klippo con el vídeo en el editor (el audio, en la lista)
        _ => {
            mostrar_ventana(app);
            if descargas.buscar(id).is_some_and(|d| !d.es_audio) {
                let _ = app.emit("abrir-editor", id);
            }
            Ok(())
        }
    };
    if let Err(error) = resultado {
        mostrar(app, &traducir("No se pudo hacer", "Something went wrong"), &error, None);
    }
}

/// "Guardado en Descargas\Klippo"; al pulsarlo, el Explorador con el archivo
fn guardado(app: &AppHandle, ruta: &Path) {
    let app_aviso = app.clone();
    let ruta_aviso = ruta.to_path_buf();
    let _ = Toast::new(&id_app(app))
        .title(&traducir(r"Guardado en Descargas\Klippo", r"Saved to Downloads\Klippo"))
        .text1(&ruta.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
        .on_activated(move |_| {
            let _ = app_aviso.opener().reveal_item_in_dir(&ruta_aviso);
            Ok(())
        })
        .show();
}

/// "Lista descargada: 10 archivos" con el nombre de la lista; al pulsarlo, Klippo
pub fn lista_terminada(app: &AppHandle, _grupo: &str, nombre: &str, archivos: usize, es_audio: bool) {
    if ventana_delante(app) {
        return;
    }
    let titulo = if es_audio {
        traducir(&format!("Lista descargada: {archivos} canciones"), &format!("Playlist downloaded: {archivos} songs"))
    } else {
        traducir(&format!("Lista descargada: {archivos} vídeos"), &format!("Playlist downloaded: {archivos} videos"))
    };
    mostrar(app, &titulo, nombre, None);
}

/// Al acabar la última canción de una lista de Spotify: el total (no un aviso por canción)
pub fn spotify_terminada(app: &AppHandle, nombre: &str, bajadas: usize, fallidas: usize) {
    if ventana_delante(app) {
        return;
    }
    let mut titulo = traducir(&format!("Spotify: {bajadas} canciones descargadas"), &format!("Spotify: {bajadas} songs downloaded"));
    if fallidas > 0 {
        titulo.push_str(&traducir(&format!(", {fallidas} con error"), &format!(", {fallidas} failed")));
    }
    mostrar(app, &titulo, nombre, None);
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
