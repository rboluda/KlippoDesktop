// Deducir artista, título y álbum de los vídeos de una lista de YouTube. En los álbumes de
// YouTube Music (listas OLAK5uy_…) los datos son buenos: el canal es el artista y el título del
// vídeo es el de la canción. En el resto se parte "Artista - Título (Official Video) [HD]" y se
// quita la morralla; puede fallar, por eso la pantalla deja corregirlo antes de descargar.

/// Nombre que Plex reconoce para los recopilatorios
pub const VARIOS: &str = "Various Artists";

#[derive(Debug, PartialEq)]
pub struct Deducidas {
    pub artista: String,
    pub titulo: String,
    /// Datos de YouTube Music (fiables) o sacados del título del vídeo (conviene mirarlos)
    pub fiable: bool,
}

/// Palabras que, solas dentro de un paréntesis, no son parte del título
const MORRALLA: [&str; 30] = [
    "official", "oficial", "video", "vídeo", "audio", "music", "musical", "lyric", "lyrics", "letra",
    "visualizer", "videoclip", "clip", "hd", "hq", "4k", "1080p", "720p", "with", "con", "en", "full",
    "album", "álbum", "completo", "complete", "playlist", "lista", "explicit", "version",
];

fn es_morralla(segmento: &str) -> bool {
    let palabras: Vec<String> = segmento
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty())
        .map(str::to_lowercase)
        .collect();
    // "version" sola no quita "(Radio Version)": hace falta que todo sea morralla
    !palabras.is_empty() && palabras.iter().all(|p| MORRALLA.contains(&p.as_str()))
}

/// "Official Video Remastered" → "Remastered": la morralla de los extremos (sin tocar "with",
/// "con" ni "en", que en "(with Lía Montes)" sí cuentan)
fn quitar_bordes(segmento: &str) -> String {
    let fuerte = |p: &str| {
        let p = p.to_lowercase();
        MORRALLA.contains(&p.as_str()) && !["with", "con", "en", "version"].contains(&p.as_str())
    };
    let palabras: Vec<&str> = segmento.split_whitespace().collect();
    let inicio = palabras.iter().position(|p| !fuerte(p)).unwrap_or(palabras.len());
    let fin = palabras.iter().rposition(|p| !fuerte(p)).map_or(inicio, |i| i + 1);
    palabras[inicio..fin.max(inicio)].join(" ")
}

/// Quita del título los paréntesis o corchetes de morralla ("(Official Video)", "[HD]") y, dentro
/// de los que tienen algo útil, solo la morralla: "(Live 2030 - Official Audio)" → "(Live 2030)"
pub fn limpiar(titulo: &str) -> String {
    let mut resultado = String::new();
    let mut resto = titulo;
    while let Some(inicio) = resto.find(['(', '[', '【']) {
        let apertura = resto[inicio..].chars().next().unwrap();
        let cierre = match apertura {
            '(' => ')',
            '[' => ']',
            _ => '】',
        };
        let Some(largo) = resto[inicio..].find(cierre) else { break };
        let dentro = &resto[inicio + apertura.len_utf8()..inicio + largo];
        resultado.push_str(&resto[..inicio]);
        let utiles: Vec<String> = dentro
            .split([',', '|', '/'])
            .flat_map(|s| s.split(" - "))
            .map(str::trim)
            .filter(|s| !es_morralla(s))
            .map(quitar_bordes)
            .collect();
        if !utiles.is_empty() && !utiles.iter().all(|s| s.is_empty()) {
            let (a, c) = if apertura == '【' { ('[', ']') } else { (apertura, cierre) };
            resultado.push(a);
            resultado.push_str(&utiles.join(" - "));
            resultado.push(c);
        }
        resto = &resto[inicio + largo + cierre.len_utf8()..];
    }
    resultado.push_str(resto);
    // Lo que va tras " | " suele ser el canal o el álbum
    let resultado = resultado.split(" | ").next().unwrap_or_default();
    let limpio = resultado.split_whitespace().collect::<Vec<_>>().join(" ");
    limpio.trim_matches(|c: char| c == '-' || c == '–' || c == '|' || c.is_whitespace()).replace("( ", "(").to_string()
}

/// El canal como artista: sin " - Topic" ni "VEVO" ni "Official"
pub fn artista_del_canal(canal: &str) -> String {
    let mut a = canal.trim();
    for sufijo in [" - Topic", " - Tema", "VEVO", " Official", " Oficial"] {
        if let Some(s) = a.strip_suffix(sufijo) {
            a = s.trim();
        }
    }
    a.to_string()
}

/// Artista y título de un vídeo de la lista
pub fn deducir(titulo_video: &str, canal: Option<&str>, es_album: bool) -> Deducidas {
    let canal = canal.map(str::trim).filter(|c| !c.is_empty());
    let de_topic = canal.is_some_and(|c| c.ends_with(" - Topic") || c.ends_with(" - Tema"));
    if (es_album || de_topic) && canal.is_some() {
        return Deducidas { artista: artista_del_canal(canal.unwrap()), titulo: limpiar(titulo_video), fiable: true };
    }
    for separador in [" - ", " – ", " — ", " ~ "] {
        if let Some((izquierda, derecha)) = titulo_video.split_once(separador) {
            let titulo = limpiar(derecha);
            let artista = limpiar(izquierda);
            if !titulo.is_empty() && !artista.is_empty() {
                return Deducidas { artista, titulo: quitar_comillas(&titulo), fiable: false };
            }
        }
    }
    Deducidas {
        artista: canal.map(artista_del_canal).unwrap_or_default(),
        titulo: quitar_comillas(&limpiar(titulo_video)),
        fiable: false,
    }
}

fn quitar_comillas(t: &str) -> String {
    let t = t.trim();
    for (a, c) in [('"', '"'), ('“', '”'), ('\'', '\''), ('«', '»')] {
        if let Some(dentro) = t.strip_prefix(a).and_then(|s| s.strip_suffix(c)) {
            return dentro.trim().to_string();
        }
    }
    t.to_string()
}

/// Nombre del álbum a partir del título de la lista: "Album - Descubrimiento" (YouTube Music) →
/// "Descubrimiento"; "Robots de Lata - En vivo 2030 (Full Album)" → "En vivo 2030" si el artista es Robots de Lata
pub fn album_de_la_lista(titulo_lista: &str, artista: &str) -> String {
    let mut titulo = titulo_lista.trim();
    for prefijo in ["Album - ", "Álbum - ", "EP - ", "Single - ", "Sencillo - "] {
        if let Some(r) = titulo.strip_prefix(prefijo) {
            titulo = r;
        }
    }
    let limpio = limpiar(titulo);
    if let Some((izquierda, derecha)) = limpio.split_once(" - ") {
        if !artista.is_empty() && izquierda.trim().eq_ignore_ascii_case(artista.trim()) {
            return derecha.trim().to_string();
        }
    }
    if limpio.is_empty() { titulo.to_string() } else { limpio }
}

/// Artista del álbum: el que más se repite si son casi todas suyas; si no, recopilatorio
pub fn artista_del_album(artistas: &[String]) -> String {
    let mut cuentas: Vec<(String, usize)> = Vec::new();
    for a in artistas.iter().filter(|a| !a.is_empty()) {
        match cuentas.iter_mut().find(|(b, _)| b.eq_ignore_ascii_case(a)) {
            Some((_, n)) => *n += 1,
            None => cuentas.push((a.clone(), 1)),
        }
    }
    let Some((mas, n)) = cuentas.into_iter().max_by_key(|(_, n)| *n) else { return VARIOS.into() };
    if n * 10 >= artistas.len() * 8 { mas } else { VARIOS.into() }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn limpiar_titulos() {
        assert_eq!(limpiar("Motor / Sí señor (Live 2030 - Official Audio)"), "Motor / Sí señor (Live 2030)");
        assert_eq!(limpiar("Vuelta al mundo (Official Music Video) [HD]"), "Vuelta al mundo");
        assert_eq!(limpiar("Buena suerte (Radio Edit) [Official Video]"), "Buena suerte (Radio Edit)");
        assert_eq!(limpiar("Siesta (Videoclip Oficial) | Los Tejados"), "Siesta");
        assert_eq!(limpiar("Otra vez más"), "Otra vez más");
        assert_eq!(limpiar("Song (Remastered 2011)"), "Song (Remastered 2011)");
        assert_eq!(limpiar("Song 【Official Lyric Video】"), "Song");
    }

    #[test]
    fn deducir_artista_y_titulo() {
        let d = deducir("Robots de Lata - Motor / Sí señor (Live 2030 - Official Audio)", Some("Robots de Lata"), false);
        assert_eq!(d, Deducidas { artista: "Robots de Lata".into(), titulo: "Motor / Sí señor (Live 2030)".into(), fiable: false });
        let d = deducir("Otra vez más", Some("Robots de Lata"), true);
        assert_eq!(d, Deducidas { artista: "Robots de Lata".into(), titulo: "Otra vez más".into(), fiable: true });
        let d = deducir("Absurdo", Some("Prisma - Topic"), false);
        assert_eq!(d, Deducidas { artista: "Prisma".into(), titulo: "Absurdo".into(), fiable: true });
        let d = deducir("Rapsodia de papel (Official Video Remastered)", Some("Corona Official"), false);
        assert_eq!(d.artista, "Corona");
        assert_eq!(d.titulo, "Rapsodia de papel (Remastered)");
        let d = deducir("Alba – \"Hola\" (Official Video)", None, false);
        assert_eq!((d.artista.as_str(), d.titulo.as_str()), ("Alba", "Hola"));
    }

    #[test]
    fn album_y_artista_del_album() {
        assert_eq!(album_de_la_lista("Album - Descubrimiento", "Robots de Lata"), "Descubrimiento");
        assert_eq!(album_de_la_lista("Robots de Lata - En vivo 2030 (Full Album)", "Robots de Lata"), "En vivo 2030");
        assert_eq!(album_de_la_lista("Mis canciones de verano", "Varios"), "Mis canciones de verano");
        let unos = vec!["Robots de Lata".to_string(); 9].into_iter().chain(["Pablo Ríos".to_string()]).collect::<Vec<_>>();
        assert_eq!(artista_del_album(&unos), "Robots de Lata");
        let mezcla = vec!["A".to_string(), "B".into(), "C".into(), "A".into()];
        assert_eq!(artista_del_album(&mezcla), VARIOS);
    }
}
