// Buscar las canciones en YouTube sin lanzar un yt-dlp por canción (arrancar Python cada vez
// sería muy lento): se reparten las consultas en unos pocos procesos con varias cada uno
//   yt-dlp --dump-single-json --flat-playlist --ignore-errors --no-warnings
//          "ytsearch5:<consulta1>" "ytsearch5:<consulta2>" …
// Sale una línea JSON por consulta según terminan: su "id" es el texto buscado y "entries" los
// vídeos. Cada resultado se califica y se manda a la interfaz con el evento "spotify-resultado".
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};

use super::coincidencias::{self, Coincidencia, Resultado};
use super::lector::Cancion;
use crate::descargas::matar;
use crate::motor::Motor;

/// Procesos de yt-dlp a la vez
const PROCESOS: usize = 3;
/// Resultados que se piden por canción
const RESULTADOS: u32 = 5;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ResultadoBusqueda {
    sesion: u64,
    posicion: u32,
    coincidencia: Coincidencia,
}

#[derive(Serialize, Clone)]
struct FinBusqueda {
    sesion: u64,
}

/// Procesos de cada búsqueda en marcha (para pararlos si se cierra la pantalla)
static EN_MARCHA: Mutex<Option<HashMap<u64, Vec<u32>>>> = Mutex::new(None);

pub fn buscar(app: AppHandle, motor: Arc<Motor>, sesion: u64, canciones: Vec<Cancion>) {
    EN_MARCHA.lock().unwrap().get_or_insert_with(HashMap::new).insert(sesion, Vec::new());
    let tanda = canciones.len().div_ceil(PROCESOS).max(1);
    let grupos: Vec<Vec<Cancion>> = canciones.chunks(tanda).map(|c| c.to_vec()).collect();
    let pendientes = Arc::new(Mutex::new(grupos.len()));
    for grupo in grupos {
        let (app, motor, pendientes) = (app.clone(), motor.clone(), pendientes.clone());
        thread::spawn(move || {
            buscar_grupo(&app, &motor, sesion, &grupo);
            let mut quedan = pendientes.lock().unwrap();
            *quedan -= 1;
            if *quedan == 0 {
                EN_MARCHA.lock().unwrap().get_or_insert_with(HashMap::new).remove(&sesion);
                let _ = app.emit("spotify-fin-busqueda", FinBusqueda { sesion });
            }
        });
    }
}

pub fn cancelar(sesion: u64) {
    let procesos = EN_MARCHA.lock().unwrap().get_or_insert_with(HashMap::new).remove(&sesion);
    for pid in procesos.unwrap_or_default() {
        matar(pid);
    }
}

fn buscar_grupo(app: &AppHandle, motor: &Motor, sesion: u64, canciones: &[Cancion]) {
    // Varias canciones pueden dar la misma consulta: todas esperan esa respuesta
    let mut por_consulta: HashMap<String, Vec<&Cancion>> = HashMap::new();
    for c in canciones {
        por_consulta.entry(coincidencias::consulta(c)).or_default().push(c);
    }
    let mut respondidas: HashSet<String> = HashSet::new();
    let avisar = |c: &Cancion, resultados: &[Resultado]| {
        let _ = app.emit(
            "spotify-resultado",
            ResultadoBusqueda { sesion, posicion: c.posicion, coincidencia: coincidencias::elegir(c, resultados) },
        );
    };

    let mut comando = motor.ytdlp();
    comando.args(["--dump-single-json", "--flat-playlist", "--ignore-errors", "--no-warnings"]);
    for consulta in por_consulta.keys() {
        comando.arg(format!("ytsearch{RESULTADOS}:{consulta}"));
    }
    let hijo = comando.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn();
    if let Ok(mut hijo) = hijo {
        let sigue = {
            let mut en_marcha = EN_MARCHA.lock().unwrap();
            match en_marcha.get_or_insert_with(HashMap::new).get_mut(&sesion) {
                Some(pids) => {
                    pids.push(hijo.id());
                    true
                }
                // Se cerró la pantalla antes de arrancar
                None => false,
            }
        };
        if !sigue {
            matar(hijo.id());
        }
        let lector = BufReader::new(hijo.stdout.take().expect("stdout"));
        for linea in lector.lines().map_while(Result::ok) {
            let Ok(json) = serde_json::from_str::<Value>(&linea) else { continue };
            let Some(consulta) = json["id"].as_str() else { continue };
            let Some(esperan) = por_consulta.get(consulta) else { continue };
            let resultados = leer_resultados(&json);
            for c in esperan {
                avisar(c, &resultados);
            }
            respondidas.insert(consulta.to_string());
        }
        let _ = hijo.wait();
    }
    // Las que no respondieron (error o red) se quedan sin resultados
    for (consulta, esperan) in &por_consulta {
        if !respondidas.contains(consulta) {
            for c in esperan {
                avisar(c, &[]);
            }
        }
    }
}

/// Los vídeos de una respuesta, sin los directos en curso
fn leer_resultados(json: &Value) -> Vec<Resultado> {
    json["entries"]
        .as_array()
        .map(|entradas| {
            entradas
                .iter()
                .filter(|e| e["live_status"].as_str() != Some("is_live"))
                .filter_map(|e| {
                    Some(Resultado {
                        id: e["id"].as_str()?.to_string(),
                        titulo: e["title"].as_str()?.to_string(),
                        duracion: e["duration"].as_f64(),
                        canal: e["channel"].as_str().or(e["uploader"].as_str()).map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn resultados_sin_directos() {
        let json: Value = serde_json::from_str(
            r#"{"id":"U2 - Pride","entries":[
                {"id":"a","title":"U2 - Pride","duration":228.0,"channel":"U2VEVO","live_status":"not_live"},
                {"id":"b","title":"U2 en directo ahora","duration":null,"uploader":"Radio","live_status":"is_live"},
                {"id":"c","title":"Pride","duration":229,"uploader":"U2 - Topic"}]}"#,
        )
        .unwrap();
        let r = leer_resultados(&json);
        assert_eq!(r.len(), 2);
        assert_eq!(r[1].canal.as_deref(), Some("U2 - Topic"));
        assert_eq!(r[1].duracion, Some(229.0));
    }
}
