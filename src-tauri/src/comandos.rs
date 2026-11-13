// Órdenes que la interfaz puede pedir (invoke)
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::cache;
use crate::entrada::{self, Entrada};
use crate::{compartir, edicion, enlaces, marcador, sesiones, spotify, Arranque};
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

/// Todo el texto del portapapeles (para pegar una lista de canciones)
#[tauri::command]
pub fn texto_del_portapapeles() -> Option<String> {
    entrada::leer_portapapeles()
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
    if ruta.starts_with(&raiz) && ruta.is_file() { Ok(sin_prefijo_largo(ruta)) } else { Err(no_encontrado()) }
}

/// canonicalize devuelve "\\?\C:\…": el menú Compartir de Windows (StorageFile) no acepta esas
/// rutas ("UNABLE_TO_MASK_PATH") y otras apps tampoco al pegarlas. Se quita si es una unidad.
fn sin_prefijo_largo(ruta: PathBuf) -> PathBuf {
    let texto = ruta.to_string_lossy();
    match texto.strip_prefix(r"\\?\") {
        Some(resto) if resto.as_bytes().get(1) == Some(&b':') => PathBuf::from(resto),
        _ => ruta,
    }
}

/// `titulo`: el del vídeo original, para el nombre del archivo
#[tauri::command]
pub async fn guardar_archivo(app: AppHandle, ruta: String, titulo: Option<String>) -> Result<PathBuf, String> {
    let ruta = de_klippo(&ruta)?;
    tauri::async_runtime::spawn_blocking(move || compartir::guardar(&app, &ruta, titulo.as_deref()))
        .await
        .map_err(texto)?
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

// ---- Spotify ----

/// Lee una lista, un álbum o una canción de Spotify (resuelve también los spotify.link)
#[tauri::command]
pub async fn leer_spotify(enlace: String) -> Result<spotify::lector::Coleccion, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let enlace = spotify::lector::resolver(&enlace)?;
        spotify::lector::leer(&enlace)
    })
    .await
    .map_err(texto)?
}

/// Una lista de canciones pegada como texto (una por línea, "Artista - Título")
#[tauri::command]
pub fn leer_lista_de_texto(texto: String, id: String, nombre: String) -> spotify::lector::Coleccion {
    spotify::texto::leer(&texto, &id, &nombre)
}

/// Busca las canciones en YouTube; cada resultado llega con el evento "spotify-resultado" y
/// el final con "spotify-fin-busqueda"
#[tauri::command]
pub fn buscar_spotify(app: AppHandle, descargas: Gestor, sesion: u64, canciones: Vec<spotify::lector::Cancion>) -> Result<(), String> {
    spotify::busqueda::buscar(app, descargas.motor()?, sesion, canciones);
    Ok(())
}

/// Búsqueda a mano de una canción no encontrada: texto libre o un enlace de YouTube
#[tauri::command]
pub async fn buscar_youtube(descargas: Gestor<'_>, texto: String) -> Result<Vec<spotify::coincidencias::Resultado>, String> {
    let motor = descargas.motor()?;
    tauri::async_runtime::spawn_blocking(move || spotify::busqueda::buscar_a_mano(&motor, &texto))
        .await
        .map_err(texto_error_hilo)?
}

fn texto_error_hilo(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Se ha cerrado la pantalla de Spotify: se paran los yt-dlp de la búsqueda
#[tauri::command]
pub fn cancelar_busqueda_spotify(sesion: u64) {
    spotify::busqueda::cancelar(sesion);
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeticionSpotify {
    cancion: spotify::lector::Cancion,
    /// El vídeo elegido y las alternativas, en orden
    videos: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColeccionSpotify {
    tipo: spotify::enlaces::Tipo,
    id: String,
    nombre: String,
    portada: Option<PathBuf>,
    /// Artista del álbum (o autor de la lista), para las etiquetas de Plex
    #[serde(default)]
    autor: Option<String>,
    /// Canciones de la lista o del álbum (las que Spotify enseña)
    #[serde(default)]
    total: Option<u32>,
    /// El enlace pegado, para volver a la lista desde su tarjeta
    #[serde(default)]
    enlace: Option<String>,
}

/// Pone en cola canciones de Spotify. Las de una lista o un álbum comparten grupo (el que se
/// pasa, o uno nuevo "spotify-<id>-<momento>", que se devuelve) y nombre de la lista.
#[tauri::command]
pub fn descargar_spotify(
    descargas: Gestor,
    coleccion: ColeccionSpotify,
    canciones: Vec<PeticionSpotify>,
    calidad: Calidad,
    guardar: bool,
    plex: Option<bool>,
    grupo: Option<String>,
) -> Result<Option<String>, String> {
    let es_lista = coleccion.tipo != spotify::enlaces::Tipo::Track;
    let es_album = coleccion.tipo == spotify::enlaces::Tipo::Album;
    const VARIOS: &str = "Various Artists";
    let grupo = es_lista.then(|| {
        grupo.unwrap_or_else(|| {
            let ahora = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0);
            format!("spotify-{}-{ahora}", coleccion.id)
        })
    });
    let canciones = canciones
        .into_iter()
        .map(|p| {
            let posicion = p.cancion.posicion;
            let datos = crate::descargas::DatosSpotify {
                uri: p.cancion.uri,
                titulo: p.cancion.titulo,
                artistas: p.cancion.artistas.clone(),
                album: es_lista.then(|| coleccion.nombre.clone()),
                pista: es_lista.then_some(posicion),
                portada: coleccion.portada.clone(),
                plex: plex.unwrap_or(false).then(|| {
                    // Álbum: su artista (si es "Various Artists", recopilatorio). Lista: siempre
                    // recopilatorio. Canción suelta: su artista principal.
                    let principal = p.cancion.artistas.first().cloned().unwrap_or_default();
                    let artista_album = match (&coleccion.tipo, &coleccion.autor) {
                        (spotify::enlaces::Tipo::Album, Some(autor)) => autor.clone(),
                        (spotify::enlaces::Tipo::Playlist, _) => VARIOS.to_string(),
                        _ => principal,
                    };
                    crate::descargas::DatosPlex {
                        recopilatorio: !es_album && es_lista || artista_album.eq_ignore_ascii_case(VARIOS),
                        artista_album,
                        total: es_lista.then_some(coleccion.total).flatten(),
                    }
                }),
                videos: p.videos,
                guardar,
                fecha: None,
                artista_album: None,
                origen: coleccion.enlace.clone().filter(|_| es_lista),
            };
            (datos, posicion)
        })
        .collect();
    descargas.descargar_spotify(canciones, calidad, grupo.clone())?;
    Ok(grupo)
}

// ---- Listas de YouTube como canciones ----

/// Lee una lista de YouTube y deduce las etiquetas de cada vídeo
#[tauri::command]
pub async fn leer_lista_musica(descargas: Gestor<'_>, enlace: String) -> Result<crate::musica::lista::ListaYoutube, String> {
    let motor = descargas.motor()?;
    tauri::async_runtime::spawn_blocking(move || crate::musica::lista::leer(&motor, &enlace))
        .await
        .map_err(texto_error_hilo)?
}

/// Álbumes de MusicBrainz que encajan
#[tauri::command]
pub async fn buscar_album_mb(artista: String, album: String) -> Result<Vec<crate::musica::musicbrainz::Lanzamiento>, String> {
    tauri::async_runtime::spawn_blocking(move || crate::musica::musicbrainz::buscar_albumes(&artista, &album))
        .await
        .map_err(texto_error_hilo)?
}

/// Un álbum de MusicBrainz con sus pistas y su portada
#[tauri::command]
pub async fn leer_album_mb(id: String) -> Result<crate::musica::musicbrainz::AlbumMb, String> {
    tauri::async_runtime::spawn_blocking(move || crate::musica::musicbrainz::leer_album(&id)).await.map_err(texto_error_hilo)?
}

/// Grabaciones de MusicBrainz para una canción
#[tauri::command]
pub async fn buscar_cancion_mb(
    artista: String,
    titulo: String,
    duracion: Option<f64>,
) -> Result<Vec<crate::musica::musicbrainz::GrabacionMb>, String> {
    tauri::async_runtime::spawn_blocking(move || crate::musica::musicbrainz::buscar_cancion(&artista, &titulo, duracion))
        .await
        .map_err(texto_error_hilo)?
}

/// Una imagen elegida por el usuario como portada: copia cuadrada en la carpeta temporal
#[tauri::command]
pub async fn portada_propia(descargas: Gestor<'_>, ruta: PathBuf) -> Result<PathBuf, String> {
    let motor = descargas.motor()?;
    tauri::async_runtime::spawn_blocking(move || {
        let carpeta = spotify::lector::carpeta();
        std::fs::create_dir_all(&carpeta).map_err(texto_error_hilo)?;
        let ahora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        crate::musica::lista::recortar_cuadrada(&motor, &ruta, &carpeta.join(format!("propia-{ahora}.jpg")))
    })
    .await
    .map_err(texto_error_hilo)?
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumYoutube {
    id: String,
    album: String,
    artista_album: String,
    fecha: Option<String>,
    portada: Option<PathBuf>,
    total: u32,
    /// El enlace pegado, para volver a la lista desde su tarjeta
    #[serde(default)]
    enlace: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancionPedida {
    posicion: u32,
    id: String,
    video: String,
    artista: String,
    titulo: String,
}

/// Pone en cola canciones de una lista de YouTube con las etiquetas del formulario. Mismo
/// camino que las de Spotify: grupo "musica-<id>-<momento>", 2 a la vez, etiquetas con FFmpeg.
#[tauri::command]
pub fn descargar_musica_youtube(
    descargas: Gestor,
    album: AlbumYoutube,
    canciones: Vec<CancionPedida>,
    calidad: Calidad,
    guardar: bool,
    plex: bool,
    grupo: Option<String>,
) -> Result<String, String> {
    let grupo = grupo.unwrap_or_else(|| {
        let ahora = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
        format!("musica-{}-{ahora}", album.id)
    });
    let artista_album = album.artista_album.trim().to_string();
    let nombre_album = Some(album.album.trim().to_string()).filter(|a| !a.is_empty()).unwrap_or_else(|| "YouTube".into());
    let canciones = canciones
        .into_iter()
        .map(|c| {
            let datos = crate::descargas::DatosSpotify {
                uri: format!("youtube:{}", c.id),
                titulo: c.titulo.trim().to_string(),
                artistas: Some(c.artista.trim().to_string()).filter(|a| !a.is_empty()).into_iter().collect(),
                album: Some(nombre_album.clone()),
                pista: Some(c.posicion),
                portada: album.portada.clone(),
                videos: vec![c.video],
                guardar,
                plex: plex.then(|| crate::descargas::DatosPlex {
                    recopilatorio: artista_album.eq_ignore_ascii_case(crate::musica::etiquetas::VARIOS),
                    artista_album: if artista_album.is_empty() { crate::musica::etiquetas::VARIOS.into() } else { artista_album.clone() },
                    total: Some(album.total),
                }),
                fecha: album.fecha.clone().map(|f| f.trim().to_string()).filter(|f| !f.is_empty()),
                artista_album: Some(artista_album.clone()).filter(|a| !a.is_empty()),
                origen: album.enlace.clone(),
            };
            (datos, c.posicion)
        })
        .collect();
    descargas.descargar_spotify(canciones, calidad, Some(grupo.clone()))?;
    Ok(grupo)
}

/// Corrige las etiquetas de canciones ya bajadas, sin volver a descargarlas
#[tauri::command]
pub async fn reetiquetar_canciones(
    descargas: Gestor<'_>,
    album: crate::descargas::AlbumEditado,
    canciones: Vec<crate::descargas::CambioEtiquetas>,
    plex: bool,
) -> Result<u32, String> {
    let descargas = descargas.inner().clone();
    tauri::async_runtime::spawn_blocking(move || descargas.reetiquetar(canciones, &album, plex))
        .await
        .map_err(texto_error_hilo)?
}

/// Cancela lo que quede por bajar de una lista (lo que está en cola no llega a arrancar)
#[tauri::command]
pub fn cancelar_lista(descargas: Gestor, grupo: String) {
    descargas.cancelar_lista(&grupo);
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use windows::Storage::StorageFile;
    use windows::core::HSTRING;

    #[test]
    fn rutas_que_acepta_compartir() {
        let archivo = std::env::temp_dir().join("klippo-prueba-compartir.mp4");
        std::fs::write(&archivo, b"x").unwrap();
        let larga = std::fs::canonicalize(&archivo).unwrap();
        assert!(larga.to_string_lossy().starts_with(r"\\?\"));
        // Así fallaba el menú Compartir del editor…
        assert!(StorageFile::GetFileFromPathAsync(&HSTRING::from(larga.as_os_str())).and_then(|o| o.join()).is_err());
        // …y así funciona
        let corta = sin_prefijo_largo(larga);
        assert!(!corta.to_string_lossy().starts_with(r"\\"));
        assert!(StorageFile::GetFileFromPathAsync(&HSTRING::from(corta.as_os_str())).and_then(|o| o.join()).is_ok());
        // Las rutas de red no se tocan
        assert_eq!(sin_prefijo_largo(PathBuf::from(r"\\?\UNC\servidor\x")), PathBuf::from(r"\\?\UNC\servidor\x"));
        std::fs::remove_file(archivo).unwrap();
    }
}
