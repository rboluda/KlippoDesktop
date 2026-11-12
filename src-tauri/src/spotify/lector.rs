// Leer una lista, un álbum o una canción de Spotify SIN cuenta ni clave de API: la página
// embebida (open.spotify.com/embed/<tipo>/<id>) trae los datos en
// <script id="__NEXT_DATA__">, en props.pageProps.state.data.entity. Solo se conecta a
// Spotify cuando el usuario pega o comparte un enlace de Spotify.
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::enlaces::{self, EnlaceSpotify, Tipo};
use crate::cache;
use crate::motor::texto;
use crate::traducir;

/// Un navegador normal (el embed no responde igual a clientes desconocidos)
const NAVEGADOR: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
/// La página normal solo trae music:song_count a navegadores de móvil
const NAVEGADOR_MOVIL: &str =
    "Mozilla/5.0 (Linux; Android 16) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Mobile Safari/537.36";
/// Sin cuenta, el embed corta las listas aquí
pub const LIMITE_EMBED: usize = 100;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cancion {
    /// Posición en la lista (desde 1)
    pub posicion: u32,
    pub titulo: String,
    pub artistas: Vec<String>,
    pub duracion_ms: u64,
    /// spotify:track:<id>
    pub uri: String,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Coleccion {
    pub tipo: Tipo,
    pub id: String,
    pub nombre: String,
    /// Autor de la lista o artista del álbum
    pub autor: Option<String>,
    /// Portada descargada en la carpeta temporal de Klippo
    pub portada: Option<PathBuf>,
    pub canciones: Vec<Cancion>,
    /// Canciones que tiene de verdad, si son más de las que deja ver el embed
    pub total_real: Option<u32>,
}

fn agente(redirecciones: u32) -> ureq::Agent {
    ureq::Agent::config_builder()
        .max_redirects(redirecciones)
        .max_redirects_will_error(false)
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into()
}

fn sin_conexion() -> String {
    traducir("No se pudo conectar con Spotify", "Could not connect to Spotify")
}

/// GET que devuelve (código, cuerpo)
fn pedir(url: &str, navegador: &str) -> Result<(u16, String), String> {
    let mut respuesta = agente(5).get(url).header("User-Agent", navegador).call().map_err(|_| sin_conexion())?;
    let codigo = respuesta.status().as_u16();
    let cuerpo = respuesta.body_mut().read_to_string().map_err(|_| sin_conexion())?;
    Ok((codigo, cuerpo))
}

/// Un enlace pegado (o acortado de spotify.link) → tipo e id. Los acortados se siguen a mano,
/// hasta 5 saltos leyendo Location; si acaban en una página, se busca el enlace dentro.
pub fn resolver(texto: &str) -> Result<EnlaceSpotify, String> {
    if let Some(enlace) = enlaces::analizar(texto) {
        return Ok(enlace);
    }
    let no_vale = || traducir("Ese enlace de Spotify no es de una lista, un álbum ni una canción", "That Spotify link isn't a playlist, album or song");
    let mut url = enlaces::corto(texto).ok_or_else(no_vale)?;
    let agente = agente(0);
    for _ in 0..5 {
        let mut respuesta = agente.get(&url).header("User-Agent", NAVEGADOR).call().map_err(|_| sin_conexion())?;
        if respuesta.status().is_redirection() {
            let destino = respuesta
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or_else(no_vale)?
                .to_string();
            if let Some(enlace) = enlaces::analizar(&destino) {
                return Ok(enlace);
            }
            url = destino;
            continue;
        }
        let html = respuesta.body_mut().read_to_string().unwrap_or_default();
        return enlaces::buscar_en_html(&html).ok_or_else(no_vale);
    }
    Err(no_vale())
}

/// Lee la colección y descarga su portada
pub fn leer(enlace: &EnlaceSpotify) -> Result<Coleccion, String> {
    let (codigo, html) = pedir(&format!("https://open.spotify.com/embed/{}/{}", enlace.tipo.ruta(), enlace.id), NAVEGADOR)?;
    if codigo == 404 {
        return Err(no_existe());
    }
    let (mut coleccion, portada_url) = interpretar(&html, enlace)?;

    // El embed corta en 100: si llegan justo 100, la página normal dice cuántas hay
    if coleccion.canciones.len() == LIMITE_EMBED {
        coleccion.total_real = pedir(&format!("https://open.spotify.com/{}/{}", enlace.tipo.ruta(), enlace.id), NAVEGADOR_MOVIL)
            .ok()
            .and_then(|(_, pagina)| total_de_la_pagina(&pagina))
            .filter(|n| *n as usize > LIMITE_EMBED);
    }
    coleccion.portada = portada_url.and_then(|url| descargar_portada(&url, enlace).ok());
    Ok(coleccion)
}

fn no_existe() -> String {
    traducir(
        "Spotify no encuentra esa lista: puede que no exista o que sea privada",
        "Spotify can't find that playlist: it may not exist or be private",
    )
}

/// <meta name="music:song_count" content="150">
fn total_de_la_pagina(html: &str) -> Option<u32> {
    let i = html.find("music:song_count")?;
    let resto = &html[i..html.len().min(i + 200)];
    let valor = resto.split("content=\"").nth(1)?;
    valor.split('"').next()?.trim().parse().ok()
}

/// Los datos del embed: la colección (sin portada) y la URL de la portada
pub fn interpretar(html: &str, enlace: &EnlaceSpotify) -> Result<(Coleccion, Option<String>), String> {
    let inicio_marca = "<script id=\"__NEXT_DATA__\" type=\"application/json\">";
    let inicio = html.find(inicio_marca).ok_or_else(formato_desconocido)? + inicio_marca.len();
    let fin = html[inicio..].find("</script>").ok_or_else(formato_desconocido)? + inicio;
    let datos: Value = serde_json::from_str(&html[inicio..fin]).map_err(|_| formato_desconocido())?;
    let props = &datos["props"]["pageProps"];
    if props["status"].as_u64() == Some(404) {
        return Err(no_existe());
    }
    let entidad = &props["state"]["data"]["entity"];
    if entidad.is_null() {
        return Err(no_existe());
    }
    let texto_de = |v: &Value| v.as_str().map(str::trim).filter(|t| !t.is_empty()).map(String::from);
    let nombre = texto_de(&entidad["name"]).or_else(|| texto_de(&entidad["title"])).unwrap_or_else(|| "Spotify".into());

    let canciones: Vec<Cancion> = match entidad["trackList"].as_array() {
        Some(pistas) => pistas
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                Some(Cancion {
                    posicion: i as u32 + 1,
                    titulo: texto_de(&p["title"])?,
                    // "Leftfield,\u{a0}DJUM DJUM": comas con espacio duro
                    artistas: p["subtitle"]
                        .as_str()
                        .unwrap_or_default()
                        .split(',')
                        .map(|a| a.replace('\u{a0}', " ").trim().to_string())
                        .filter(|a| !a.is_empty())
                        .collect(),
                    duracion_ms: p["duration"].as_u64().unwrap_or(0),
                    uri: texto_de(&p["uri"]).unwrap_or_default(),
                })
            })
            .collect(),
        // Una canción suelta: los datos están en la propia entidad
        None if enlace.tipo == Tipo::Track => vec![Cancion {
            posicion: 1,
            titulo: nombre.clone(),
            artistas: entidad["artists"]
                .as_array()
                .map(|a| a.iter().filter_map(|x| texto_de(&x["name"])).collect())
                .unwrap_or_default(),
            duracion_ms: entidad["duration"].as_u64().unwrap_or(0),
            uri: texto_de(&entidad["uri"]).unwrap_or_else(|| format!("spotify:track:{}", enlace.id)),
        }],
        None => Vec::new(),
    };
    if canciones.is_empty() {
        return Err(traducir("Esa lista no tiene canciones", "That playlist has no songs"));
    }

    let autor = texto_de(&entidad["subtitle"]).map(|a| a.replace('\u{a0}', " "));
    // Portada: la de la lista o, si no, la imagen más grande (álbumes y canciones)
    let portada = entidad["coverArt"]["sources"][0]["url"].as_str().map(String::from).or_else(|| {
        entidad["visualIdentity"]["image"]
            .as_array()?
            .iter()
            .max_by_key(|i| i["maxWidth"].as_u64().unwrap_or(0))
            .and_then(|i| i["url"].as_str().map(String::from))
    });
    Ok((
        Coleccion {
            tipo: enlace.tipo,
            id: enlace.id.clone(),
            nombre,
            autor: if enlace.tipo == Tipo::Track { None } else { autor },
            portada: None,
            canciones,
            total_real: None,
        },
        portada,
    ))
}

fn formato_desconocido() -> String {
    traducir(
        "Spotify ha respondido algo que Klippo no entiende; prueba más tarde",
        "Spotify replied with something Klippo doesn't understand; try again later",
    )
}

/// La portada a la carpeta temporal de Klippo (para mostrarla y ponerla en cada canción)
fn descargar_portada(url: &str, enlace: &EnlaceSpotify) -> Result<PathBuf, String> {
    let carpeta = carpeta();
    fs::create_dir_all(&carpeta).map_err(texto)?;
    let destino = carpeta.join(format!("{}-{}.jpg", enlace.tipo.ruta(), enlace.id));
    let mut respuesta = agente(5).get(url).header("User-Agent", NAVEGADOR).call().map_err(texto)?;
    if !respuesta.status().is_success() {
        return Err(sin_conexion());
    }
    let bytes = respuesta.body_mut().read_to_vec().map_err(texto)?;
    fs::write(&destino, bytes).map_err(texto)?;
    Ok(destino)
}

/// Portadas de Spotify (en la carpeta temporal de Klippo)
pub fn carpeta() -> PathBuf {
    cache::raiz().join("spotify")
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn embed(entidad: &str) -> String {
        format!(
            r#"<html><script id="__NEXT_DATA__" type="application/json">{{"props":{{"pageProps":{{"state":{{"data":{{"entity":{entidad}}}}}}}}}}}</script></html>"#
        )
    }

    #[test]
    fn lista_con_artistas_y_portada() {
        let html = embed(
            r#"{"type":"playlist","name":"WipEout","subtitle":"Oliver","coverArt":{"sources":[{"url":"https://img/lista.jpg"}]},
               "trackList":[{"uri":"spotify:track:1","title":"Afro Ride","subtitle":"Leftfield, DJUM DJUM","duration":552266},
                            {"uri":"spotify:track:2","title":"Firestarter","subtitle":"The Prodigy","duration":281000}]}"#,
        );
        let enlace = EnlaceSpotify { tipo: Tipo::Playlist, id: "x".repeat(22) };
        let (c, portada) = interpretar(&html, &enlace).unwrap();
        assert_eq!(c.nombre, "WipEout");
        assert_eq!(c.autor.as_deref(), Some("Oliver"));
        assert_eq!(c.canciones.len(), 2);
        assert_eq!(c.canciones[0].artistas, ["Leftfield", "DJUM DJUM"]);
        assert_eq!(c.canciones[1].posicion, 2);
        assert_eq!(portada.as_deref(), Some("https://img/lista.jpg"));
    }

    #[test]
    fn cancion_suelta_y_portada_mas_grande() {
        let html = embed(
            r#"{"type":"track","name":"Pride","uri":"spotify:track:24rv66H94QGVqIXHjA3kn6","artists":[{"name":"U2"}],"duration":228466,
               "visualIdentity":{"image":[{"maxWidth":300,"url":"https://img/300"},{"maxWidth":640,"url":"https://img/640"},{"maxWidth":64,"url":"https://img/64"}]}}"#,
        );
        let enlace = EnlaceSpotify { tipo: Tipo::Track, id: "24rv66H94QGVqIXHjA3kn6".into() };
        let (c, portada) = interpretar(&html, &enlace).unwrap();
        assert_eq!(c.canciones, [Cancion { posicion: 1, titulo: "Pride".into(), artistas: vec!["U2".into()], duracion_ms: 228466, uri: "spotify:track:24rv66H94QGVqIXHjA3kn6".into() }]);
        assert_eq!(portada.as_deref(), Some("https://img/640"));
        assert_eq!(c.autor, None);
    }

    #[test]
    fn errores_claros() {
        let enlace = EnlaceSpotify { tipo: Tipo::Playlist, id: "x".repeat(22) };
        let no_existe = r#"<script id="__NEXT_DATA__" type="application/json">{"props":{"pageProps":{"status":404}}}</script>"#;
        assert!(interpretar(no_existe, &enlace).is_err());
        assert!(interpretar(&embed(r#"{"name":"Vacía","trackList":[]}"#), &enlace).is_err());
        assert!(interpretar("<html>otra cosa</html>", &enlace).is_err());
        assert_eq!(total_de_la_pagina(r#"<meta name="music:song_count" content="150"/>"#), Some(150));
    }
}
