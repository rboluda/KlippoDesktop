// Exportación del editor: todo lo elegido (recorte, encuadre, textos y emojis, pixelado y
// silencio) se traduce a un único comando de FFmpeg, igual que en Klippo para Android
// (edicion/Exportacion.kt). La interfaz solo recoge los parámetros y dibuja los textos y
// emojis en PNG; aquí se monta el filter_complex y se ejecuta.
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;

use base64::Engine;
use serde::Deserialize;

use crate::cache;
use crate::motor::{texto, Motor};

/// Recorte de inicio y fin en milisegundos
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct Recorte {
    pub inicio_ms: u64,
    pub fin_ms: u64,
}

impl Recorte {
    fn duracion_ms(self) -> u64 {
        self.fin_ms.saturating_sub(self.inicio_ms)
    }
}

/// Zona relativa (0..1) al fotograma completo, ya girado como se ve en pantalla
#[derive(Deserialize, Clone, Copy)]
pub struct Zona {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
}

/// Texto o emoji ya dibujado en PNG por la interfaz, con su esquina superior izquierda en
/// píxeles del vídeo
#[derive(Deserialize)]
pub struct Superpuesto {
    /// PNG en base64
    pub png: String,
    pub x: i64,
    pub y: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parametros {
    pub recorte: Option<Recorte>,
    pub encuadre: Option<Zona>,
    #[serde(default)]
    pub superpuestos: Vec<Superpuesto>,
    #[serde(default)]
    pub pixelados: Vec<Zona>,
    #[serde(default)]
    pub silenciar: bool,
    /// Medidas del vídeo tal como se ve (ya girado) y duración
    pub ancho: u32,
    pub alto: u32,
    pub duracion_ms: u64,
    /// Texto estable que identifica estos parámetros: si ya se exportó igual, se reutiliza
    pub clave: String,
}

/// Proceso de FFmpeg de la exportación en curso (para cancelarla)
static EN_CURSO: Mutex<Option<u32>> = Mutex::new(None);

pub fn carpeta() -> PathBuf {
    cache::raiz().join("exportaciones")
}

/// Genera el vídeo editado en la carpeta temporal y devuelve su ruta. `al_progreso` recibe
/// de 0 a 1.
pub fn exportar(
    motor: &Motor,
    id: &str,
    original: &Path,
    parametros: &Parametros,
    al_progreso: impl Fn(f64),
) -> Result<PathBuf, String> {
    let mut resumen = DefaultHasher::new();
    parametros.clave.hash(&mut resumen);
    let nombre = format!("{id}_{:x}", resumen.finish());
    let carpeta = carpeta();
    fs::create_dir_all(&carpeta).map_err(texto)?;
    let destino = carpeta.join(format!("{nombre}.mp4"));
    if destino.exists() {
        return Ok(destino);
    }

    // Se escribe en un temporal y se renombra al terminar: nunca queda un MP4 a medias
    let temporal = carpeta.join(format!("{nombre}.tmp.mp4"));
    let mut pngs = Vec::new();
    let resultado = (|| {
        for (i, s) in parametros.superpuestos.iter().enumerate() {
            let png = carpeta.join(format!("{nombre}_{i}.png"));
            let bytes = base64::engine::general_purpose::STANDARD.decode(&s.png).map_err(texto)?;
            fs::write(&png, bytes).map_err(texto)?;
            pngs.push(png);
        }
        let duracion_ms = parametros.recorte.map(Recorte::duracion_ms).unwrap_or(parametros.duracion_ms);
        ejecutar(motor, &argumentos(original, &temporal, parametros, &pngs), duracion_ms, al_progreso)?;
        fs::rename(&temporal, &destino).map_err(texto)
    })();
    let _ = fs::remove_file(&temporal);
    for png in pngs {
        let _ = fs::remove_file(png);
    }
    resultado.map(|_| destino)
}

pub fn cancelar() {
    if let Some(pid) = EN_CURSO.lock().unwrap().take() {
        crate::descargas::matar(pid);
    }
}

fn argumentos(entrada: &Path, salida: &Path, p: &Parametros, pngs: &[PathBuf]) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    // -ss y -t delante de -i: se aplican solo al vídeo (no a las imágenes); al recodificar,
    // el corte es exacto al fotograma
    if let Some(r) = p.recorte {
        a.extend(["-ss".into(), segundos(r.inicio_ms), "-t".into(), segundos(r.duracion_ms())]);
    }
    a.extend(["-i".into(), entrada.to_string_lossy().into_owned()]);
    // Cada texto o emoji es una entrada más (1, 2, 3...); overlay la repite todo el vídeo
    for png in pngs {
        a.extend(["-i".into(), png.to_string_lossy().into_owned()]);
    }

    let grafo = grafo_video(p);
    match &grafo {
        None => a.extend(["-map".into(), "0:v:0".into()]),
        Some((filtro, etiqueta)) => {
            a.extend(["-filter_complex".into(), filtro.clone(), "-map".into(), format!("[{etiqueta}]")])
        }
    }
    if grafo.is_none() && p.recorte.is_none() {
        // Solo se quita el sonido: el vídeo se copia tal cual, sin recodificar (instantáneo)
        a.extend(["-c:v".into(), "copy".into()]);
    } else {
        // H.264: se reproduce y comparte en cualquier sitio
        a.extend(
            ["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-pix_fmt", "yuv420p"].map(String::from),
        );
    }
    if p.silenciar {
        a.push("-an".into());
    } else {
        a.extend(["-map", "0:a:0?", "-c:a", "aac", "-b:a", "160k"].map(String::from));
    }
    a.extend(["-movflags", "+faststart", "-f", "mp4"].map(String::from));
    a.push(salida.to_string_lossy().into_owned());
    a
}

/// Un único filter_complex con todos los pasos, en este orden: pixelado → textos y emojis
/// (ambos sobre el fotograma completo) → encuadre. Devuelve el grafo y la etiqueta final, o
/// None si no hay nada que aplicar. FFmpeg ya ha girado el fotograma según sus metadatos,
/// así que iw/ih coinciden con lo que se ve en el editor.
fn grafo_video(p: &Parametros) -> Option<(String, String)> {
    let mut pasos: Vec<String> = Vec::new();
    let mut actual = "0:v".to_string();
    let mut contador = 0;
    let mut etiqueta = || {
        contador += 1;
        format!("v{}", contador - 1)
    };

    // Pixelado: se copia el fotograma, se recorta la zona, se reduce y se vuelve a ampliar
    // sin suavizar (bloques) y se pega encima en el mismo sitio
    for zona in &p.pixelados {
        let z = ZonaEnPixeles::de(*zona, p.ancho, p.alto);
        let (base, copia, bloques, salida) = (etiqueta(), etiqueta(), etiqueta(), etiqueta());
        pasos.push(format!("[{actual}]split[{base}][{copia}]"));
        pasos.push(format!(
            "[{copia}]crop={}:{}:{}:{},scale={}:{}:flags=neighbor,scale={}:{}:flags=neighbor[{bloques}]",
            z.ancho,
            z.alto,
            z.x,
            z.y,
            (z.ancho / z.bloque).max(1),
            (z.alto / z.bloque).max(1),
            z.ancho,
            z.alto
        ));
        pasos.push(format!("[{base}][{bloques}]overlay={}:{}[{salida}]", z.x, z.y));
        actual = salida;
    }
    for (i, s) in p.superpuestos.iter().enumerate() {
        let salida = etiqueta();
        pasos.push(format!("[{actual}][{}:v]overlay={}:{}[{salida}]", i + 1, s.x, s.y));
        actual = salida;
    }
    if let Some(e) = p.encuadre {
        // Ancho y alto pares: libx264 con yuv420p no admite dimensiones impares
        let salida = etiqueta();
        pasos.push(format!(
            "[{actual}]crop=trunc(iw*{}/2)*2:trunc(ih*{}/2)*2:trunc(iw*{}):trunc(ih*{})[{salida}]",
            decimal(e.ancho),
            decimal(e.alto),
            decimal(e.x),
            decimal(e.y)
        ));
        actual = salida;
    }
    (!pasos.is_empty()).then(|| (pasos.join(";"), actual))
}

/// Zona pasada a píxeles pares del vídeo, lista para el filtro de FFmpeg
#[derive(Debug, PartialEq)]
struct ZonaEnPixeles {
    x: i64,
    y: i64,
    ancho: i64,
    alto: i64,
    bloque: i64,
}

impl ZonaEnPixeles {
    fn de(zona: Zona, ancho_video: u32, alto_video: u32) -> ZonaEnPixeles {
        let (w, h) = (ancho_video as i64, alto_video as i64);
        let par = |valor: f64| (valor / 2.0).round() as i64 * 2;
        let par_abajo = |valor: i64| valor / 2 * 2;
        let x = par(zona.x * w as f64).clamp(0, par_abajo(w - 2).max(0));
        let y = par(zona.y * h as f64).clamp(0, par_abajo(h - 2).max(0));
        // Por el redondeo, la zona no puede salirse del fotograma (crop fallaría)
        let ancho = par(zona.ancho * w as f64).clamp(2, par_abajo(w - x).max(2));
        let alto = par(zona.alto * h as f64).clamp(2, par_abajo(h - y).max(2));
        // Unos 10 bloques en el lado corto de la zona: se ve pixelado sin dejar adivinar
        let bloque = (ancho.min(alto) / 10).max(4);
        ZonaEnPixeles { x, y, ancho, alto, bloque }
    }
}

/// FFmpeg con -progress: líneas "clave=valor"; out_time_us dice por dónde va
fn ejecutar(motor: &Motor, argumentos: &[String], duracion_ms: u64, al_progreso: impl Fn(f64)) -> Result<(), String> {
    let mut hijo = motor
        .ffmpeg()
        .args(["-hide_banner", "-nostats", "-progress", "pipe:1", "-y"])
        .args(argumentos)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(texto)?;
    *EN_CURSO.lock().unwrap() = Some(hijo.id());
    let mut errores = hijo.stderr.take().expect("stderr");
    let errores = std::thread::spawn(move || {
        let mut texto = String::new();
        let _ = std::io::Read::read_to_string(&mut errores, &mut texto);
        texto
    });
    for linea in BufReader::new(hijo.stdout.take().expect("stdout")).lines().map_while(Result::ok) {
        if let Some(us) = linea.strip_prefix("out_time_us=").and_then(|v| v.trim().parse::<f64>().ok()) {
            if duracion_ms > 0 {
                al_progreso((us / 1000.0 / duracion_ms as f64).clamp(0.0, 1.0));
            }
        }
    }
    let estado = hijo.wait().map_err(texto)?;
    let cancelada = EN_CURSO.lock().unwrap().take().is_none();
    let errores = errores.join().unwrap_or_default();
    if estado.success() {
        Ok(())
    } else if cancelada {
        Err("cancelada".into())
    } else {
        // La última línea de FFmpeg suele ser la que explica el fallo
        Err(errores.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or("FFmpeg").to_string())
    }
}

fn segundos(ms: u64) -> String {
    format!("{:.3}", ms as f64 / 1000.0)
}

/// Número con punto decimal y 4 cifras
fn decimal(valor: f64) -> String {
    format!("{valor:.4}")
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn parametros() -> Parametros {
        Parametros {
            recorte: None,
            encuadre: None,
            superpuestos: Vec::new(),
            pixelados: Vec::new(),
            silenciar: false,
            ancho: 1920,
            alto: 1080,
            duracion_ms: 10_000,
            clave: String::new(),
        }
    }

    #[test]
    fn solo_silenciar_copia_el_video() {
        let p = Parametros { silenciar: true, ..parametros() };
        let a = argumentos(Path::new("e.mp4"), Path::new("s.mp4"), &p, &[]);
        assert!(a.windows(2).any(|w| w == ["-c:v", "copy"]));
        assert!(a.contains(&"-an".to_string()));
        assert!(!a.contains(&"-filter_complex".to_string()));
    }

    #[test]
    fn recorte_recodifica_y_va_delante_de_la_entrada() {
        let p = Parametros { recorte: Some(Recorte { inicio_ms: 1500, fin_ms: 4000 }), ..parametros() };
        let a = argumentos(Path::new("e.mp4"), Path::new("s.mp4"), &p, &[]);
        assert_eq!(&a[..5], ["-ss", "1.500", "-t", "2.500", "-i"]);
        assert!(a.contains(&"libx264".to_string()));
        assert!(a.windows(2).any(|w| w == ["-map", "0:a:0?"]));
    }

    #[test]
    fn grafo_en_orden_pixelado_textos_encuadre() {
        let p = Parametros {
            pixelados: vec![Zona { x: 0.1, y: 0.1, ancho: 0.2, alto: 0.2 }],
            superpuestos: vec![Superpuesto { png: String::new(), x: 10, y: 20 }],
            encuadre: Some(Zona { x: 0.25, y: 0.0, ancho: 0.5, alto: 1.0 }),
            ..parametros()
        };
        let (grafo, final_) = grafo_video(&p).unwrap();
        assert_eq!(
            grafo,
            "[0:v]split[v0][v1];\
             [v1]crop=384:216:192:108,scale=18:10:flags=neighbor,scale=384:216:flags=neighbor[v2];\
             [v0][v2]overlay=192:108[v3];\
             [v3][1:v]overlay=10:20[v4];\
             [v4]crop=trunc(iw*0.5000/2)*2:trunc(ih*1.0000/2)*2:trunc(iw*0.2500):trunc(ih*0.0000)[v5]"
        );
        assert_eq!(final_, "v5");
    }

    #[test]
    fn zona_pixelada_dentro_del_fotograma() {
        let z = ZonaEnPixeles::de(Zona { x: 0.95, y: 0.95, ancho: 0.3, alto: 0.3 }, 1280, 720);
        assert!(z.x + z.ancho <= 1280 && z.y + z.alto <= 720);
        assert_eq!(z.x % 2 + z.y % 2 + z.ancho % 2 + z.alto % 2, 0);
        assert!(z.bloque >= 4);
    }
}
