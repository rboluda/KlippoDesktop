// Carpeta temporal de Klippo: %TEMP%\Klippo\descargas\<id>\ con el vídeo (o el audio),
// meta.json y miniatura.jpg. Lo que tiene más de una hora se borra solo; a la carpeta de
// Descargas del usuario solo llega lo que él guarde.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::descargas::Calidad;

pub const UNA_HORA: Duration = Duration::from_secs(60 * 60);
const ARCHIVO_META: &str = "meta.json";
pub const ARCHIVO_MINIATURA: &str = "miniatura.jpg";
/// Archivos finales posibles: vídeo MP4 o solo audio M4A o MP3
const EXTENSIONES: [&str; 3] = ["mp4", "m4a", "mp3"];

pub fn raiz() -> PathBuf {
    std::env::temp_dir().join("Klippo")
}

pub fn carpeta_descargas() -> PathBuf {
    raiz().join("descargas")
}

/// meta.json de cada descarga: permite reconstruir la lista al volver a abrir Klippo
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub url: String,
    pub calidad: Calidad,
    /// Milisegundos desde 1970; ordena la lista
    pub creado: u64,
    #[serde(default)]
    pub titulo: Option<String>,
    /// Segundos
    #[serde(default)]
    pub duracion: Option<f64>,
    /// Se marca al terminar: una carpeta sin esto es una descarga que no acabó
    #[serde(default)]
    pub completada: bool,
    /// Copia guardada en Descargas\Klippo, si el usuario la pidió
    #[serde(default)]
    pub guardado: Option<PathBuf>,
    /// Los vídeos de una misma lista (o carrusel) comparten grupo: el id de la descarga original
    #[serde(default)]
    pub grupo: Option<String>,
    /// Nombre de la lista
    #[serde(default)]
    pub lista: Option<String>,
}

pub fn escribir_meta(carpeta: &Path, meta: &Meta) -> std::io::Result<()> {
    fs::create_dir_all(carpeta)?;
    fs::write(carpeta.join(ARCHIVO_META), serde_json::to_vec(meta)?)
}

pub fn leer_meta(carpeta: &Path) -> Option<Meta> {
    serde_json::from_slice(&fs::read(carpeta.join(ARCHIVO_META)).ok()?).ok()
}

/// El vídeo o el audio descargado (el más grande si hubiera varios)
pub fn archivo_final(carpeta: &Path) -> Option<PathBuf> {
    fs::read_dir(carpeta)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONES.contains(&e.to_lowercase().as_str()))
        })
        .max_by_key(|p| p.metadata().map(|m| m.len()).unwrap_or(0))
}

/// Todos los vídeos o audios de la carpeta en orden: video.mp4, o video-1, video-2… video-10
/// si es un carrusel
pub fn archivos_finales(carpeta: &Path) -> Vec<PathBuf> {
    let mut archivos: Vec<PathBuf> = fs::read_dir(carpeta)
        .map(|e| e.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    archivos.retain(|p| {
        p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| EXTENSIONES.contains(&e.to_lowercase().as_str()))
    });
    let indice = |p: &PathBuf| -> u32 {
        p.file_stem()
            .and_then(|n| n.to_str())
            .and_then(|n| n.rsplit_once('-'))
            .and_then(|(_, i)| i.parse().ok())
            .unwrap_or(0)
    };
    archivos.sort_by_key(indice);
    archivos
}

/// Borra lo que tenga más de `edad_maxima`, salvo las descargas en curso.
/// Cada carpeta de descarga se trata como un bloque: se borra entera o no se toca.
pub fn limpiar(activas: &HashSet<String>, edad_maxima: Duration) {
    let limite = SystemTime::now() - edad_maxima;
    // Vídeos exportados por el editor: archivo a archivo
    if let Ok(exportados) = fs::read_dir(crate::edicion::carpeta()) {
        for archivo in exportados.flatten().map(|e| e.path()) {
            if ultima_modificacion(&archivo).is_none_or(|t| t < limite) {
                let _ = fs::remove_file(&archivo);
            }
        }
    }
    let Ok(entradas) = fs::read_dir(carpeta_descargas()) else { return };
    for carpeta in entradas.flatten().map(|e| e.path()) {
        let id = carpeta.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if activas.contains(id) {
            continue;
        }
        if ultima_modificacion(&carpeta).is_none_or(|t| t < limite) {
            let _ = fs::remove_dir_all(&carpeta);
        }
    }
}

/// Fecha del archivo más reciente de la carpeta (una descarga larga sigue "viva")
fn ultima_modificacion(ruta: &Path) -> Option<SystemTime> {
    let propia = ruta.metadata().and_then(|m| m.modified()).ok();
    if !ruta.is_dir() {
        return propia;
    }
    fs::read_dir(ruta)
        .ok()?
        .flatten()
        .filter_map(|e| ultima_modificacion(&e.path()))
        .chain(propia)
        .max()
}

/// Bytes que ocupa la carpeta temporal de Klippo
pub fn tamano(ruta: &Path) -> u64 {
    match fs::read_dir(ruta) {
        Ok(entradas) => entradas
            .flatten()
            .map(|e| {
                let p = e.path();
                if p.is_dir() { tamano(&p) } else { e.metadata().map(|m| m.len()).unwrap_or(0) }
            })
            .sum(),
        Err(_) => 0,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn meta_ida_y_vuelta() {
        let carpeta = std::env::temp_dir().join(format!("klippo-prueba-{}", std::process::id()));
        let meta = Meta {
            url: "https://youtu.be/x".into(),
            calidad: Calidad::P720,
            creado: 1,
            titulo: Some("Título con acentos ñ".into()),
            duracion: Some(12.5),
            completada: true,
            guardado: None,
            grupo: None,
            lista: None,
        };
        escribir_meta(&carpeta, &meta).unwrap();
        fs::write(carpeta.join("video.mp4"), b"12345").unwrap();
        fs::write(carpeta.join("miniatura.jpg"), b"123456789").unwrap();
        let leida = leer_meta(&carpeta).unwrap();
        assert_eq!(leida.titulo, meta.titulo);
        assert_eq!(leida.calidad, Calidad::P720);
        assert_eq!(archivo_final(&carpeta), Some(carpeta.join("video.mp4")));
        assert!(tamano(&carpeta) >= 14);
        fs::remove_dir_all(&carpeta).unwrap();
    }
}
