// Leer una lista de YouTube para bajarla como canciones: un solo yt-dlp con --flat-playlist
// (unos segundos, sin abrir cada vídeo) da el título de la lista y, de cada vídeo, su título,
// canal y duración. Con eso se deducen las etiquetas y se prepara la portada cuadrada.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::etiquetas::{self, VARIOS};
use crate::motor::{linea_de_error, texto, Motor};
use crate::traducir;

/// Como en las listas de vídeos: hasta 100
const MAXIMO: u32 = 100;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CancionYoutube {
    pub posicion: u32,
    pub id: String,
    pub video: String,
    pub titulo_video: String,
    pub canal: Option<String>,
    pub duracion: Option<f64>,
    pub artista: String,
    pub titulo: String,
    pub fiable: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListaYoutube {
    pub id: String,
    /// Título de la lista tal cual
    pub nombre: String,
    /// Álbum de YouTube Music (datos fiables)
    pub es_album: bool,
    pub album: String,
    pub artista_album: String,
    pub portada: Option<PathBuf>,
    pub canciones: Vec<CancionYoutube>,
    /// La lista tiene más de 100 vídeos (solo se leen los 100 primeros)
    pub total_real: Option<u32>,
}

pub fn leer(motor: &Motor, url: &str) -> Result<ListaYoutube, String> {
    let mut comando = motor.ytdlp();
    comando
        .args(["--dump-single-json", "--flat-playlist", "--yes-playlist", "--no-warnings"])
        .args(["--playlist-items", &format!("1:{MAXIMO}")])
        .arg(url)
        .stdin(Stdio::null());
    let salida = comando.output().map_err(texto)?;
    let json: Value = serde_json::from_slice(&salida.stdout).map_err(|_| {
        linea_de_error(&String::from_utf8_lossy(&salida.stderr))
            .unwrap_or_else(|| traducir("No se pudo leer la lista", "Could not read the playlist"))
    })?;
    let mut lista = interpretar(&json)?;
    lista.portada = portada(motor, &json, &lista);
    Ok(lista)
}

fn interpretar(json: &Value) -> Result<ListaYoutube, String> {
    let id = json["id"].as_str().unwrap_or("lista").to_string();
    let nombre = json["title"].as_str().map(str::trim).filter(|t| !t.is_empty()).unwrap_or("YouTube").to_string();
    let es_album = id.starts_with("OLAK5uy_");
    let mut canciones = Vec::new();
    for e in json["entries"].as_array().into_iter().flatten() {
        let (Some(vid), Some(titulo_video)) = (e["id"].as_str(), e["title"].as_str()) else { continue };
        // Vídeos privados o borrados que siguen en la lista
        if titulo_video.starts_with('[') && titulo_video.ends_with(" video]") {
            continue;
        }
        let canal = e["channel"].as_str().or(e["uploader"].as_str()).map(String::from);
        let d = etiquetas::deducir(titulo_video, canal.as_deref(), es_album);
        canciones.push(CancionYoutube {
            posicion: canciones.len() as u32 + 1,
            id: vid.to_string(),
            video: format!("https://www.youtube.com/watch?v={vid}"),
            titulo_video: titulo_video.to_string(),
            canal,
            duracion: e["duration"].as_f64(),
            artista: d.artista,
            titulo: d.titulo,
            fiable: d.fiable,
        });
    }
    if canciones.is_empty() {
        return Err(traducir("La lista está vacía o es privada", "The playlist is empty or private"));
    }
    let artistas: Vec<String> = canciones.iter().map(|c| c.artista.clone()).collect();
    let artista_album = etiquetas::artista_del_album(&artistas);
    let album = etiquetas::album_de_la_lista(&nombre, if artista_album == VARIOS { "" } else { &artista_album });
    let total_real = json["playlist_count"].as_u64().map(|n| n as u32).filter(|n| *n > MAXIMO);
    Ok(ListaYoutube { id, nombre, es_album, album, artista_album, portada: None, canciones, total_real })
}

/// Portada cuadrada: la de la lista si ya es cuadrada (álbumes de YouTube Music, hasta
/// 1200×1200); si no, la miniatura grande del primer vídeo recortada al centro
fn portada(motor: &Motor, json: &Value, lista: &ListaYoutube) -> Option<PathBuf> {
    let carpeta = crate::spotify::lector::carpeta();
    fs::create_dir_all(&carpeta).ok()?;
    let original = carpeta.join(format!("yt-{}-original.jpg", lista.id));
    let destino = carpeta.join(format!("yt-{}.jpg", lista.id));
    let cuadrada = json["thumbnails"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| t["width"].as_u64().is_some() && t["width"] == t["height"])
        .max_by_key(|t| t["width"].as_u64().unwrap_or(0))
        .and_then(|t| t["url"].as_str());
    let primero = &lista.canciones[0].id;
    let candidatas: Vec<String> = cuadrada
        .map(String::from)
        .into_iter()
        .chain([
            format!("https://i.ytimg.com/vi/{primero}/maxresdefault.jpg"),
            format!("https://i.ytimg.com/vi/{primero}/hqdefault.jpg"),
        ])
        .collect();
    let bajada = candidatas.iter().any(|url| bajar(url, &original).is_ok());
    if !bajada {
        return None;
    }
    let resultado = recortar_cuadrada(motor, &original, &destino);
    let _ = fs::remove_file(&original);
    resultado.ok()
}

pub fn bajar(url: &str, destino: &Path) -> Result<(), String> {
    let agente: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let mut respuesta = agente.get(url).call().map_err(texto)?;
    if !respuesta.status().is_success() {
        return Err(respuesta.status().to_string());
    }
    let bytes = respuesta.body_mut().with_config().limit(20 * 1024 * 1024).read_to_vec().map_err(texto)?;
    // Las miniaturas que no existen a veces llegan como una imagen gris de 120×90 (~1 KB)
    if bytes.len() < 2048 {
        return Err("vacía".into());
    }
    fs::write(destino, bytes).map_err(texto)
}

/// Recorta al centro un cuadrado del lado menor y lo guarda en JPG (para la portada)
pub fn recortar_cuadrada(motor: &Motor, origen: &Path, destino: &Path) -> Result<PathBuf, String> {
    let temporal = destino.with_extension("tmp.jpg");
    let estado = motor
        .ffmpeg()
        .args(["-v", "error", "-y", "-i"])
        .arg(origen)
        .args(["-vf", "crop='min(iw,ih)':'min(iw,ih)',scale='min(1200,iw)':-2", "-frames:v", "1", "-q:v", "2"])
        .arg(&temporal)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(texto)?;
    if !estado.success() || !temporal.exists() {
        let _ = fs::remove_file(&temporal);
        return Err(traducir("No se pudo preparar la portada", "Could not prepare the cover"));
    }
    fs::rename(&temporal, destino).map_err(texto)?;
    Ok(destino.to_path_buf())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn lista_normal_y_album() {
        let json: Value = serde_json::from_str(
            r#"{"id":"PL1","title":"Robots de Lata - En vivo 2030 (Full Album)","playlist_count":3,"entries":[
              {"id":"a","title":"Robots de Lata - Motor / Sí señor (Live 2030 - Official Audio)","channel":"Robots de Lata","duration":388},
              {"id":"b","title":"[Private video]","channel":null},
              {"id":"c","title":"Robots de Lata - Tócalo / Tecnología (Live 2030 - Official Audio)","channel":"Robots de Lata","duration":330}]}"#,
        )
        .unwrap();
        let l = interpretar(&json).unwrap();
        assert_eq!(l.canciones.len(), 2);
        assert_eq!(l.canciones[1].posicion, 2);
        assert_eq!(l.album, "En vivo 2030");
        assert_eq!(l.artista_album, "Robots de Lata");
        assert!(!l.es_album);

        let json: Value = serde_json::from_str(
            r#"{"id":"OLAK5uy_x","title":"Album - Descubrimiento","entries":[
              {"id":"a","title":"Otra vez más","channel":"Robots de Lata","duration":321}]}"#,
        )
        .unwrap();
        let l = interpretar(&json).unwrap();
        assert!(l.es_album && l.canciones[0].fiable);
        assert_eq!((l.album.as_str(), l.canciones[0].titulo.as_str()), ("Descubrimiento", "Otra vez más"));
    }
}
