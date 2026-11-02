// Sesión opcional en Instagram, X y Facebook, para los vídeos que piden haber iniciado sesión.
// Se abre la web oficial en una ventana de Klippo; en cuanto aparece la cookie de sesión, sus
// cookies se copian en formato Netscape (el que entiende yt-dlp con --cookies) y la ventana se
// cierra sola. Klippo nunca ve la contraseña: se escribe en la web de la plataforma.
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Url, WebviewUrl, WebviewWindowBuilder};

use crate::motor::texto;
use crate::{enlaces, traducir};

#[derive(Clone, Copy, PartialEq, Serialize, serde::Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Plataforma {
    Instagram,
    X,
    Facebook,
}

pub const PLATAFORMAS: [Plataforma; 3] = [Plataforma::Instagram, Plataforma::X, Plataforma::Facebook];

impl Plataforma {
    pub fn nombre(self) -> &'static str {
        match self {
            Plataforma::Instagram => "Instagram",
            Plataforma::X => "X",
            Plataforma::Facebook => "Facebook",
        }
    }

    fn url_login(self) -> &'static str {
        match self {
            Plataforma::Instagram => "https://www.instagram.com/accounts/login/",
            Plataforma::X => "https://x.com/i/flow/login",
            Plataforma::Facebook => "https://www.facebook.com/login/",
        }
    }

    fn dominios(self) -> &'static [&'static str] {
        match self {
            Plataforma::Instagram => &["instagram.com"],
            Plataforma::X => &["x.com", "twitter.com"],
            // fb.watch y fb.com redirigen a facebook.com, que es donde se usan las cookies
            Plataforma::Facebook => &["facebook.com", "fb.watch", "fb.com"],
        }
    }

    /// Cookie que solo existe con la sesión iniciada
    fn cookie_sesion(self) -> &'static str {
        match self {
            Plataforma::Instagram => "sessionid",
            Plataforma::X => "auth_token",
            Plataforma::Facebook => "c_user",
        }
    }

    fn clave(self) -> &'static str {
        match self {
            Plataforma::Instagram => "instagram",
            Plataforma::X => "x",
            Plataforma::Facebook => "facebook",
        }
    }

    /// Instagram y X agrupan varios vídeos en una publicación (carrusel)
    pub fn tiene_carruseles(self) -> bool {
        matches!(self, Plataforma::Instagram | Plataforma::X)
    }

    pub fn de_url(url: &str) -> Option<Plataforma> {
        let host = enlaces::host(url)?;
        PLATAFORMAS
            .into_iter()
            .find(|p| p.dominios().iter().any(|d| host == *d || host.ends_with(&format!(".{d}"))))
    }
}

#[derive(Serialize, Clone)]
pub struct EstadoSesion {
    plataforma: Plataforma,
    nombre: &'static str,
    iniciada: bool,
}

/// Archivo de cookies de la plataforma (en los datos de la app: no se borra cada hora)
fn archivo(app: &AppHandle, plataforma: Plataforma) -> Option<PathBuf> {
    let carpeta = app.path().app_data_dir().ok()?.join("cookies");
    Some(carpeta.join(format!("{}.txt", plataforma.clave())))
}

/// Datos del navegador de la ventana de inicio de sesión, aparte de los de Klippo
fn datos_web(app: &AppHandle, plataforma: Plataforma) -> Option<PathBuf> {
    Some(app.path().app_local_data_dir().ok()?.join("sesiones").join(plataforma.clave()))
}

/// Archivo de cookies a usar para descargar `url`, si hay sesión guardada
pub fn cookies_para(app: &AppHandle, url: &str) -> Option<PathBuf> {
    archivo(app, Plataforma::de_url(url)?).filter(|a| a.exists())
}

pub fn estado(app: &AppHandle) -> Vec<EstadoSesion> {
    PLATAFORMAS
        .into_iter()
        .map(|p| EstadoSesion {
            plataforma: p,
            nombre: p.nombre(),
            iniciada: archivo(app, p).is_some_and(|a| a.exists()),
        })
        .collect()
}

fn etiqueta(plataforma: Plataforma) -> String {
    format!("sesion-{}", plataforma.clave())
}

/// Abre la web de la plataforma. Un hilo mira las cookies cada poco (en Windows leerlas desde
/// un evento de la ventana bloquea WebView2) y, al ver la de sesión, guarda y cierra.
pub fn iniciar(app: &AppHandle, plataforma: Plataforma) -> Result<(), String> {
    if let Some(ventana) = app.get_webview_window(&etiqueta(plataforma)) {
        let _ = ventana.set_focus();
        return Ok(());
    }
    let url = Url::parse(plataforma.url_login()).map_err(texto)?;
    let mut constructor = WebviewWindowBuilder::new(app, etiqueta(plataforma), WebviewUrl::External(url))
        .title(traducir(
            &format!("Iniciar sesión en {} · Klippo", plataforma.nombre()),
            &format!("Log in to {} · Klippo", plataforma.nombre()),
        ))
        .inner_size(520.0, 760.0)
        .center();
    if let Some(datos) = datos_web(app, plataforma) {
        constructor = constructor.data_directory(datos);
    }
    let ventana = constructor.build().map_err(texto)?;

    let app = app.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(2));
            // La ventana se cerró sin iniciar sesión
            let Some(ventana) = app.get_webview_window(&ventana.label().to_string()) else { return };
            let Ok(cookies) = ventana.cookies() else { continue };
            if guardar(&app, plataforma, &cookies).unwrap_or(false) {
                let _ = ventana.close();
                let _ = app.emit("sesiones", estado(&app));
                return;
            }
        }
    });
    Ok(())
}

/// Escribe el archivo de cookies; false si aún no hay sesión iniciada
fn guardar(app: &AppHandle, plataforma: Plataforma, cookies: &[tauri::webview::Cookie<'static>]) -> Result<bool, String> {
    let de_la_plataforma: Vec<_> = cookies
        .iter()
        .filter(|c| {
            let dominio = c.domain().unwrap_or_default().trim_start_matches('.');
            plataforma.dominios().iter().any(|d| dominio == *d || dominio.ends_with(&format!(".{d}")))
        })
        .collect();
    if !de_la_plataforma.iter().any(|c| c.name() == plataforma.cookie_sesion() && !c.value().is_empty()) {
        return Ok(false);
    }

    // Sin caducidad (cookies de sesión): un año; yt-dlp las refresca al usarlas
    let ahora = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let mut texto_cookies = String::from("# Netscape HTTP Cookie File\n");
    for c in de_la_plataforma {
        let dominio = c.domain().unwrap_or_default();
        let dominio = if dominio.starts_with('.') { dominio.to_string() } else { format!(".{dominio}") };
        let caduca = c.expires_datetime().map(|f| f.unix_timestamp()).unwrap_or(ahora + 365 * 24 * 3600);
        let prefijo = if c.http_only().unwrap_or(false) { "#HttpOnly_" } else { "" };
        texto_cookies.push_str(&format!(
            "{prefijo}{dominio}\tTRUE\t{}\t{}\t{caduca}\t{}\t{}\n",
            c.path().unwrap_or("/"),
            if c.secure().unwrap_or(false) { "TRUE" } else { "FALSE" },
            c.name(),
            c.value()
        ));
    }
    let ruta = archivo(app, plataforma).ok_or("Sin carpeta de datos")?;
    fs::create_dir_all(ruta.parent().unwrap()).map_err(texto)?;
    fs::write(ruta, texto_cookies).map_err(texto)?;
    Ok(true)
}

/// Borra las cookies guardadas y los datos de la web (la próxima vez pide entrar de nuevo)
pub fn cerrar(app: &AppHandle, plataforma: Plataforma) {
    if let Some(ventana) = app.get_webview_window(&etiqueta(plataforma)) {
        let _ = ventana.close();
    }
    if let Some(a) = archivo(app, plataforma) {
        let _ = fs::remove_file(a);
    }
    if let Some(datos) = datos_web(app, plataforma) {
        // Si WebView2 aún tiene algo abierto, se reintenta un poco después
        thread::spawn(move || {
            for _ in 0..10 {
                if fs::remove_dir_all(&datos).is_ok() || !datos.exists() {
                    return;
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn plataforma_por_url() {
        assert_eq!(Plataforma::de_url("https://www.instagram.com/reel/abc/"), Some(Plataforma::Instagram));
        assert_eq!(Plataforma::de_url("https://twitter.com/a/status/1"), Some(Plataforma::X));
        assert_eq!(Plataforma::de_url("https://fb.watch/xyz"), Some(Plataforma::Facebook));
        assert_eq!(Plataforma::de_url("https://www.youtube.com/watch?v=1"), None);
        assert!(Plataforma::X.tiene_carruseles() && !Plataforma::Facebook.tiene_carruseles());
    }
}
