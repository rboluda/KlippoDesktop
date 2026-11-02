// Órdenes que la interfaz puede pedir (invoke)
use std::sync::Arc;

use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::cache;
use crate::descargas::{Calidad, Descarga, Descargas};
use crate::motor::{texto, ResultadoActualizacion, Versiones};
use crate::traducir;

type Gestor<'a> = State<'a, Arc<Descargas>>;

#[tauri::command]
pub fn listar_descargas(descargas: Gestor) -> Vec<Descarga> {
    descargas.listar()
}

#[tauri::command]
pub fn descargar(descargas: Gestor, url: String, calidad: Calidad) -> Result<String, String> {
    descargas.descargar(url, calidad)
}

#[tauri::command]
pub fn cancelar_descarga(descargas: Gestor, id: String) {
    descargas.cancelar(&id);
}

#[tauri::command]
pub fn reintentar_descarga(descargas: Gestor, id: String) {
    descargas.reintentar(&id);
}

#[tauri::command]
pub fn eliminar_descarga(descargas: Gestor, id: String) {
    descargas.eliminar(&id);
}

/// Abre el vídeo con el reproductor de Windows
#[tauri::command]
pub fn reproducir(app: AppHandle, descargas: Gestor, id: String) -> Result<(), String> {
    let archivo = descargas.buscar(&id).and_then(|d| d.archivo).ok_or_else(no_encontrado)?;
    app.opener().open_path(archivo.to_string_lossy(), None::<&str>).map_err(texto)
}

/// Abre el Explorador con el archivo seleccionado
#[tauri::command]
pub fn mostrar_en_carpeta(app: AppHandle, descargas: Gestor, id: String) -> Result<(), String> {
    let archivo = descargas.buscar(&id).and_then(|d| d.archivo).ok_or_else(no_encontrado)?;
    app.opener().reveal_item_in_dir(archivo).map_err(texto)
}

#[tauri::command]
pub fn info_motor(descargas: Gestor) -> Result<Versiones, String> {
    Ok(descargas.motor()?.versiones())
}

#[tauri::command]
pub async fn actualizar_motor(descargas: Gestor<'_>) -> Result<ResultadoActualizacion, String> {
    // Con yt-dlp descargando, Windows no deja sustituir el ejecutable
    if !descargas.activas().is_empty() {
        return Err(traducir(
            "Espera a que terminen las descargas",
            "Wait until the downloads finish",
        ));
    }
    let motor = descargas.motor()?;
    tauri::async_runtime::spawn_blocking(move || motor.actualizar())
        .await
        .map_err(texto)?
}

#[tauri::command]
pub fn tamano_cache() -> u64 {
    cache::tamano(&cache::raiz())
}

#[tauri::command]
pub fn vaciar_cache(descargas: Gestor) {
    descargas.vaciar();
}

fn no_encontrado() -> String {
    traducir("No se encontró el vídeo descargado", "The downloaded video could not be found")
}
