// Del navegador a Klippo, sin extensiones:
// - Portapapeles: al copiar un enlace de una plataforma conocida, si Klippo está delante se
//   pone en el campo; si está detrás o en la bandeja, un aviso de Windows ofrece descargarlo
//   (nunca descarga solo).
// - Marcador "Enviar a Klippo": abre klippo://descargar?url=<enlace de la pestaña>.
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use clipboard_master::{CallbackResult, ClipboardHandler, Master};
use serde::de::DeserializeOwned;
use tauri::{AppHandle, Emitter, Manager, Url};

use crate::descargas::{Calidad, Descargas};
use crate::{avisos, enlaces, mostrar_ventana};

/// Enlace que espera a que la interfaz pregunte la calidad (o a que termine de arrancar)
#[derive(Default)]
pub struct Entrada {
    pendiente: Mutex<Option<String>>,
}

impl Entrada {
    pub fn tomar_pendiente(&self) -> Option<String> {
        self.pendiente.lock().unwrap().take()
    }
}

/// ¿Está la ventana de Klippo delante?
pub fn ventana_delante(app: &AppHandle) -> bool {
    app.get_webview_window("main")
        .is_some_and(|v| v.is_visible().unwrap_or(false) && v.is_focused().unwrap_or(false))
}

/// Ajuste guardado por la interfaz (tauri-plugin-store escribe ajustes.json en los datos de
/// la app). Aquí solo se lee, directamente del archivo.
pub fn ajuste<T: DeserializeOwned>(app: &AppHandle, clave: &str, por_defecto: T) -> T {
    let ruta: Option<PathBuf> = app.path().app_data_dir().ok().map(|d| d.join("ajustes.json"));
    ruta.and_then(|r| std::fs::read(r).ok())
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|json| json.get(clave).cloned())
        .and_then(|valor| serde_json::from_value(valor).ok())
        .unwrap_or(por_defecto)
}

/// Descargar un enlace que llega de fuera (marcador o botón "Descargar" de un aviso).
/// Si hay que preguntar la calidad, o el enlace no es de una plataforma conocida, Klippo se
/// pone delante con el enlace en el campo; si no, descarga sin quitarte de lo que hacías.
pub fn pedir_descarga(app: &AppHandle, url: String, avisar_al_empezar: bool) {
    let preguntar = ajuste(app, "preguntarCalidad", true);
    if preguntar || !enlaces::es_soportada(&url) {
        *app.state::<Entrada>().pendiente.lock().unwrap() = Some(url);
        mostrar_ventana(app);
        let _ = app.emit("enlace-pendiente", ());
        return;
    }
    let calidad: Calidad = ajuste(app, "calidad", Calidad::Mejor);
    match app.state::<Arc<Descargas>>().descargar(url.clone(), calidad) {
        Ok(_) if avisar_al_empezar && !ventana_delante(app) => avisos::descargando(app, &url),
        Ok(_) => {}
        Err(_) => mostrar_ventana(app),
    }
}

/// klippo://descargar?url=…
pub fn abrir_enlace_klippo(app: &AppHandle, enlace: &Url) {
    if enlace.scheme() != "klippo" {
        return;
    }
    let url = enlace.query_pairs().find(|(clave, _)| clave == "url").map(|(_, valor)| valor.into_owned());
    match url.and_then(|u| enlaces::extraer_url(&u)) {
        Some(url) => pedir_descarga(app, url, true),
        None => mostrar_ventana(app),
    }
}

// ---- Portapapeles ----

/// Texto del portapapeles (otra app puede tenerlo abierto un instante: se reintenta)
pub fn leer_portapapeles() -> Option<String> {
    for _ in 0..5 {
        if let Ok(texto) = clipboard_win::get_clipboard_string() {
            return Some(texto);
        }
        thread::sleep(Duration::from_millis(40));
    }
    None
}

pub fn vigilar_portapapeles(app: AppHandle) {
    thread::spawn(move || {
        if let Ok(mut master) = Master::new(Vigia { app, ultimo: None }) {
            let _ = master.run();
        }
    });
}

struct Vigia {
    app: AppHandle,
    /// El mismo enlace no se ofrece dos veces seguidas
    ultimo: Option<String>,
}

impl ClipboardHandler for Vigia {
    fn on_clipboard_change(&mut self) -> CallbackResult {
        let url = leer_portapapeles()
            .and_then(|texto| enlaces::extraer_url(&texto))
            .filter(|url| enlaces::es_soportada(url));
        if let Some(url) = url {
            if self.ultimo.as_deref() != Some(url.as_str()) {
                self.ultimo = Some(url.clone());
                enlace_copiado(&self.app, url);
            }
        }
        CallbackResult::Next
    }

    fn on_clipboard_error(&mut self, _error: std::io::Error) -> CallbackResult {
        CallbackResult::Next
    }
}

fn enlace_copiado(app: &AppHandle, url: String) {
    if ventana_delante(app) {
        let _ = app.emit("enlace-copiado", url);
        return;
    }
    let ya_descargado = app.state::<Arc<Descargas>>().listar().iter().any(|d| d.url == url);
    if !ya_descargado && ajuste(app, "vigilarPortapapeles", true) {
        avisos::ofrecer_enlace(app, url);
    }
}
