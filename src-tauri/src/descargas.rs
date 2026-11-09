// Descargas: lista compartida con la interfaz (evento "descargas"), un hilo por descarga
// con yt-dlp, comprobación de audio con reintento y miniatura local con FFmpeg.
// Las completadas se reconstruyen desde la carpeta temporal; las demás solo viven en memoria.
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::avisos;
use crate::cache::{self, Meta};
use crate::compartir;
use crate::enlaces;
use crate::formatos;
use crate::motor::{self, Motor};
use crate::sesiones;

const ARCHIVO_TITULO: &str = "titulo.txt";
/// Lado mayor de las miniaturas
const LADO_MINIATURA: u32 = 480;
/// Como mucho un aviso de progreso a la interfaz cada tanto
const INTERVALO_PROGRESO: Duration = Duration::from_millis(150);
/// Máximo de vídeos que se bajan de un carrusel
const MAX_CARRUSEL: usize = 20;
/// Máximo de elementos de una lista (que un enlace de canal no se ponga a bajar miles)
const MAX_LISTA: usize = 100;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Calidad {
    Mejor,
    P1080,
    P720,
    P480,
    /// Solo la pista de audio, en M4A (AAC, sin recodificar)
    Audio,
    /// Solo la pista de audio, en MP3 (VBR V0)
    Mp3,
}

impl Calidad {
    fn altura_max(self) -> Option<u32> {
        match self {
            Calidad::P1080 => Some(1080),
            Calidad::P720 => Some(720),
            Calidad::P480 => Some(480),
            Calidad::Mejor | Calidad::Audio | Calidad::Mp3 => None,
        }
    }

    fn solo_audio(self) -> bool {
        matches!(self, Calidad::Audio | Calidad::Mp3)
    }
}

#[derive(Serialize, Clone, PartialEq)]
#[serde(tag = "tipo", rename_all = "camelCase")]
pub enum Estado {
    /// yt-dlp está leyendo el enlace (aún no se sabe el tamaño)
    Analizando,
    Descargando,
    /// Uniendo imagen y sonido o convirtiendo el audio
    Procesando,
    Completada,
    Cancelada,
    Fallida { mensaje: String },
}

impl Estado {
    fn activa(&self) -> bool {
        matches!(self, Estado::Analizando | Estado::Descargando | Estado::Procesando)
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Descarga {
    pub id: String,
    pub url: String,
    pub calidad: Calidad,
    pub creado: u64,
    pub titulo: Option<String>,
    pub estado: Estado,
    /// De 0 a 1; None mientras no se sabe
    pub progreso: Option<f64>,
    /// Segundos que faltan
    pub eta: Option<u64>,
    pub archivo: Option<PathBuf>,
    pub miniatura: Option<PathBuf>,
    pub duracion: Option<f64>,
    pub es_audio: bool,
    /// Copia en Descargas\Klippo (si se guardó y sigue ahí)
    pub guardado: Option<PathBuf>,
    /// Vídeos de una misma lista o carrusel: id de la descarga original y nombre de la lista
    pub grupo: Option<String>,
    pub lista: Option<String>,
    /// Mientras se baja una lista: por qué elemento va y cuántos hay ("4 de 10")
    pub elemento: Option<u32>,
    pub total: Option<u32>,
}

impl Descarga {
    fn carpeta(&self) -> PathBuf {
        cache::carpeta_descargas().join(&self.id)
    }

    /// Una descarga completada a partir de su carpeta (None si no terminó)
    fn desde_disco(carpeta: &Path) -> Option<Descarga> {
        let meta = cache::leer_meta(carpeta).filter(|m| m.completada)?;
        let archivo = cache::archivo_final(carpeta)?;
        let miniatura = carpeta.join(cache::ARCHIVO_MINIATURA);
        Some(Descarga {
            id: carpeta.file_name()?.to_str()?.to_string(),
            url: meta.url,
            calidad: meta.calidad,
            creado: meta.creado,
            titulo: meta.titulo,
            estado: Estado::Completada,
            progreso: Some(1.0),
            eta: None,
            es_audio: es_audio(&archivo),
            archivo: Some(archivo),
            miniatura: miniatura.exists().then_some(miniatura),
            duracion: meta.duracion,
            guardado: meta.guardado.filter(|g| g.exists()),
            grupo: meta.grupo,
            lista: meta.lista,
            elemento: None,
            total: None,
        })
    }
}

enum Fallo {
    Cancelada,
    Error(String),
}

impl From<std::io::Error> for Fallo {
    fn from(e: std::io::Error) -> Self {
        Fallo::Error(e.to_string())
    }
}

pub struct Descargas {
    app: AppHandle,
    motor: Result<Arc<Motor>, String>,
    lista: Mutex<Vec<Descarga>>,
    /// Proceso de yt-dlp de cada descarga en curso (para cancelarla)
    procesos: Mutex<HashMap<String, u32>>,
    canceladas: Mutex<HashSet<String>>,
    ultimo_aviso: Mutex<Instant>,
}

impl Descargas {
    pub fn new(app: AppHandle, motor: Result<Motor, String>) -> Arc<Descargas> {
        let descargas = Arc::new(Descargas {
            app,
            motor: motor.map(Arc::new),
            lista: Mutex::new(Vec::new()),
            procesos: Mutex::new(HashMap::new()),
            canceladas: Mutex::new(HashSet::new()),
            ultimo_aviso: Mutex::new(Instant::now()),
        });
        // Al abrir se borra lo que tenga más de una hora, y después cada diez minutos
        let limpieza = descargas.clone();
        thread::spawn(move || loop {
            cache::limpiar(&limpieza.activas(), cache::UNA_HORA);
            limpieza.sincronizar();
            thread::sleep(Duration::from_secs(10 * 60));
        });
        descargas
    }

    pub fn motor(&self) -> Result<Arc<Motor>, String> {
        self.motor.clone()
    }

    pub fn listar(&self) -> Vec<Descarga> {
        self.lista.lock().unwrap().clone()
    }

    pub fn activas(&self) -> HashSet<String> {
        self.lista.lock().unwrap().iter().filter(|d| d.estado.activa()).map(|d| d.id.clone()).collect()
    }

    /// Sustituye las completadas por lo que hay realmente en disco (p. ej. tras la limpieza)
    pub fn sincronizar(&self) {
        let en_disco: Vec<Descarga> = fs::read_dir(cache::carpeta_descargas())
            .map(|e| e.flatten().filter_map(|e| Descarga::desde_disco(&e.path())).collect())
            .unwrap_or_default();
        {
            let mut lista = self.lista.lock().unwrap();
            lista.retain(|d| d.estado != Estado::Completada);
            let ids: HashSet<String> = lista.iter().map(|d| d.id.clone()).collect();
            lista.extend(en_disco.into_iter().filter(|d| !ids.contains(&d.id)));
            lista.sort_by(|a, b| b.creado.cmp(&a.creado));
        }
        self.avisar();
    }

    pub fn descargar(self: &Arc<Self>, url: String, calidad: Calidad) -> Result<String, String> {
        self.motor()?;
        let url = url.trim().to_string();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("No es un enlace".into());
        }
        // El id es también el nombre de la carpeta: momento de creación + un contador por si
        // llegan dos enlaces en el mismo milisegundo
        static CONTADOR: AtomicU32 = AtomicU32::new(0);
        let creado = ahora_ms();
        let id = format!("{creado}-{}", CONTADOR.fetch_add(1, Ordering::Relaxed));
        let descarga = Descarga {
            id: id.clone(),
            url,
            calidad,
            creado,
            titulo: None,
            estado: Estado::Analizando,
            progreso: None,
            eta: None,
            archivo: None,
            miniatura: None,
            duracion: None,
            es_audio: calidad.solo_audio(),
            guardado: None,
            grupo: None,
            lista: None,
            elemento: None,
            total: None,
        };
        self.lista.lock().unwrap().insert(0, descarga);
        self.avisar();
        self.lanzar(id.clone());
        Ok(id)
    }

    pub fn reintentar(self: &Arc<Self>, id: &str) {
        let puede = self.cambiar(id, |d| {
            if d.estado.activa() || d.estado == Estado::Completada {
                return false;
            }
            d.estado = Estado::Analizando;
            d.progreso = None;
            d.eta = None;
            d.elemento = None;
            d.total = None;
            true
        });
        if puede == Some(true) {
            self.avisar();
            self.lanzar(id.to_string());
        }
    }

    pub fn cancelar(&self, id: &str) {
        self.canceladas.lock().unwrap().insert(id.to_string());
        if let Some(pid) = self.procesos.lock().unwrap().get(id) {
            matar(*pid);
        }
    }

    pub fn eliminar(&self, id: &str) {
        let activa = self.buscar(id).is_some_and(|d| d.estado.activa());
        self.lista.lock().unwrap().retain(|d| d.id != id);
        self.avisar();
        if activa {
            // El hilo de la descarga borra la carpeta cuando yt-dlp termine de cerrarse
            self.cancelar(id);
        } else {
            let _ = fs::remove_dir_all(cache::carpeta_descargas().join(id));
        }
    }

    pub fn vaciar(&self) {
        cache::limpiar(&self.activas(), Duration::ZERO);
        self.sincronizar();
    }

    /// Al salir de Klippo no se deja ningún yt-dlp ni FFmpeg descargando
    pub fn cancelar_todas(&self) {
        for id in self.activas() {
            self.cancelar(&id);
        }
    }

    pub fn buscar(&self, id: &str) -> Option<Descarga> {
        self.lista.lock().unwrap().iter().find(|d| d.id == id).cloned()
    }

    /// El vídeo (o audio) de una descarga completada
    pub fn archivo(&self, id: &str) -> Option<PathBuf> {
        self.buscar(id).and_then(|d| d.archivo).filter(|a| a.exists())
    }

    /// Guarda la copia en Descargas\Klippo (una sola vez: si ya está, devuelve esa)
    pub fn guardar(&self, id: &str) -> Result<PathBuf, String> {
        let descarga = self.buscar(id).ok_or_else(no_encontrada)?;
        let carpeta = descarga.carpeta();
        if let Some(guardado) = descarga.guardado.filter(|g| g.exists()) {
            return Ok(guardado);
        }
        let archivo = descarga.archivo.filter(|a| a.exists()).ok_or_else(no_encontrada)?;
        let destino = compartir::guardar(&self.app, &archivo, descarga.titulo.as_deref())?;
        if let Some(mut meta) = cache::leer_meta(&carpeta) {
            meta.guardado = Some(destino.clone());
            let _ = cache::escribir_meta(&carpeta, &meta);
        }
        self.cambiar(id, |d| d.guardado = Some(destino.clone()));
        self.avisar();
        Ok(destino)
    }

    // ---- Listas (vídeos con el mismo grupo) ----

    /// Los elementos completados de una lista, en su orden
    pub fn de_la_lista(&self, grupo: &str) -> Vec<Descarga> {
        let mut elementos: Vec<Descarga> = self
            .lista
            .lock()
            .unwrap()
            .iter()
            .filter(|d| d.grupo.as_deref() == Some(grupo) && d.estado == Estado::Completada)
            .cloned()
            .collect();
        elementos.sort_by(|a, b| b.creado.cmp(&a.creado));
        elementos
    }

    /// Los archivos de una lista, en su orden
    pub fn archivos_de_la_lista(&self, grupo: &str) -> Vec<PathBuf> {
        self.de_la_lista(grupo).into_iter().filter_map(|d| d.archivo).filter(|a| a.exists()).collect()
    }

    /// Guarda la lista entera en Descargas\Klippo\<lista>\, cada archivo con su título.
    /// Devuelve la carpeta.
    pub fn guardar_lista(&self, grupo: &str) -> Result<PathBuf, String> {
        let elementos = self.de_la_lista(grupo);
        let nombre = elementos.iter().find_map(|d| d.lista.clone()).unwrap_or_else(|| "Klippo".into());
        let mut carpeta = None;
        for (i, d) in elementos.iter().enumerate() {
            let Some(archivo) = d.archivo.as_ref().filter(|a| a.exists()) else { continue };
            // Si ya se guardó (suelto o con la lista), no se repite
            if let Some(guardado) = d.guardado.as_ref().filter(|g| g.exists()) {
                carpeta = carpeta.or_else(|| guardado.parent().map(Path::to_path_buf));
                continue;
            }
            let titulo = d.titulo.clone().unwrap_or_else(|| format!("{} {}", nombre, i + 1));
            let destino = compartir::guardar_como(&self.app, archivo, &nombre, &titulo)?;
            carpeta = destino.parent().map(Path::to_path_buf);
            let carpeta_cache = d.carpeta();
            if let Some(mut meta) = cache::leer_meta(&carpeta_cache) {
                meta.guardado = Some(destino.clone());
                let _ = cache::escribir_meta(&carpeta_cache, &meta);
            }
            self.cambiar(&d.id, |x| x.guardado = Some(destino.clone()));
        }
        self.avisar();
        carpeta.ok_or_else(no_encontrada)
    }

    pub fn eliminar_lista(&self, grupo: &str) {
        for d in self.de_la_lista(grupo) {
            self.eliminar(&d.id);
        }
    }

    // ---- Trabajo de cada descarga ----

    fn lanzar(self: &Arc<Self>, id: String) {
        self.canceladas.lock().unwrap().remove(&id);
        let yo = self.clone();
        thread::spawn(move || yo.trabajar(&id));
    }

    fn trabajar(&self, id: &str) {
        let Some(descarga) = self.buscar(id) else { return };
        let carpeta = descarga.carpeta();
        let resultado = self.descargar_en(&descarga, &carpeta);
        self.procesos.lock().unwrap().remove(id);

        let cancelada = self.canceladas.lock().unwrap().remove(id);
        // Lo que quedara a medias no sirve: al reintentar se empieza de cero
        if self.buscar(id).is_none() {
            let _ = fs::remove_dir_all(&carpeta);
            return;
        }
        let estado = match resultado {
            Ok(()) => return,
            Err(Fallo::Error(_)) if cancelada => Estado::Cancelada,
            Err(Fallo::Cancelada) => Estado::Cancelada,
            Err(Fallo::Error(mensaje)) => Estado::Fallida { mensaje },
        };
        let _ = fs::remove_dir_all(&carpeta);
        if let Estado::Fallida { mensaje } = &estado {
            avisos::descarga_fallida(&self.app, &descarga.url, mensaje);
        }
        self.cambiar(id, |d| {
            d.estado = estado;
            d.progreso = None;
            d.eta = None;
        });
        self.avisar();
    }

    fn descargar_en(&self, descarga: &Descarga, carpeta: &Path) -> Result<(), Fallo> {
        let motor = self.motor().map_err(Fallo::Error)?;
        let _ = fs::remove_dir_all(carpeta);
        let meta = Meta {
            url: descarga.url.clone(),
            calidad: descarga.calidad,
            creado: descarga.creado,
            titulo: None,
            duracion: None,
            completada: false,
            guardado: None,
            grupo: None,
            lista: None,
        };
        cache::escribir_meta(carpeta, &meta)?;

        let calidad = descarga.calidad;
        let solo_lista = enlaces::es_solo_lista(&descarga.url);
        let formato = (!calidad.solo_audio())
            .then(|| formatos::formato(calidad.altura_max(), formatos::es_tiktok(&descarga.url)));
        self.ejecutar(&motor, descarga, carpeta, formato.as_deref())?;
        let mut archivos = cache::archivos_finales(carpeta);

        // Comprobación de audio: hay formatos que dicen tener sonido y no lo tienen (algunos
        // H.265 de TikTok, yt-dlp issues 16622 y 17372). Si un vídeo llega mudo, se repite la
        // descarga con un formato de imagen y sonido juntos en H.264, que siempre lo trae.
        // (No con una lista entera de YouTube: repetirla toda por un vídeo no compensa)
        if !calidad.solo_audio() && !solo_lista && archivos.iter().any(|a| !tiene_audio(&motor, a)) {
            for archivo in &archivos {
                let _ = fs::remove_file(archivo);
            }
            let _ = fs::remove_file(carpeta.join(ARCHIVO_TITULO));
            self.cambiar(&descarga.id, |d| {
                d.estado = Estado::Analizando;
                d.progreso = None;
            });
            self.avisar();
            let formato = formatos::formato_con_audio(calidad.altura_max());
            self.ejecutar(&motor, descarga, carpeta, Some(&formato))?;
            archivos = cache::archivos_finales(carpeta);
        }
        if archivos.is_empty() {
            return Err(sin_archivo());
        }

        // titulo.txt: por cada archivo, "ruta<TAB>lista<TAB>título" (after_move)
        let datos = leer_titulos(&carpeta.join(ARCHIVO_TITULO));
        let dato = |archivo: &Path| -> (Option<String>, Option<String>) {
            let nombre = archivo.file_stem().and_then(|n| n.to_str()).unwrap_or_default().to_lowercase();
            datos.get(&nombre).cloned().unwrap_or_default()
        };
        // Varios archivos (lista o carrusel): comparten grupo y se muestran juntos
        let es_lista = archivos.len() > 1;
        let nombre_lista = es_lista.then(|| {
            archivos
                .iter()
                .find_map(|a| dato(a).0)
                .or_else(|| dato(&archivos[0]).1)
                .unwrap_or_else(|| "Klippo".into())
        });
        let base = Descarga {
            grupo: es_lista.then(|| descarga.id.clone()),
            lista: nombre_lista.clone(),
            elemento: None,
            total: None,
            ..descarga.clone()
        };

        // El primer vídeo se queda en esta tarjeta y cada uno de los demás pasa a la suya
        // (su carpeta, su miniatura y su meta.json)
        for (i, archivo) in archivos.iter().enumerate().skip(1) {
            let id = format!("{}-{}", descarga.id, i + 1);
            let destino = cache::carpeta_descargas().join(&id);
            let _ = fs::remove_dir_all(&destino);
            fs::create_dir_all(&destino)?;
            let movido = destino.join(format!("video.{}", extension(archivo)));
            fs::rename(archivo, &movido)?;
            mover_miniatura_de_audio(carpeta, archivo, &destino);
            let extra = Descarga {
                id,
                creado: descarga.creado.saturating_sub(i as u64),
                ..base.clone()
            };
            let terminada = self.terminar(&motor, &extra, &destino, &movido, dato(archivo).1)?;
            let mut lista = self.lista.lock().unwrap();
            let posicion = lista.iter().position(|d| d.id == descarga.id).map_or(0, |p| p + i);
            let posicion = posicion.min(lista.len());
            lista.insert(posicion, terminada);
        }

        // El primero se llama siempre video.<ext>, como una descarga normal
        let primero = carpeta.join(format!("video.{}", extension(&archivos[0])));
        mover_miniatura_de_audio(carpeta, &archivos[0], carpeta);
        let titulo = dato(&archivos[0]).1;
        if archivos[0] != primero {
            fs::rename(&archivos[0], &primero)?;
        }
        let terminada = self.terminar(&motor, &base, carpeta, &primero, titulo)?;
        self.cambiar(&descarga.id, |d| *d = terminada.clone());
        self.avisar();
        match nombre_lista {
            Some(nombre) => avisos::lista_terminada(&self.app, &descarga.id, &nombre, archivos.len(), terminada.es_audio),
            None => avisos::descarga_terminada(&self.app, &terminada),
        }
        Ok(())
    }

    /// Miniatura, duración y meta.json de un vídeo ya descargado; devuelve su tarjeta completada
    fn terminar(
        &self,
        motor: &Motor,
        descarga: &Descarga,
        carpeta: &Path,
        archivo: &Path,
        titulo: Option<String>,
    ) -> Result<Descarga, Fallo> {
        let miniatura = carpeta.join(cache::ARCHIVO_MINIATURA);
        if !es_audio(archivo) {
            crear_miniatura(motor, archivo, &miniatura);
        }
        let meta = Meta {
            url: descarga.url.clone(),
            calidad: descarga.calidad,
            creado: descarga.creado,
            titulo,
            duracion: duracion(motor, archivo),
            completada: true,
            guardado: None,
            grupo: descarga.grupo.clone(),
            lista: descarga.lista.clone(),
        };
        cache::escribir_meta(carpeta, &meta)?;
        Ok(Descarga {
            estado: Estado::Completada,
            progreso: Some(1.0),
            eta: None,
            titulo: meta.titulo,
            duracion: meta.duracion,
            es_audio: es_audio(archivo),
            miniatura: miniatura.exists().then_some(miniatura),
            archivo: Some(archivo.to_path_buf()),
            guardado: None,
            ..descarga.clone()
        })
    }

    /// Una ejecución de yt-dlp. `formato` None = solo audio (M4A o MP3 según la calidad)
    fn ejecutar(&self, motor: &Motor, descarga: &Descarga, carpeta: &Path, formato: Option<&str>) -> Result<(), Fallo> {
        let mut comando = motor.ytdlp();
        match formato {
            Some(formato) => {
                // Si solo hay WebM/MKV, se cambia el contenedor a MP4 sin recodificar
                comando.args(["-f", formato, "--remux-video", "mp4"]);
            }
            None if descarga.calidad == Calidad::Mp3 => {
                comando.args(["-f", formatos::FORMATO_AUDIO_MP3, "-x", "--audio-format", "mp3", "--audio-quality", "0"]);
            }
            None => {
                comando.args(["-f", formatos::FORMATO_AUDIO_M4A, "-x", "--audio-format", "m4a"]);
            }
        }
        if formato.is_none() {
            comando
                // La carátula y los datos van dentro del audio: título, artista y, en una lista,
                // el álbum (el nombre de la lista) y el número de pista (fuera de una lista,
                // yt-dlp solo avisa de que no los hay). El JPG se queda junto al audio con su
                // mismo nombre (video-2.jpg) y sirve de miniatura en Klippo.
                .args(["--write-thumbnail", "--convert-thumbnails", "jpg", "--embed-thumbnail", "--embed-metadata"])
                .args(["--parse-metadata", "playlist_title:%(album)s"])
                .args(["--parse-metadata", "playlist_index:%(track_number)s"]);
        }
        let carrusel = sesiones::Plataforma::de_url(&descarga.url).is_some_and(|p| p.tiene_carruseles());
        let lista = enlaces::es_solo_lista(&descarga.url);
        if carrusel || lista {
            // Un carrusel de Instagram o X (yt-dlp no descarga las fotos) o un enlace que es
            // solo de lista: se bajan todos sus vídeos. Si alguno es privado o se ha borrado,
            // se siguen bajando los demás.
            let maximo = if lista { MAX_LISTA } else { MAX_CARRUSEL };
            comando.args(["--yes-playlist", "--ignore-errors", "--playlist-items", &format!("1:{maximo}")]);
        } else {
            // Solo el vídeo del enlace, aunque se haya abierto dentro de una lista
            comando.arg("--no-playlist");
        }
        // yt-dlp las lee y, al terminar, guarda en el mismo archivo las que se renueven
        if let Some(cookies) = sesiones::cookies_para(&self.app, &descarga.url) {
            comando.arg("--cookies").arg(cookies);
        }
        comando
            .args(["--newline", "--progress-template"])
            .arg("download:klippo|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.eta)s")
            .arg("-o")
            // video.mp4 si es uno solo; video-1.mp4, video-2.mp4… si es un carrusel
            .arg(carpeta.join("video%(playlist_index&-{}|)s.%(ext)s"))
            // Por cada archivo, al terminar: su ruta, el nombre de la lista y su título
            // (after_move no activa el modo simulación)
            .args(["--print-to-file", "after_move:%(filepath)s\t%(playlist_title|)s\t%(title)s"])
            .arg(carpeta.join(ARCHIVO_TITULO))
            .arg("--")
            .arg(&descarga.url)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut hijo = comando.spawn()?;
        self.procesos.lock().unwrap().insert(descarga.id.clone(), hijo.id());
        if self.canceladas.lock().unwrap().contains(&descarga.id) {
            matar(hijo.id());
        }

        let mut salida_error = hijo.stderr.take().expect("stderr");
        let errores = thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = salida_error.read_to_end(&mut bytes);
            String::from_utf8_lossy(&bytes).into_owned()
        });

        let mut progreso = Progreso::default();
        let mut lector = BufReader::new(hijo.stdout.take().expect("stdout"));
        let mut linea = Vec::new();
        while lector.read_until(b'\n', &mut linea).unwrap_or(0) > 0 {
            let texto = String::from_utf8_lossy(&linea).trim().to_string();
            linea.clear();
            if let Some(avance) = progreso.leer(&texto) {
                self.cambiar(&descarga.id, |d| {
                    d.estado = Estado::Descargando;
                    d.progreso = avance.fraccion;
                    d.eta = avance.eta;
                    d.elemento = avance.elemento.map(|(x, _)| x);
                    d.total = avance.elemento.map(|(_, n)| n);
                });
                self.avisar_progreso();
            } else if es_postproceso(&texto) && progreso.elemento.is_none() {
                // (En una lista la barra es la del total: no se vacía entre canción y canción)
                self.cambiar(&descarga.id, |d| {
                    d.estado = Estado::Procesando;
                    d.progreso = None;
                    d.eta = None;
                });
                self.avisar();
            }
        }

        let estado = hijo.wait()?;
        let errores = errores.join().unwrap_or_default();
        if self.canceladas.lock().unwrap().contains(&descarga.id) {
            return Err(Fallo::Cancelada);
        }
        // En una lista, si fallan algunos elementos se dan por buenos los demás; en solo audio,
        // si el audio está (p. ej. solo ha fallado la carátula), también
        let tolerable = carrusel || lista || descarga.calidad.solo_audio();
        if !estado.success() && tolerable && !cache::archivos_finales(carpeta).is_empty() {
            return Ok(());
        }
        if !estado.success() {
            let mensaje = motor::linea_de_error(&errores)
                .or_else(|| errores.lines().rev().map(str::trim).find(|l| !l.is_empty()).map(String::from))
                .unwrap_or_else(|| format!("yt-dlp terminó con el código {}", estado.code().unwrap_or(-1)));
            return Err(Fallo::Error(mensaje));
        }
        Ok(())
    }

    // ---- Lista compartida con la interfaz ----

    fn cambiar<R>(&self, id: &str, cambio: impl FnOnce(&mut Descarga) -> R) -> Option<R> {
        self.lista.lock().unwrap().iter_mut().find(|d| d.id == id).map(cambio)
    }

    fn avisar(&self) {
        *self.ultimo_aviso.lock().unwrap() = Instant::now();
        let _ = self.app.emit("descargas", self.listar());
    }

    fn avisar_progreso(&self) {
        if self.ultimo_aviso.lock().unwrap().elapsed() >= INTERVALO_PROGRESO {
            self.avisar();
        }
    }
}

/// Progreso de una ejecución de yt-dlp. Cuando la imagen y el sonido van aparte, yt-dlp
/// descarga dos archivos seguidos y cada uno va de 0 a 100 %: se juntan en una sola barra,
/// dando al vídeo casi todo el recorrido porque es casi todo el peso. En una lista, la barra
/// es la del total: ((elemento − 1) + avance del archivo) / elementos.
#[derive(Default)]
struct Progreso {
    partes: usize,
    parte: usize,
    /// "[download] Downloading item 4 of 10" → (4, 10)
    elemento: Option<(u32, u32)>,
}

#[derive(Debug, PartialEq)]
struct Avance {
    fraccion: Option<f64>,
    eta: Option<u64>,
    elemento: Option<(u32, u32)>,
}

impl Progreso {
    /// Devuelve el avance si la línea es de progreso
    fn leer(&mut self, linea: &str) -> Option<Avance> {
        if let Some(resto) = linea.strip_prefix("[download] Downloading item ") {
            let (x, n) = resto.split_once(" of ")?;
            self.elemento = x.trim().parse().ok().zip(n.trim().parse().ok());
            // Cada elemento vuelve a empezar sus partes (imagen + sonido)
            self.partes = 0;
            self.parte = 0;
            return None;
        }
        // "[info] abc: Downloading 1 format(s): 137+140"
        if linea.starts_with("[info]") {
            if let Some((_, formatos)) = linea.split_once("format(s): ") {
                self.partes = formatos.split('+').count();
            }
            return None;
        }
        if linea.starts_with("[download] Destination:") {
            self.parte += 1;
            return None;
        }
        let campos: Vec<&str> = linea.strip_prefix("klippo|")?.split('|').collect();
        let numero = |i: usize| campos.get(i).and_then(|c| c.parse::<f64>().ok());
        let total = numero(1).or(numero(2)).filter(|t| *t > 0.0);
        let fraccion = numero(0).zip(total).map(|(bajado, total)| (bajado / total).clamp(0.0, 1.0));
        let eta = numero(3).map(|e| e as u64);
        let fraccion = fraccion.map(|f| match (self.partes, self.parte) {
            (2, 2) => 0.85 + f * 0.15,
            (2, _) => f * 0.85,
            _ => f,
        });
        let fraccion = match self.elemento {
            Some((x, n)) if n > 0 => fraccion.map(|f| ((x.saturating_sub(1)) as f64 + f) / n as f64),
            _ => fraccion,
        };
        // En una lista, el tiempo que queda es solo el del archivo en curso: no se enseña
        let eta = if self.elemento.is_some() { None } else { eta };
        Some(Avance { fraccion, eta, elemento: self.elemento })
    }
}

/// titulo.txt de una descarga: por archivo, "ruta<TAB>lista<TAB>título". Devuelve, por nombre
/// del archivo sin extensión (en minúsculas), el nombre de la lista y el título.
fn leer_titulos(ruta: &Path) -> HashMap<String, (Option<String>, Option<String>)> {
    let no_vacio = |t: &str| Some(t.trim().to_string()).filter(|t| !t.is_empty() && t != "NA");
    fs::read_to_string(ruta)
        .unwrap_or_default()
        .lines()
        .filter_map(|linea| {
            let mut campos = linea.splitn(3, '\t');
            let archivo = Path::new(campos.next()?.trim());
            let nombre = archivo.file_stem()?.to_str()?.to_lowercase();
            Some((nombre, (campos.next().and_then(no_vacio), campos.next().and_then(no_vacio))))
        })
        .collect()
}

/// En solo audio, yt-dlp deja la imagen de cada elemento junto al audio y con su nombre
/// (video.jpg, video-2.jpg…): pasa a miniatura.jpg en la carpeta de ese elemento
fn mover_miniatura_de_audio(origen: &Path, archivo: &Path, destino: &Path) {
    let nombre = archivo.file_stem().and_then(|n| n.to_str()).unwrap_or("video");
    let sufijo = nombre.strip_prefix("video").unwrap_or("");
    let final_ = destino.join(cache::ARCHIVO_MINIATURA);
    for candidata in [format!("miniatura{sufijo}.jpg"), format!("{nombre}.jpg")] {
        let ruta = origen.join(candidata);
        if ruta.exists() && ruta != final_ {
            let _ = fs::rename(&ruta, &final_);
            return;
        }
    }
}

fn es_postproceso(linea: &str) -> bool {
    ["[Merger]", "[VideoRemuxer]", "[ExtractAudio]", "[Fixup", "[VideoConvertor]"]
        .iter()
        .any(|p| linea.starts_with(p))
}

/// Para el proceso y todos sus hijos (yt-dlp arranca Python, FFmpeg y Deno por debajo)
pub fn matar(pid: u32) {
    let _ = motor::sin_ventana(Path::new("taskkill"))
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// ¿Tiene el archivo alguna pista de audio? Si no se puede leer, no se reintenta a ciegas.
fn tiene_audio(motor: &Motor, archivo: &Path) -> bool {
    motor
        .ffprobe()
        .args(["-v", "error", "-select_streams", "a", "-show_entries", "stream=codec_type", "-of", "csv=p=0"])
        .arg(archivo)
        .stdin(Stdio::null())
        .output()
        .map(|s| !s.status.success() || String::from_utf8_lossy(&s.stdout).contains("audio"))
        .unwrap_or(true)
}

fn duracion(motor: &Motor, archivo: &Path) -> Option<f64> {
    let salida = motor
        .ffprobe()
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
        .arg(archivo)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&salida.stdout).trim().parse().ok()
}

/// Un fotograma al segundo 1 suele ser más representativo que el primero (a menudo negro);
/// si el vídeo dura menos, el primero
fn crear_miniatura(motor: &Motor, video: &Path, destino: &Path) {
    let escala = format!(
        "scale={LADO_MINIATURA}:{LADO_MINIATURA}:force_original_aspect_ratio=decrease"
    );
    for segundo in ["1", "0"] {
        let _ = motor
            .ffmpeg()
            .args(["-v", "error", "-y", "-ss", segundo, "-i"])
            .arg(video)
            .args(["-frames:v", "1", "-vf", &escala, "-q:v", "4"])
            .arg(destino)
            .stdin(Stdio::null())
            .status();
        if destino.metadata().is_ok_and(|m| m.len() > 0) {
            return;
        }
    }
}

fn extension(archivo: &Path) -> String {
    archivo.extension().and_then(|e| e.to_str()).unwrap_or("mp4").to_lowercase()
}

fn es_audio(archivo: &Path) -> bool {
    archivo.extension().is_some_and(|e| e.eq_ignore_ascii_case("m4a") || e.eq_ignore_ascii_case("mp3"))
}

fn no_encontrada() -> String {
    crate::traducir("No se encontró el vídeo descargado", "The downloaded video could not be found")
}

fn sin_archivo() -> Fallo {
    Fallo::Error("No se encontró el vídeo descargado".into())
}

fn ahora_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn progreso_de_un_solo_archivo() {
        let mut p = Progreso::default();
        assert_eq!(p.leer("[info] abc: Downloading 1 format(s): 18"), None);
        assert_eq!(p.leer("[download] Destination: C:\\x\\video.mp4"), None);
        assert_eq!(p.leer("klippo|50|100|NA|3"), Some(avance(Some(0.5), Some(3), None)));
        // Sin tamaño total, con el estimado
        assert_eq!(p.leer("klippo|25|NA|100.0|NA"), Some(avance(Some(0.25), None, None)));
        // Sin nada que sirva
        assert_eq!(p.leer("klippo|NA|NA|NA|NA"), Some(avance(None, None, None)));
        assert_eq!(p.leer("[youtube] abc: Downloading webpage"), None);
    }

    #[test]
    fn progreso_de_imagen_y_sonido_aparte() {
        let mut p = Progreso::default();
        p.leer("[info] abc: Downloading 1 format(s): 137+140");
        p.leer("[download] Destination: video.f137.mp4");
        assert_eq!(p.leer("klippo|100|100|NA|0"), Some(avance(Some(0.85), Some(0), None)));
        p.leer("[download] Destination: video.f140.m4a");
        assert_eq!(p.leer("klippo|0|10|NA|1"), Some(avance(Some(0.85), Some(1), None)));
        assert_eq!(p.leer("klippo|10|10|NA|0"), Some(avance(Some(1.0), Some(0), None)));
    }

    fn avance(fraccion: Option<f64>, eta: Option<u64>, elemento: Option<(u32, u32)>) -> Avance {
        Avance { fraccion, eta, elemento }
    }

    #[test]
    fn progreso_del_total_de_una_lista() {
        let mut p = Progreso::default();
        assert_eq!(p.leer("[download] Downloading item 4 of 10"), None);
        p.leer("[info] abc: Downloading 1 format(s): 140");
        p.leer("[download] Destination: video-4.webm");
        // 3 terminados + la mitad del cuarto, de 10
        assert_eq!(p.leer("klippo|50|100|NA|7"), Some(avance(Some(0.35), None, Some((4, 10)))));
        // El siguiente vuelve a empezar sus partes
        p.leer("[download] Downloading item 5 of 10");
        p.leer("[info] def: Downloading 1 format(s): 137+140");
        p.leer("[download] Destination: video-5.f137.mp4");
        assert_eq!(p.leer("klippo|100|100|NA|0"), Some(avance(Some(0.485), None, Some((5, 10)))));
    }

    #[test]
    fn titulos_por_archivo() {
        let ruta = std::env::temp_dir().join(format!("klippo-titulos-{}.txt", std::process::id()));
        fs::write(
            &ruta,
            "C:\\x\\video-1.m4a\tZelda OST\tSong of Storms\nC:\\x\\video-2.m4a\tZelda OST\tLost Woods\nC:\\x\\video.mp4\tNA\tSolo\n",
        )
        .unwrap();
        let datos = leer_titulos(&ruta);
        assert_eq!(datos["video-1"], (Some("Zelda OST".into()), Some("Song of Storms".into())));
        assert_eq!(datos["video-2"].1.as_deref(), Some("Lost Woods"));
        assert_eq!(datos["video"], (None, Some("Solo".into())));
        fs::remove_file(ruta).unwrap();
    }
}
