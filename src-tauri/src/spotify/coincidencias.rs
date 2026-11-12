// Lo parecido que es un resultado de YouTube a una canción de Spotify (sin red ni interfaz):
// título, intérprete, duración y si es otra versión (directo, cover, karaoke…).
//   Puntos = título×40 + intérprete (principal 25, otro 15) + duración×30 + canal "- Topic" 5
//            − otra versión 40
//   IDÉNTICA: título 100 % + algún intérprete + misma duración (±5 s) + no es otra versión
//   PARECIDA: título ≥ 75 %, o título ≥ 50 % con intérprete y misma duración
//   NO ENCONTRADA: el resto
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use super::lector::Cancion;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Resultado {
    /// Id del vídeo de YouTube
    pub id: String,
    pub titulo: String,
    /// Segundos (YouTube no siempre la da)
    pub duracion: Option<f64>,
    pub canal: Option<String>,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum Grado {
    Identica,
    Parecida,
    NoEncontrada,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Coincidencia {
    pub grado: Grado,
    pub elegido: Option<Resultado>,
    pub puntos: f64,
    /// Los demás resultados válidos, en orden, por si el elegido no se puede bajar
    pub alternativas: Vec<Resultado>,
}

/// Diferencia de duración que aún cuenta como "la misma"
const TOLERANCIA_S: f64 = 5.0;
/// A partir de aquí la duración ya no puntúa
const DIFERENCIA_MAXIMA_S: f64 = 30.0;

/// Palabras de relleno que no cuentan al comparar títulos
const RELLENO: [&str; 22] = [
    "the", "a", "an", "of", "and", "to", "in", "on", "for", "el", "la", "los", "las", "lo", "de", "del", "y",
    "en", "un", "una", "le", "les",
];

/// Señales de que es otra versión (en el título de YouTube)
const OTRA_VERSION: [&str; 20] = [
    "live", "directo", "en vivo", "en directo", "cover", "karaoke", "reaction", "react", "remix", "instrumental",
    "sped up", "slowed", "nightcore", "8d", "tutorial", "acoustic", "acustico", "tribute", "lesson", "concert",
];

/// minúsculas, sin tildes, "&" → "and" y solo letras y números separados por espacios
pub fn normalizar(texto: &str) -> String {
    let sin_tildes: String = texto
        .replace('&', " and ")
        .to_lowercase()
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect();
    sin_tildes
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// El título de Spotify sin "(feat. …)", "(with …)", "(… Remaster …)", "- Remastered 2009",
/// "- Radio Edit", "- Mono"…
pub fn titulo_base(titulo: &str) -> String {
    let sobra = |dentro: &str| {
        let d = dentro.trim().to_lowercase();
        d.starts_with("feat") || d.starts_with("ft.") || d.starts_with("ft ") || d.starts_with("with ")
            || ["remaster", "radio edit", "mono", "stereo", "single version", "album version"]
                .iter()
                .any(|p| d.contains(p))
            || d == "edit"
    };
    // Paréntesis y corchetes que sobran
    let mut resultado = String::new();
    let mut resto = titulo;
    while let Some(i) = resto.find(['(', '[']) {
        let cierre = if resto[i..].starts_with('(') { ')' } else { ']' };
        let Some(j) = resto[i..].find(cierre) else { break };
        let dentro = &resto[i + 1..i + j];
        resultado.push_str(&resto[..i]);
        if !sobra(dentro) {
            resultado.push_str(&resto[i..=i + j]);
        }
        resto = &resto[i + j + 1..];
    }
    resultado.push_str(resto);
    // Coletillas tras " - "
    let mut partes: Vec<&str> = resultado.split(" - ").collect();
    while partes.len() > 1 && sobra(partes.last().unwrap()) {
        partes.pop();
    }
    partes.join(" - ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// "Artista principal - Título base", lo que se busca en YouTube
pub fn consulta(cancion: &Cancion) -> String {
    let base = titulo_base(&cancion.titulo);
    match cancion.artistas.first() {
        Some(artista) => format!("{artista} - {base}"),
        None => base,
    }
}

/// De 1 (±5 s) a 0 (30 s o más), bajando de forma lineal; 0,5 si YouTube no la da
pub fn puntos_duracion(spotify_ms: u64, youtube_s: Option<f64>) -> f64 {
    let Some(youtube) = youtube_s else { return 0.5 };
    let diferencia = (spotify_ms as f64 / 1000.0 - youtube).abs();
    if diferencia <= TOLERANCIA_S {
        1.0
    } else if diferencia >= DIFERENCIA_MAXIMA_S {
        0.0
    } else {
        1.0 - (diferencia - TOLERANCIA_S) / (DIFERENCIA_MAXIMA_S - TOLERANCIA_S)
    }
}

fn misma_duracion(spotify_ms: u64, youtube_s: Option<f64>) -> bool {
    youtube_s.is_some_and(|y| (spotify_ms as f64 / 1000.0 - y).abs() <= TOLERANCIA_S)
}

/// Proporción de palabras del título base (sin relleno) que aparecen en el de YouTube
fn proporcion_titulo(spotify: &str, youtube: &str) -> f64 {
    let base = normalizar(&titulo_base(spotify));
    let palabras_yt: Vec<&str> = youtube.split(' ').collect();
    let mut palabras: Vec<&str> = base.split(' ').filter(|p| !p.is_empty() && !RELLENO.contains(p)).collect();
    if palabras.is_empty() {
        palabras = base.split(' ').filter(|p| !p.is_empty()).collect();
    }
    if palabras.is_empty() {
        return 0.0;
    }
    palabras.iter().filter(|p| palabras_yt.contains(p)).count() as f64 / palabras.len() as f64
}

/// ¿Contiene `texto` (normalizado) la expresión como palabras enteras?
fn contiene(texto: &str, expresion: &str) -> bool {
    format!(" {texto} ").contains(&format!(" {expresion} "))
}

/// Grado y puntos de un resultado para una canción
pub fn puntuar(cancion: &Cancion, r: &Resultado) -> (Grado, f64) {
    let titulo_yt = normalizar(&r.titulo);
    let titulo_sp = normalizar(&cancion.titulo);
    let titulo = proporcion_titulo(&cancion.titulo, &titulo_yt);

    // El intérprete en el título o en el canal, sin espacios ("Artista - Topic", "ArtistaVEVO")
    let junto = |t: &str| normalizar(t).replace(' ', "");
    let donde = format!("{}|{}", titulo_yt.replace(' ', ""), r.canal.as_deref().map(junto).unwrap_or_default());
    let aparece = |artista: &String| {
        let a = junto(artista);
        !a.is_empty() && donde.contains(&a)
    };
    let principal = cancion.artistas.first().is_some_and(aparece);
    let otro = cancion.artistas.iter().skip(1).any(aparece);
    let interprete = if principal { 25.0 } else if otro { 15.0 } else { 0.0 };
    let hay_interprete = principal || otro;

    let duracion = puntos_duracion(cancion.duracion_ms, r.duracion);
    let misma = misma_duracion(cancion.duracion_ms, r.duracion);
    let otra_version = OTRA_VERSION.iter().any(|v| contiene(&titulo_yt, v) && !contiene(&titulo_sp, v));
    let topic = r.canal.as_deref().is_some_and(|c| c.trim_end().ends_with("- Topic"));

    let puntos = titulo * 40.0 + interprete + duracion * 30.0 + if topic { 5.0 } else { 0.0 }
        - if otra_version { 40.0 } else { 0.0 };
    let grado = if titulo >= 1.0 && hay_interprete && misma && !otra_version {
        Grado::Identica
    } else if titulo >= 0.75 || (titulo >= 0.5 && hay_interprete && misma) {
        Grado::Parecida
    } else {
        Grado::NoEncontrada
    };
    (grado, puntos)
}

/// El mejor resultado (por grado y después por puntos) y las alternativas válidas
pub fn elegir(cancion: &Cancion, resultados: &[Resultado]) -> Coincidencia {
    let mut puntuados: Vec<(Grado, f64, &Resultado)> = resultados
        .iter()
        .map(|r| {
            let (g, p) = puntuar(cancion, r);
            (g, p, r)
        })
        .collect();
    puntuados.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
    let Some((grado, puntos, elegido)) = puntuados.first() else {
        return Coincidencia { grado: Grado::NoEncontrada, elegido: None, puntos: 0.0, alternativas: Vec::new() };
    };
    Coincidencia {
        grado: *grado,
        elegido: Some((*elegido).clone()),
        puntos: *puntos,
        alternativas: puntuados
            .iter()
            .skip(1)
            .filter(|(g, _, _)| *g != Grado::NoEncontrada)
            .map(|(_, _, r)| (*r).clone())
            .collect(),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn cancion(titulo: &str, artistas: &[&str], segundos: u64) -> Cancion {
        Cancion {
            posicion: 1,
            titulo: titulo.into(),
            artistas: artistas.iter().map(|a| a.to_string()).collect(),
            duracion_ms: segundos * 1000,
            uri: String::new(),
        }
    }

    fn video(titulo: &str, canal: &str, segundos: Option<f64>) -> Resultado {
        Resultado { id: titulo.replace(' ', ""), titulo: titulo.into(), duracion: segundos, canal: Some(canal.into()) }
    }

    #[test]
    fn duracion_con_tolerancia_y_bajada_gradual() {
        assert_eq!(puntos_duracion(200_000, Some(204.0)), 1.0);
        assert_eq!(puntos_duracion(200_000, Some(195.0)), 1.0);
        let a = puntos_duracion(200_000, Some(215.0));
        let b = puntos_duracion(200_000, Some(225.0));
        assert!(a > b && b > 0.0 && a < 1.0);
        assert_eq!(puntos_duracion(200_000, Some(230.0)), 0.0);
        assert_eq!(puntos_duracion(200_000, Some(400.0)), 0.0);
        assert_eq!(puntos_duracion(200_000, None), 0.5);
    }

    #[test]
    fn identica_con_canal_topic() {
        let c = cancion("Firestarter", &["The Prodigy"], 281);
        let (g, _) = puntuar(&c, &video("Firestarter", "The Prodigy - Topic", Some(282.0)));
        assert_eq!(g, Grado::Identica);
    }

    #[test]
    fn videoclip_con_intro_larga_sale_parecida() {
        let c = cancion("Bitter Sweet Symphony", &["The Verve"], 358);
        let (g, _) = puntuar(&c, &video("The Verve - Bitter Sweet Symphony (Official Music Video)", "TheVerveVEVO", Some(404.0)));
        assert_eq!(g, Grado::Parecida);
    }

    #[test]
    fn entre_dos_se_prefiere_la_de_igual_duracion() {
        let c = cancion("Bitter Sweet Symphony", &["The Verve"], 358);
        let clip = video("The Verve - Bitter Sweet Symphony (Official Video)", "TheVerveVEVO", Some(404.0));
        let audio = video("Bitter Sweet Symphony", "The Verve - Topic", Some(358.0));
        let elegida = elegir(&c, &[clip.clone(), audio.clone()]);
        assert_eq!(elegida.elegido, Some(audio));
        assert_eq!(elegida.grado, Grado::Identica);
        assert_eq!(elegida.alternativas, [clip]);
    }

    #[test]
    fn otras_versiones_penalizadas_salvo_que_spotify_lo_diga() {
        let c = cancion("Wonderwall", &["Oasis"], 258);
        for titulo in ["Oasis - Wonderwall (Live at Knebworth)", "Wonderwall cover Oasis", "Oasis Wonderwall Karaoke", "Oasis - Wonderwall en directo"] {
            let (g, p) = puntuar(&c, &video(titulo, "Alguien", Some(258.0)));
            assert_ne!(g, Grado::Identica, "{titulo}");
            assert!(p < puntuar(&c, &video("Oasis - Wonderwall", "Oasis", Some(258.0))).1, "{titulo}");
        }
        let en_directo = cancion("Wonderwall - Live", &["Oasis"], 258);
        assert_eq!(puntuar(&en_directo, &video("Oasis - Wonderwall (Live)", "Oasis", Some(258.0))).0, Grado::Identica);
    }

    #[test]
    fn se_ignoran_feat_y_remaster() {
        assert_eq!(titulo_base("Señorita (feat. Camila Cabello)"), "Señorita");
        assert_eq!(titulo_base("Under Pressure - Remastered 2011"), "Under Pressure");
        assert_eq!(titulo_base("Heroes (2017 Remaster)"), "Heroes");
        assert_eq!(titulo_base("Song 2 - Radio Edit"), "Song 2");
        assert_eq!(titulo_base("Help! - Mono"), "Help!");
        assert_eq!(titulo_base("Stay (with Justin Bieber)"), "Stay");
        assert_eq!(titulo_base("Pride (In The Name Of Love)"), "Pride (In The Name Of Love)");
        let c = cancion("Under Pressure - Remastered 2011", &["Queen", "David Bowie"], 248);
        assert_eq!(consulta(&c), "Queen - Under Pressure");
        assert_eq!(puntuar(&c, &video("Queen & David Bowie - Under Pressure", "Queen Official", Some(249.0))).0, Grado::Identica);
    }

    #[test]
    fn tildes_y_mayusculas_dan_igual() {
        assert_eq!(normalizar("Canción DEL Verano & Más"), "cancion del verano and mas");
        let c = cancion("Despacito", &["Luis Fonsi"], 229);
        assert_eq!(puntuar(&c, &video("LUIS FONSI - DESPACITO", "LuisFonsiVEVO", Some(230.0))).0, Grado::Identica);
        let otra = cancion("La Bicicleta", &["Carlos Vives", "Shakira"], 228);
        assert_eq!(puntuar(&otra, &video("Carlos Vives, Shakira - La Bicicleta", "CarlosVivesVEVO", Some(229.0))).0, Grado::Identica);
    }

    #[test]
    fn sin_resultados_o_sin_parecido() {
        let c = cancion("Firestarter", &["The Prodigy"], 281);
        assert_eq!(elegir(&c, &[]).grado, Grado::NoEncontrada);
        let nada = elegir(&c, &[video("Recetas de cocina fáciles", "Cocina", Some(600.0))]);
        assert_eq!(nada.grado, Grado::NoEncontrada);
        assert!(nada.alternativas.is_empty());
    }
}
