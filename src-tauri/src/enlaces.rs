// Reconocer enlaces de vídeo en texto copiado (igual que util/Enlaces.kt de Android)

const DOMINIOS_SOPORTADOS: [&str; 9] = [
    "youtube.com", "youtu.be", "tiktok.com", "x.com", "twitter.com", "instagram.com",
    "facebook.com", "fb.watch", "fb.com",
];

/// Las apps suelen copiar "Mira este vídeo https://...": se queda solo la URL
pub fn extraer_url(texto: &str) -> Option<String> {
    let minusculas = texto.to_ascii_lowercase();
    let inicio = ["https://", "http://"].iter().filter_map(|p| minusculas.find(p)).min()?;
    let url: String = texto[inicio..]
        .chars()
        .take_while(|c| !c.is_whitespace() && !matches!(c, '<' | '>' | '"' | '\''))
        .collect();
    let url = url.trim_end_matches(['.', ',', ')', ']']).to_string();
    host(&url).map(|_| url)
}

/// ¿Es de una de las plataformas conocidas?
pub fn es_soportada(url: &str) -> bool {
    host(url).is_some_and(|h| DOMINIOS_SOPORTADOS.iter().any(|d| h == *d || h.ends_with(&format!(".{d}"))))
}

/// ¿Es un enlace que es solo de una lista de YouTube (youtube.com/playlist?list=…)? Un vídeo
/// abierto dentro de una lista (watch?v=…&list=…) no lo es: se baja solo ese vídeo.
pub fn es_solo_lista(url: &str) -> bool {
    let Some(h) = host(url) else { return false };
    if !(h == "youtube.com" || h.ends_with(".youtube.com")) {
        return false;
    }
    let resto = url.trim().split_once("://").map_or("", |(_, r)| r);
    let ruta_y_consulta = resto.split_once('/').map_or("", |(_, r)| r);
    let (ruta, consulta) = ruta_y_consulta.split_once('?').unwrap_or((ruta_y_consulta, ""));
    ruta.trim_end_matches('/').eq_ignore_ascii_case("playlist")
        && consulta.split('&').any(|p| p.starts_with("list=") && p.len() > 5)
}

/// Dominio de una URL en minúsculas, sin usuario ni puerto
pub fn host(url: &str) -> Option<String> {
    let resto = url.trim().split_once("://")?.1;
    let autoridad = resto.split(['/', '?', '#']).next()?;
    let sin_usuario = autoridad.rsplit('@').next()?;
    let sin_puerto = sin_usuario.split(':').next()?;
    (!sin_puerto.is_empty()).then(|| sin_puerto.to_lowercase())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn extrae_la_url_de_un_texto() {
        assert_eq!(
            extraer_url("Mira este vídeo https://youtu.be/abc.").as_deref(),
            Some("https://youtu.be/abc")
        );
        assert_eq!(
            extraer_url("(HTTPS://www.TikTok.com/@a/video/1)").as_deref(),
            Some("HTTPS://www.TikTok.com/@a/video/1")
        );
        assert_eq!(extraer_url("sin enlace"), None);
        assert_eq!(extraer_url("https://"), None);
    }

    #[test]
    fn enlaces_que_son_solo_de_lista() {
        assert!(es_solo_lista("https://www.youtube.com/playlist?list=PL123"));
        assert!(es_solo_lista("https://music.youtube.com/playlist?list=OLAK5uy_x&si=a"));
        assert!(!es_solo_lista("https://www.youtube.com/watch?v=abc&list=PL123"));
        assert!(!es_solo_lista("https://youtu.be/abc?list=PL123"));
        assert!(!es_solo_lista("https://www.youtube.com/playlist"));
        assert!(!es_solo_lista("https://ejemplo.com/playlist?list=PL123"));
    }

    #[test]
    fn plataformas_soportadas() {
        assert!(es_soportada("https://www.youtube.com/watch?v=1"));
        assert!(es_soportada("https://m.facebook.com/x"));
        assert!(es_soportada("https://fb.watch/abc"));
        assert!(!es_soportada("https://ejemplo.com/youtube.com"));
        assert!(!es_soportada("https://notx.com/a"));
        assert_eq!(host("HTTPS://user@Www.X.com:443/a"), Some("www.x.com".into()));
    }
}
