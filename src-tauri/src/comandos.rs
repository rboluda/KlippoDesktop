// Órdenes que la interfaz puede pedir (invoke)
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::cache;
use crate::entrada::{self, Entrada};
use crate::{compartir, enlaces, marcador, Arranque};
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

// ---- Del navegador a Klippo ----

/// El enlace que haya en el portapapeles (para el botón Pegar, Ctrl+V y al volver a Klippo)
#[tauri::command]
pub fn enlace_del_portapapeles() -> Option<String> {
    entrada::leer_portapapeles().and_then(|texto| enlaces::extraer_url(&texto))
}

/// Enlace llegado por el marcador o por un aviso que espera a que se elija la calidad
#[tauri::command]
pub fn tomar_enlace_pendiente(entrada: State<Entrada>) -> Option<String> {
    entrada.tomar_pendiente()
}

#[tauri::command]
pub fn instalar_marcador(app: AppHandle) -> Result<(), String> {
    marcador::abrir_pagina(&app)
}

#[tauri::command]
pub fn codigo_marcador() -> &'static str {
    marcador::CODIGO
}

/// Klippo arrancó con Windows: se queda en la bandeja sin enseñar la ventana
#[tauri::command]
pub fn arranque_oculto(arranque: State<Arranque>) -> bool {
    arranque.oculto
}

// ---- Guardar y compartir ----

/// Copia en Descargas\Klippo; devuelve dónde quedó
/// (async: copiar un vídeo grande no debe congelar la ventana)
#[tauri::command]
pub async fn guardar_descarga(descargas: Gestor<'_>, id: String) -> Result<PathBuf, String> {
    let descargas = descargas.inner().clone();
    tauri::async_runtime::spawn_blocking(move || descargas.guardar(&id)).await.map_err(texto)?
}

/// Abre el Explorador con la copia guardada seleccionada
#[tauri::command]
pub fn mostrar_guardado(app: AppHandle, descargas: Gestor, id: String) -> Result<(), String> {
    let guardado = descargas.buscar(&id).and_then(|d| d.guardado).ok_or_else(no_encontrado)?;
    app.opener().reveal_item_in_dir(guardado).map_err(texto)
}

#[tauri::command]
pub fn copiar_descarga(descargas: Gestor, id: String) -> Result<(), String> {
    compartir::copiar(&descargas.archivo(&id).ok_or_else(no_encontrado)?)
}

/// Copia el vídeo y abre WhatsApp. true = la app de escritorio, false = WhatsApp Web
#[tauri::command]
pub fn whatsapp_descarga(app: AppHandle, descargas: Gestor, id: String) -> Result<bool, String> {
    compartir::whatsapp(&app, &descargas.archivo(&id).ok_or_else(no_encontrado)?)
}

/// Menú Compartir de Windows. Las órdenes que no son async corren en el hilo de la ventana,
/// que es donde Windows exige abrirlo.
#[tauri::command]
pub fn compartir_descarga(app: AppHandle, descargas: Gestor, id: String) -> Result<(), String> {
    let descarga = descargas.buscar(&id).ok_or_else(no_encontrado)?;
    let archivo = descargas.archivo(&id).ok_or_else(no_encontrado)?;
    let titulo = descarga.titulo.unwrap_or_else(|| "Klippo".into());
    let ventana = app.get_webview_window("main").ok_or_else(no_encontrado)?;
    let hwnd = ventana.hwnd().map_err(texto)?.0 as isize;
    compartir::compartir(hwnd, &archivo, &titulo).map_err(texto)
}
