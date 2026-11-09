// Órdenes que la interfaz puede pedir (invoke)
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::cache;
use crate::entrada::{self, Entrada};
use crate::{compartir, edicion, enlaces, marcador, sesiones, Arranque};
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
    compartir::copiar(&[descargas.archivo(&id).ok_or_else(no_encontrado)?])
}

/// Copia el vídeo y abre WhatsApp. true = la app de escritorio, false = WhatsApp Web
#[tauri::command]
pub fn whatsapp_descarga(app: AppHandle, descargas: Gestor, id: String) -> Result<bool, String> {
    compartir::whatsapp(&app, &[descargas.archivo(&id).ok_or_else(no_encontrado)?])
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
    compartir::compartir(hwnd, &[archivo], &titulo).map_err(texto)
}

// ---- Editor ----

/// Genera el vídeo editado; avisa del avance con el evento "exportacion" (de 0 a 1)
#[tauri::command]
pub async fn exportar(
    app: AppHandle,
    descargas: Gestor<'_>,
    id: String,
    parametros: edicion::Parametros,
) -> Result<PathBuf, String> {
    let motor = descargas.motor()?;
    let original = descargas.archivo(&id).ok_or_else(no_encontrado)?;
    tauri::async_runtime::spawn_blocking(move || {
        edicion::exportar(&motor, &id, &original, &parametros, |progreso| {
            let _ = app.emit("exportacion", progreso);
        })
    })
    .await
    .map_err(texto)?
}

#[tauri::command]
pub fn cancelar_exportacion() {
    edicion::cancelar();
}

// ---- Guardar y compartir un archivo de la carpeta temporal (el vídeo exportado) ----

/// Solo se aceptan archivos de la carpeta temporal de Klippo
fn de_klippo(ruta: &str) -> Result<PathBuf, String> {
    let ruta = std::fs::canonicalize(ruta).map_err(|_| no_encontrado())?;
    let raiz = std::fs::canonicalize(cache::raiz()).map_err(texto)?;
    if ruta.starts_with(&raiz) && ruta.is_file() { Ok(ruta) } else { Err(no_encontrado()) }
}

#[tauri::command]
pub async fn guardar_archivo(app: AppHandle, ruta: String) -> Result<PathBuf, String> {
    let ruta = de_klippo(&ruta)?;
    tauri::async_runtime::spawn_blocking(move || compartir::guardar(&app, &ruta)).await.map_err(texto)?
}

#[tauri::command]
pub fn copiar_archivo(ruta: String) -> Result<(), String> {
    compartir::copiar(&[de_klippo(&ruta)?])
}

#[tauri::command]
pub fn whatsapp_archivo(app: AppHandle, ruta: String) -> Result<bool, String> {
    compartir::whatsapp(&app, &[de_klippo(&ruta)?])
}

#[tauri::command]
pub fn compartir_archivo(app: AppHandle, ruta: String, titulo: String) -> Result<(), String> {
    let ruta = de_klippo(&ruta)?;
    let ventana = app.get_webview_window("main").ok_or_else(no_encontrado)?;
    let hwnd = ventana.hwnd().map_err(texto)?.0 as isize;
    compartir::compartir(hwnd, &[ruta], &titulo).map_err(texto)
}

#[tauri::command]
pub fn reproducir_archivo(app: AppHandle, ruta: String) -> Result<(), String> {
    let ruta = de_klippo(&ruta)?;
    app.opener().open_path(ruta.to_string_lossy(), None::<&str>).map_err(texto)
}

/// Abre el Explorador con el archivo seleccionado (también la copia guardada en Descargas)
#[tauri::command]
pub fn mostrar_archivo(app: AppHandle, ruta: String) -> Result<(), String> {
    let ruta = PathBuf::from(ruta);
    if !ruta.is_file() {
        return Err(no_encontrado());
    }
    app.opener().reveal_item_in_dir(ruta).map_err(texto)
}

// ---- Sesiones de Instagram, X y Facebook ----

#[tauri::command]
pub fn estado_sesiones(app: AppHandle) -> Vec<sesiones::EstadoSesion> {
    sesiones::estado(&app)
}

/// Abre la web de la plataforma en una ventana; al detectar la sesión, guarda y la cierra
/// (avisa con el evento "sesiones"). Async: en Windows, crear una ventana desde una orden
/// síncrona (que corre en el hilo de las ventanas) la bloquea.
#[tauri::command]
pub async fn iniciar_sesion(app: AppHandle, plataforma: sesiones::Plataforma) -> Result<(), String> {
    sesiones::iniciar(&app, plataforma)
}

#[tauri::command]
pub async fn cerrar_sesion(app: AppHandle, plataforma: sesiones::Plataforma) -> Vec<sesiones::EstadoSesion> {
    sesiones::cerrar(&app, plataforma);
    sesiones::estado(&app)
}

/// Plataforma (con sesión posible) de un enlace, para el botón de la descarga fallida
#[tauri::command]
pub fn plataforma_de(url: String) -> Option<sesiones::Plataforma> {
    sesiones::Plataforma::de_url(&url)
}

// ---- Listas: acciones sobre todos sus vídeos a la vez ----

/// Guarda la lista en Descargas\Klippo\<lista>\; devuelve la carpeta
#[tauri::command]
pub async fn guardar_lista(descargas: Gestor<'_>, grupo: String) -> Result<PathBuf, String> {
    let descargas = descargas.inner().clone();
    tauri::async_runtime::spawn_blocking(move || descargas.guardar_lista(&grupo)).await.map_err(texto)?
}

#[tauri::command]
pub fn copiar_lista(descargas: Gestor, grupo: String) -> Result<(), String> {
    compartir::copiar(&archivos_de_la_lista(&descargas, &grupo)?)
}

#[tauri::command]
pub fn whatsapp_lista(app: AppHandle, descargas: Gestor, grupo: String) -> Result<bool, String> {
    compartir::whatsapp(&app, &archivos_de_la_lista(&descargas, &grupo)?)
}

/// Menú Compartir de Windows con todos los archivos (en el hilo de la ventana)
#[tauri::command]
pub fn compartir_lista(app: AppHandle, descargas: Gestor, grupo: String) -> Result<(), String> {
    let archivos = archivos_de_la_lista(&descargas, &grupo)?;
    let titulo = descargas.de_la_lista(&grupo).iter().find_map(|d| d.lista.clone()).unwrap_or_else(|| "Klippo".into());
    let ventana = app.get_webview_window("main").ok_or_else(no_encontrado)?;
    let hwnd = ventana.hwnd().map_err(texto)?.0 as isize;
    compartir::compartir(hwnd, &archivos, &titulo).map_err(texto)
}

#[tauri::command]
pub fn eliminar_lista(descargas: Gestor, grupo: String) {
    descargas.eliminar_lista(&grupo);
}

/// Abre el Explorador con la carpeta (la de una lista guardada)
#[tauri::command]
pub fn abrir_carpeta(app: AppHandle, ruta: String) -> Result<(), String> {
    let ruta = PathBuf::from(ruta);
    if !ruta.is_dir() {
        return Err(no_encontrado());
    }
    app.opener().open_path(ruta.to_string_lossy(), None::<&str>).map_err(texto)
}

fn archivos_de_la_lista(descargas: &Descargas, grupo: &str) -> Result<Vec<PathBuf>, String> {
    Some(descargas.archivos_de_la_lista(grupo)).filter(|a| !a.is_empty()).ok_or_else(no_encontrado)
}
