// Reconocer enlaces de Spotify (sin depender de la interfaz):
// - https://open.spotify.com/[intl-xx/ o intl-xx-yy/][embed/](playlist|album|track)/<id>
// - spotify:(playlist|album|track):<id>
// - https://spotify.link/<código> (acortado: hay que seguir sus redirecciones)
// El id siempre tiene 22 letras o números. Los artistas no se aceptan.
use serde::{Deserialize, Serialize};

use crate::enlaces;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tipo {
    Playlist,
    Album,
    Track,
}

impl Tipo {
    fn de(texto: &str) -> Option<Tipo> {
        match texto.to_ascii_lowercase().as_str() {
            "playlist" => Some(Tipo::Playlist),
            "album" => Some(Tipo::Album),
            "track" => Some(Tipo::Track),
            _ => None,
        }
    }

    pub fn ruta(self) -> &'static str {
        match self {
            Tipo::Playlist => "playlist",
            Tipo::Album => "album",
            Tipo::Track => "track",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
pub struct EnlaceSpotify {
    pub tipo: Tipo,
    pub id: String,
}

fn es_id(texto: &str) -> bool {
    texto.len() == 22 && texto.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Un enlace de open.spotify.com o una URI spotify: dentro del texto
pub fn analizar(texto: &str) -> Option<EnlaceSpotify> {
    if let Some(enlace) = enlaces::extraer_url(texto).and_then(|url| analizar_url(&url)) {
        return Some(enlace);
    }
    analizar_uri(texto)
}

fn analizar_url(url: &str) -> Option<EnlaceSpotify> {
    // Solo el dominio exacto (no open.spotify.com.ejemplo.com)
    if enlaces::host(url)? != "open.spotify.com" {
        return None;
    }
    let resto = url.split_once("://")?.1;
    let ruta = resto.split_once('/').map_or("", |(_, r)| r);
    let ruta = ruta.split(['?', '#']).next().unwrap_or_default();
    let mut partes = ruta.split('/').filter(|p| !p.is_empty()).peekable();
    // Prefijo de idioma (intl-es, intl-pt-br) y modo embebido
    if partes.peek().is_some_and(|p| p.to_ascii_lowercase().starts_with("intl-")) {
        partes.next();
    }
    if partes.peek().is_some_and(|p| p.eq_ignore_ascii_case("embed")) {
        partes.next();
    }
    let tipo = Tipo::de(partes.next()?)?;
    let id = partes.next()?;
    es_id(id).then(|| EnlaceSpotify { tipo, id: id.to_string() })
}

fn analizar_uri(texto: &str) -> Option<EnlaceSpotify> {
    let inicio = texto.to_ascii_lowercase().find("spotify:")?;
    let mut partes = texto[inicio + "spotify:".len()..].split(':');
    let tipo = Tipo::de(partes.next()?)?;
    let id: String = partes.next()?.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    es_id(&id).then_some(EnlaceSpotify { tipo, id })
}

/// Enlace acortado de spotify.link en el texto (hay que resolverlo con la red)
pub fn corto(texto: &str) -> Option<String> {
    let url = enlaces::extraer_url(texto)?;
    let host = enlaces::host(&url)?;
    (host == "spotify.link" && url.split_once("://")?.1.split_once('/').is_some_and(|(_, c)| !c.is_empty()))
        .then_some(url)
}

/// ¿Es un enlace de Spotify que Klippo entiende (o uno acortado)?
pub fn es_spotify(texto: &str) -> bool {
    analizar(texto).is_some() || corto(texto).is_some()
}

/// Si un enlace acortado acaba en una página, el enlace de verdad va dentro
pub fn buscar_en_html(html: &str) -> Option<EnlaceSpotify> {
    let mut resto = html;
    while let Some(i) = resto.find("open.spotify.com/") {
        let candidato: String = resto[i..]
            .chars()
            .take_while(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | '<' | '>' | '\\'))
            .collect();
        if let Some(enlace) = analizar_url(&format!("https://{candidato}")) {
            return Some(enlace);
        }
        resto = &resto[i + 1..];
    }
    None
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn lista(id: &str) -> Option<EnlaceSpotify> {
        Some(EnlaceSpotify { tipo: Tipo::Playlist, id: id.into() })
    }

    #[test]
    fn enlaces_de_open_spotify() {
        let id = "3E6gcAU0fyq74s3oUomwfo";
        assert_eq!(analizar(&format!("https://open.spotify.com/playlist/{id}")), lista(id));
        assert_eq!(analizar(&format!("https://open.spotify.com/playlist/{id}?si=abc123&utm_source=copy&pi=x")), lista(id));
        assert_eq!(analizar(&format!("Escucha esto https://open.spotify.com/intl-es/playlist/{id}?si=1 ")), lista(id));
        assert_eq!(analizar(&format!("https://open.spotify.com/intl-pt-br/playlist/{id}")), lista(id));
        assert_eq!(analizar(&format!("https://open.spotify.com/embed/playlist/{id}")), lista(id));
        assert_eq!(
            analizar("https://open.spotify.com/album/0WSpHK6tinGHU4gvP8fHih"),
            Some(EnlaceSpotify { tipo: Tipo::Album, id: "0WSpHK6tinGHU4gvP8fHih".into() })
        );
        assert_eq!(
            analizar("https://open.spotify.com/track/24rv66H94QGVqIXHjA3kn6"),
            Some(EnlaceSpotify { tipo: Tipo::Track, id: "24rv66H94QGVqIXHjA3kn6".into() })
        );
    }

    #[test]
    fn uris_de_spotify() {
        assert_eq!(analizar("spotify:playlist:3E6gcAU0fyq74s3oUomwfo"), lista("3E6gcAU0fyq74s3oUomwfo"));
        assert_eq!(
            analizar("spotify:track:24rv66H94QGVqIXHjA3kn6"),
            Some(EnlaceSpotify { tipo: Tipo::Track, id: "24rv66H94QGVqIXHjA3kn6".into() })
        );
    }

    #[test]
    fn rechazos() {
        assert_eq!(analizar("https://open.spotify.com/artist/0OdUWJ0sBjDrqHygGUXeCF"), None);
        assert_eq!(analizar("https://open.spotify.com/playlist/corto"), None);
        assert_eq!(analizar("https://open.spotify.com/playlist/3E6gcAU0fyq74s3oUomwfoXX"), None);
        assert_eq!(analizar("https://open.spotify.com.ejemplo.com/playlist/3E6gcAU0fyq74s3oUomwfo"), None);
        assert_eq!(analizar("spotify:artist:0OdUWJ0sBjDrqHygGUXeCF"), None);
        assert_eq!(analizar("https://www.youtube.com/watch?v=abc"), None);
        assert!(!es_spotify("https://spotify.link.ejemplo.com/abc"));
    }

    #[test]
    fn enlaces_acortados() {
        assert_eq!(corto("Mira https://spotify.link/AbCdEf123").as_deref(), Some("https://spotify.link/AbCdEf123"));
        assert_eq!(corto("https://spotify.link/"), None);
        assert!(es_spotify("https://spotify.link/AbCdEf123"));
        let html = r#"<html><a href="https://open.spotify.com/playlist/3E6gcAU0fyq74s3oUomwfo?si=x">Abrir</a></html>"#;
        assert_eq!(buscar_en_html(html), lista("3E6gcAU0fyq74s3oUomwfo"));
        assert_eq!(buscar_en_html("<html>nada</html>"), None);
    }
}
