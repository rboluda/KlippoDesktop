// MusicBrainz (base de datos musical abierta, sin cuenta ni clave) y Cover Art Archive (sus
// portadas). Solo se consulta cuando el usuario pulsa buscar; se manda el texto buscado y,
// como pide MusicBrainz, un User-Agent con el nombre de la app y su web (nunca datos del
// usuario). Límite: 1 consulta por segundo.
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

use super::lista::bajar;
use crate::motor::texto;
use crate::traducir;

const API: &str = "https://musicbrainz.org/ws/2";
const PORTADAS: &str = "https://coverartarchive.org/release";

fn agente_mb() -> String {
    format!("Klippo/{} ( https://github.com/rboluda/KlippoDesktop )", env!("CARGO_PKG_VERSION"))
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Lanzamiento {
    pub id: String,
    pub titulo: String,
    pub artista: String,
    pub fecha: Option<String>,
    pub pais: Option<String>,
    pub pistas: Option<u32>,
    pub formato: Option<String>,
    pub tipo: Option<String>,
    pub miniatura: Option<PathBuf>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PistaMb {
    pub posicion: u32,
    pub titulo: String,
    pub artista: String,
    pub duracion_ms: Option<u64>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AlbumMb {
    pub titulo: String,
    pub artista: String,
    pub fecha: Option<String>,
    pub pistas: Vec<PistaMb>,
    pub portada: Option<PathBuf>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GrabacionMb {
    pub titulo: String,
    pub artista: String,
    pub album: Option<String>,
    pub fecha: Option<String>,
    pub duracion_ms: Option<u64>,
}

static ULTIMA: Mutex<Option<Instant>> = Mutex::new(None);

fn sin_conexion() -> String {
    traducir("No se pudo conectar con MusicBrainz", "Could not connect to MusicBrainz")
}

/// GET a la API respetando 1 consulta por segundo. Si está saturado (503, a menudo en la
/// primera consulta), se reintenta dos veces esperando un poco más cada vez.
fn consultar(ruta: &str) -> Result<Value, String> {
    let mut intento = 0;
    loop {
        match consultar_una_vez(ruta) {
            Err(Saturado) if intento < 2 => {
                intento += 1;
                thread::sleep(Duration::from_millis(1500 * intento));
            }
            Err(Saturado) => {
                return Err(traducir(
                    "MusicBrainz está saturado; prueba en unos segundos",
                    "MusicBrainz is busy; try again in a few seconds",
                ))
            }
            Ok(r) => return r,
        }
    }
}

/// MusicBrainz ha respondido 503
struct Saturado;

fn consultar_una_vez(ruta: &str) -> Result<Result<Value, String>, Saturado> {
    {
        let mut ultima = ULTIMA.lock().unwrap();
        if let Some(t) = *ultima {
            let pasado = t.elapsed();
            if pasado < Duration::from_millis(1100) {
                thread::sleep(Duration::from_millis(1100) - pasado);
            }
        }
        *ultima = Some(Instant::now());
    }
    let agente: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into();
    let Ok(mut respuesta) = agente.get(format!("{API}/{ruta}")).header("User-Agent", agente_mb()).call() else {
        return Ok(Err(sin_conexion()));
    };
    if respuesta.status().as_u16() == 503 {
        return Err(Saturado);
    }
    if !respuesta.status().is_success() {
        return Ok(Err(sin_conexion()));
    }
    let cuerpo = respuesta.body_mut().with_config().limit(10 * 1024 * 1024).read_to_string();
    Ok(cuerpo.ok().and_then(|c| serde_json::from_str(&c).ok()).ok_or_else(sin_conexion))
}

/// Texto entre comillas para la búsqueda (Lucene)
fn frase(t: &str) -> String {
    let limpio: String = t.chars().map(|c| if matches!(c, '"' | '\\') { ' ' } else { c }).collect();
    format!("\"{}\"", limpio.trim())
}

fn codificar(t: &str) -> String {
    t.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn consulta(campo: &str, valor: &str, artista: &str) -> String {
    let mut q = format!("{campo}:{}", frase(valor));
    if !artista.trim().is_empty() && artista.trim() != super::etiquetas::VARIOS {
        q.push_str(&format!(" AND artist:{}", frase(artista)));
    }
    codificar(&q)
}

/// "Daft Punk feat. Pharrell Williams": nombres y uniones del crédito
fn credito(v: &Value) -> String {
    v.as_array()
        .map(|a| a.iter().map(|c| format!("{}{}", c["name"].as_str().unwrap_or(""), c["joinphrase"].as_str().unwrap_or(""))).collect::<String>())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn cadena(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

/// Álbumes que encajan con artista + nombre, con su miniatura (si la tienen)
pub fn buscar_albumes(artista: &str, album: &str) -> Result<Vec<Lanzamiento>, String> {
    if album.trim().is_empty() {
        return Ok(Vec::new());
    }
    let json = consultar(&format!("release/?query={}&fmt=json&limit=8", consulta("release", album, artista)))?;
    let mut lanzamientos: Vec<Lanzamiento> = json["releases"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            Some(Lanzamiento {
                id: cadena(&r["id"])?,
                titulo: cadena(&r["title"])?,
                artista: credito(&r["artist-credit"]),
                fecha: cadena(&r["date"]),
                pais: cadena(&r["country"]),
                pistas: r["track-count"].as_u64().map(|n| n as u32),
                formato: r["media"].as_array().and_then(|m| m.first()).and_then(|m| cadena(&m["format"])),
                tipo: cadena(&r["release-group"]["primary-type"]),
                miniatura: None,
            })
        })
        .collect();
    // Las miniaturas a la vez (Cover Art Archive no tiene el límite de MusicBrainz)
    let carpeta = crate::spotify::lector::carpeta();
    let _ = fs::create_dir_all(&carpeta);
    thread::scope(|s| {
        for l in lanzamientos.iter_mut() {
            let destino = carpeta.join(format!("mb-{}-250.jpg", l.id));
            s.spawn(move || {
                if destino.exists() || bajar(&format!("{PORTADAS}/{}/front-250", l.id), &destino).is_ok() {
                    l.miniatura = Some(destino);
                }
            });
        }
    });
    Ok(lanzamientos)
}

/// Un álbum: nombre, artista, fecha, sus pistas en orden (todos los discos seguidos) y la
/// portada grande
pub fn leer_album(id: &str) -> Result<AlbumMb, String> {
    if !id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(sin_conexion());
    }
    let json = consultar(&format!("release/{id}?inc=recordings+artist-credits&fmt=json"))?;
    let mut pistas = Vec::new();
    for medio in json["media"].as_array().into_iter().flatten() {
        for p in medio["tracks"].as_array().into_iter().flatten() {
            pistas.push(PistaMb {
                posicion: pistas.len() as u32 + 1,
                titulo: cadena(&p["title"]).unwrap_or_default(),
                artista: credito(&p["artist-credit"]),
                duracion_ms: p["length"].as_u64(),
            });
        }
    }
    let carpeta = crate::spotify::lector::carpeta();
    let _ = fs::create_dir_all(&carpeta);
    let destino = carpeta.join(format!("mb-{id}.jpg"));
    let portada = (destino.exists() || bajar(&format!("{PORTADAS}/{id}/front-1200"), &destino).map_err(texto).is_ok())
        .then_some(destino);
    Ok(AlbumMb {
        titulo: cadena(&json["title"]).unwrap_or_default(),
        artista: credito(&json["artist-credit"]),
        fecha: cadena(&json["date"]),
        pistas,
        portada,
    })
}

/// Grabaciones que encajan con artista + título, las de duración más parecida primero
pub fn buscar_cancion(artista: &str, titulo: &str, duracion: Option<f64>) -> Result<Vec<GrabacionMb>, String> {
    if titulo.trim().is_empty() {
        return Ok(Vec::new());
    }
    let json = consultar(&format!("recording/?query={}&fmt=json&limit=15", consulta("recording", titulo, artista)))?;
    let mut grabaciones: Vec<GrabacionMb> = json["recordings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            // De sus discos, mejor un álbum de estudio que un recopilatorio
            let lanzamientos = r["releases"].as_array().cloned().unwrap_or_default();
            let elegido = lanzamientos
                .iter()
                .find(|l| {
                    l["release-group"]["primary-type"].as_str() == Some("Album")
                        && l["release-group"]["secondary-types"].as_array().is_none_or(|s| s.is_empty())
                })
                .or(lanzamientos.first());
            Some(GrabacionMb {
                titulo: cadena(&r["title"])?,
                artista: credito(&r["artist-credit"]),
                album: elegido.and_then(|l| cadena(&l["title"])),
                fecha: cadena(&r["first-release-date"]).or_else(|| elegido.and_then(|l| cadena(&l["date"]))),
                duracion_ms: r["length"].as_u64(),
            })
        })
        .collect();
    if let Some(d) = duracion {
        let objetivo = (d * 1000.0) as i64;
        grabaciones.sort_by_key(|g| g.duracion_ms.map_or(i64::MAX, |ms| (ms as i64 - objetivo).abs()));
    }
    // Sin repetir lo mismo (la misma grabación sale en muchos discos)
    let mut vistas: Vec<(String, String, Option<String>)> = Vec::new();
    grabaciones.retain(|g| {
        let clave = (g.titulo.to_lowercase(), g.artista.to_lowercase(), g.album.as_ref().map(|a| a.to_lowercase()));
        if vistas.contains(&clave) {
            false
        } else {
            vistas.push(clave);
            true
        }
    });
    grabaciones.truncate(8);
    Ok(grabaciones)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn consultas() {
        assert_eq!(consulta("release", "Alive 2007", "Daft Punk"), "release%3A%22Alive%202007%22%20AND%20artist%3A%22Daft%20Punk%22");
        assert_eq!(consulta("release", "Mix \"verano\"", "Various Artists"), "release%3A%22Mix%20%20verano%22");
        let v: Value = serde_json::from_str(r#"[{"name":"Daft Punk","joinphrase":" feat. "},{"name":"Pharrell Williams","joinphrase":""}]"#).unwrap();
        assert_eq!(credito(&v), "Daft Punk feat. Pharrell Williams");
    }
}
