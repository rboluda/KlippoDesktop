// Listas de canciones pegadas como texto (sin red): una canción por línea, como
// "Artista - Título". Se quitan la numeración ("1.", "01 -", "3)"), las viñetas y la duración
// del final ("3:45", "(3:45)"), que si está se usa para comparar con YouTube. Las líneas con
// enlaces o que acaban en ":" (encabezados) no cuentan. Sale una colección como las de Spotify
// para buscarla y bajarla por el mismo camino.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::coincidencias::normalizar;
use super::enlaces::Tipo;
use super::lector::{Cancion, Coleccion};

/// Canciones como mucho (como las listas de YouTube)
pub const MAXIMO: usize = 500;

/// Separadores entre artista y título, en orden de preferencia
const SEPARADORES: [&str; 5] = ["\t", " - ", " – ", " — ", " | "];

/// Lo que va antes de otros intérpretes en el artista
const INVITADOS: [&str; 4] = [" feat. ", " feat ", " ft. ", " featuring "];

/// Lee el texto. `total_real` dice cuántas había si se pasa del máximo.
pub fn leer(texto: &str, id: &str, nombre: &str) -> Coleccion {
    let mut canciones = canciones(texto);
    let total = canciones.len();
    canciones.truncate(MAXIMO);
    Coleccion {
        tipo: Tipo::Playlist,
        id: id.to_string(),
        nombre: nombre.trim().to_string(),
        autor: None,
        portada: None,
        canciones,
        total_real: (total > MAXIMO).then_some(total as u32),
    }
}

pub fn canciones(texto: &str) -> Vec<Cancion> {
    texto
        .lines()
        .filter_map(linea)
        .enumerate()
        .map(|(i, (artistas, titulo, duracion_ms))| Cancion {
            posicion: i as u32 + 1,
            uri: uri(&artistas, &titulo),
            titulo,
            artistas,
            duracion_ms,
        })
        .collect()
}

/// "texto:<huella de artista y título>": la misma canción se reconoce aunque cambie de línea
fn uri(artistas: &[String], titulo: &str) -> String {
    let mut huella = DefaultHasher::new();
    normalizar(&format!("{} {titulo}", artistas.join(" "))).hash(&mut huella);
    format!("texto:{:016x}", huella.finish())
}

/// Una línea → (artistas, título, duración en ms; 0 si no la trae)
fn linea(linea: &str) -> Option<(Vec<String>, String, u64)> {
    let limpia = linea.trim();
    if limpia.is_empty() || limpia.contains("://") || limpia.ends_with(':') {
        return None;
    }
    let limpia = sin_numeracion(sin_vineta(limpia));
    let (limpia, duracion_ms) = sin_duracion(limpia);
    let limpia = limpia.trim_end_matches([' ', '-', '–', '—', '|', '·', '\t']).trim();

    let (artista, titulo) = SEPARADORES
        .iter()
        .find_map(|s| {
            let (a, t) = limpia.split_once(s)?;
            // Columnas de una hoja de cálculo: las dos primeras con algo
            let t = if *s == "\t" { t.split('\t').map(str::trim).find(|c| !c.is_empty()).unwrap_or("") } else { t };
            Some((a.trim(), t.trim()))
        })
        .filter(|(a, t)| !a.is_empty() && !t.is_empty())
        .unwrap_or(("", limpia));
    let titulo = sin_comillas(titulo).to_string();
    if titulo.is_empty() || !titulo.chars().any(char::is_alphanumeric) {
        return None;
    }
    Some((artistas(artista), titulo, duracion_ms))
}

/// "-", "*", "•"… seguidos de espacio
fn sin_vineta(texto: &str) -> &str {
    let mut letras = texto.chars();
    match (letras.next(), letras.next()) {
        (Some('-' | '*' | '•' | '·' | '–' | '—' | '>'), Some(c)) if c.is_whitespace() => letras.as_str().trim_start(),
        _ => texto,
    }
}

/// "1. ", "01 - ", "3) ", "#4 ", "05 " (con cero delante: "99 Luftballons" no es numeración)
fn sin_numeracion(texto: &str) -> &str {
    let resto = texto.strip_prefix('#').unwrap_or(texto);
    let cifras = resto.len() - resto.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if cifras == 0 || cifras > 3 {
        return texto;
    }
    let despues = &resto[cifras..];
    let separado = despues.trim_start();
    if let Some(sin) = separado.strip_prefix(['.', ')', '-', '–', ':']) {
        if sin.starts_with(char::is_whitespace) || despues.starts_with(['.', ')']) {
            return sin.trim_start();
        }
    }
    let con_almohadilla = resto.len() != texto.len();
    if (con_almohadilla || resto.starts_with('0')) && despues.starts_with(char::is_whitespace) {
        return separado;
    }
    texto
}

/// Quita una duración al final ("3:45", "(3:45)", "[1:02:03]") y la devuelve en ms
fn sin_duracion(texto: &str) -> (&str, u64) {
    let recortado = texto.trim_end();
    let (sin_cierre, cierre) = match recortado.chars().last() {
        Some(c @ (')' | ']')) => (&recortado[..recortado.len() - 1], Some(if c == ')' { '(' } else { '[' })),
        _ => (recortado, None),
    };
    let inicio = sin_cierre.rfind(|c: char| !(c.is_ascii_digit() || c == ':')).map_or(0, |i| i + 1);
    let Some(ms) = duracion(&sin_cierre[inicio..]) else { return (texto, 0) };
    let antes = sin_cierre[..inicio].trim_end();
    match cierre {
        Some(apertura) => match antes.strip_suffix(apertura) {
            Some(antes) => (antes, ms),
            None => (texto, 0),
        },
        // Sin paréntesis tiene que ir separada del título
        None if inicio == 0 || sin_cierre[..inicio].ends_with(char::is_whitespace) => (antes, ms),
        None => (texto, 0),
    }
}

/// "m:ss" o "h:mm:ss"
fn duracion(texto: &str) -> Option<u64> {
    let partes: Vec<&str> = texto.split(':').collect();
    if !(2..=3).contains(&partes.len()) || partes.iter().any(|p| p.is_empty() || p.len() > 2) {
        return None;
    }
    if partes[1..].iter().any(|p| p.len() != 2) {
        return None;
    }
    let numeros: Vec<u64> = partes.iter().map(|p| p.parse().ok()).collect::<Option<_>>()?;
    if numeros[1..].iter().any(|n| *n >= 60) {
        return None;
    }
    Some(numeros.iter().fold(0, |total, n| total * 60 + n) * 1000)
}

fn sin_comillas(texto: &str) -> &str {
    for (a, c) in [('"', '"'), ('“', '”'), ('«', '»'), ('\'', '\'')] {
        if let Some(dentro) = texto.strip_prefix(a).and_then(|t| t.strip_suffix(c)) {
            return dentro.trim();
        }
    }
    texto
}

/// "Uno feat. Otra" → ["Uno", "Otra"]. Las comas y "&" se dejan: suelen ser parte del nombre.
fn artistas(texto: &str) -> Vec<String> {
    let mut resto = texto.to_string();
    let mut lista = Vec::new();
    loop {
        let minusculas = resto.to_lowercase();
        let corte = INVITADOS.iter().filter_map(|i| minusculas.find(i).map(|p| (p, i.len()))).min();
        match corte {
            // Los separadores son ASCII: la posición vale igual en el texto original
            Some((p, largo)) if resto.is_char_boundary(p) => {
                lista.push(resto[..p].trim().to_string());
                resto = resto[p + largo..].to_string();
            }
            _ => break,
        }
    }
    lista.push(resto.trim().to_string());
    lista.retain(|a| !a.is_empty());
    lista
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn una(texto: &str) -> (Vec<String>, String, u64) {
        linea(texto).unwrap_or_else(|| panic!("«{texto}» debería ser una canción"))
    }

    fn v(lista: &[&str]) -> Vec<String> {
        lista.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn artista_y_titulo() {
        assert_eq!(una("Los Marineros - Viento del sur"), (v(&["Los Marineros"]), "Viento del sur".into(), 0));
        assert_eq!(una("Los Marineros – Viento del sur"), (v(&["Los Marineros"]), "Viento del sur".into(), 0));
        assert_eq!(una("Los Marineros | Viento del sur"), (v(&["Los Marineros"]), "Viento del sur".into(), 0));
        assert_eq!(una("Solo un título"), (vec![], "Solo un título".into(), 0));
        // Solo el primer separador: el resto es del título
        assert_eq!(una("Banda Ficticia - Canción - Versión larga").1, "Canción - Versión larga");
        assert_eq!(una("Banda Ficticia - \"Entre comillas\"").1, "Entre comillas");
    }

    #[test]
    fn columnas_de_hoja_de_calculo() {
        assert_eq!(una("Banda Ficticia\tCanción inventada\tÁlbum"), (v(&["Banda Ficticia"]), "Canción inventada".into(), 0));
        assert_eq!(una("Banda Ficticia\t\tCanción inventada"), (v(&["Banda Ficticia"]), "Canción inventada".into(), 0));
    }

    #[test]
    fn numeracion_y_vinetas() {
        for texto in [
            "1. Grupo Imaginario - Tema",
            "01 - Grupo Imaginario - Tema",
            "12) Grupo Imaginario - Tema",
            "3.Grupo Imaginario - Tema",
            "#4 Grupo Imaginario - Tema",
            "05 Grupo Imaginario - Tema",
            "- Grupo Imaginario - Tema",
            "• Grupo Imaginario - Tema",
            "  * Grupo Imaginario - Tema  ",
        ] {
            assert_eq!(una(texto), (v(&["Grupo Imaginario"]), "Tema".into(), 0), "{texto}");
        }
        // Un número que es parte del nombre se queda
        assert_eq!(una("99 Globos - Tema").0, v(&["99 Globos"]));
        assert_eq!(una("1999 - Tema").0, v(&["1999"]));
    }

    #[test]
    fn duracion_al_final() {
        assert_eq!(una("Grupo Imaginario - Tema 3:45").2, 225_000);
        assert_eq!(una("Grupo Imaginario - Tema (3:45)"), (v(&["Grupo Imaginario"]), "Tema".into(), 225_000));
        assert_eq!(una("Grupo Imaginario - Tema [1:02:03]").2, 3_723_000);
        assert_eq!(una("Grupo Imaginario - Tema - 4:05"), (v(&["Grupo Imaginario"]), "Tema".into(), 245_000));
        assert_eq!(una("1. Grupo Imaginario - Tema\t3:10"), (v(&["Grupo Imaginario"]), "Tema".into(), 190_000));
        // No son duraciones
        assert_eq!(una("Grupo Imaginario - Las 12:5").1, "Las 12:5");
        assert_eq!(una("Grupo Imaginario - Hora:45").1, "Hora:45");
        assert_eq!(una("Grupo Imaginario - Tema (1:75)").1, "Tema (1:75)");
    }

    #[test]
    fn invitados() {
        assert_eq!(una("Uno Inventado feat. Otra Inventada - Tema").0, v(&["Uno Inventado", "Otra Inventada"]));
        assert_eq!(una("Uno Inventado Ft. Otra - Tema").0, v(&["Uno Inventado", "Otra"]));
        // Comas y "&" se respetan
        assert_eq!(una("Pareja & Cía - Tema").0, v(&["Pareja & Cía"]));
    }

    #[test]
    fn lineas_que_no_cuentan() {
        for texto in ["", "   ", "Mi lista para el coche:", "https://www.youtube.com/watch?v=abc", "---", "1."] {
            assert_eq!(linea(texto), None, "«{texto}»");
        }
    }

    #[test]
    fn lista_completa() {
        let texto = "Para la fiesta:\n\n1. Banda Uno - Primera\n2. Banda Dos - Segunda (2:30)\n\nTercera sin artista\n";
        let c = leer(texto, "abc", "  Fiesta ");
        assert_eq!(c.nombre, "Fiesta");
        assert_eq!(c.tipo, Tipo::Playlist);
        assert_eq!(c.canciones.len(), 3);
        assert_eq!(c.canciones.iter().map(|c| c.posicion).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(c.canciones[1].duracion_ms, 150_000);
        assert!(c.canciones[0].uri.starts_with("texto:"));
        assert_eq!(c.total_real, None);
        // La misma canción, el mismo uri, esté donde esté y se escriba como se escriba
        let otra = leer("x - y\n5. BANDA UNO - primera", "abc", "Otra");
        assert_eq!(otra.canciones[1].uri, c.canciones[0].uri);
        assert_ne!(c.canciones[0].uri, c.canciones[1].uri);
    }

    #[test]
    fn maximo() {
        let texto: String = (0..MAXIMO + 3).map(|i| format!("Banda - Tema {i}\n")).collect();
        let c = leer(&texto, "abc", "Larga");
        assert_eq!(c.canciones.len(), MAXIMO);
        assert_eq!(c.total_real, Some(MAXIMO as u32 + 3));
    }
}
