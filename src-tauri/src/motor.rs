// Motor de descarga: yt-dlp + Deno (para los retos de JavaScript de YouTube) + FFmpeg.
// Llegan con el instalador en <instalación>\binarios. yt-dlp se copia a los datos de la app
// (%LOCALAPPDATA%\es.rboluda.klippo.desktop\motor) para poder actualizarlo sin tocar la
// instalación; si una versión nueva de Klippo trae un yt-dlp más reciente, se vuelve a copiar.
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use tauri::{AppHandle, Manager};

const ARCHIVO_VERSION: &str = "version-yt-dlp.txt";

pub struct Motor {
    pub ytdlp: PathBuf,
    pub deno: PathBuf,
    /// Carpeta con ffmpeg.exe y ffprobe.exe
    pub ffmpeg: PathBuf,
    datos: PathBuf,
    versiones: Versiones,
}

#[derive(Clone, Default, Serialize, serde::Deserialize)]
pub struct Versiones {
    #[serde(rename = "yt-dlp")]
    pub ytdlp: String,
    pub deno: String,
    pub ffmpeg: String,
}

#[derive(Serialize)]
#[serde(tag = "resultado", rename_all = "camelCase")]
pub enum ResultadoActualizacion {
    Actualizado { version: String },
    AlDia,
}

impl Motor {
    pub fn preparar(app: &AppHandle) -> Result<Motor, String> {
        let binarios = app.path().resource_dir().map_err(texto)?.join("binarios");
        let datos = app.path().app_local_data_dir().map_err(texto)?.join("motor");
        fs::create_dir_all(&datos).map_err(texto)?;

        let versiones: Versiones = fs::read_to_string(binarios.join("versiones.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .ok_or("Falta el motor de descarga: reinstala Klippo")?;

        let ytdlp = datos.join("yt-dlp.exe");
        let copiada = fs::read_to_string(datos.join(ARCHIVO_VERSION)).unwrap_or_default();
        if !ytdlp.exists() || es_mas_nueva(&versiones.ytdlp, copiada.trim()) {
            fs::copy(binarios.join("yt-dlp.exe"), &ytdlp).map_err(texto)?;
            fs::write(datos.join(ARCHIVO_VERSION), &versiones.ytdlp).map_err(texto)?;
        }

        Ok(Motor {
            ytdlp,
            deno: binarios.join("deno.exe"),
            ffmpeg: binarios.join("ffmpeg"),
            datos,
            versiones,
        })
    }

    /// Versiones en uso: la de yt-dlp es la de la copia (puede estar actualizada)
    pub fn versiones(&self) -> Versiones {
        let ytdlp = fs::read_to_string(self.datos.join(ARCHIVO_VERSION))
            .map(|v| v.trim().to_string())
            .unwrap_or_else(|_| self.versiones.ytdlp.clone());
        Versiones { ytdlp, ..self.versiones.clone() }
    }

    /// yt-dlp con las opciones comunes a todas las ejecuciones
    pub fn ytdlp(&self) -> Command {
        let mut comando = sin_ventana(&self.ytdlp);
        comando
            .arg("--js-runtimes")
            .arg(format!("deno:{}", self.deno.display()))
            .arg("--ffmpeg-location")
            .arg(&self.ffmpeg)
            .args(["--encoding", "utf-8", "--color", "never", "--no-mtime"])
            // Python escribe en UTF-8 aunque la consola de Windows no lo sea
            .env("PYTHONUTF8", "1")
            .env("PYTHONIOENCODING", "utf-8");
        comando
    }

    pub fn ffmpeg(&self) -> Command {
        sin_ventana(&self.ffmpeg.join("ffmpeg.exe"))
    }

    pub fn ffprobe(&self) -> Command {
        sin_ventana(&self.ffmpeg.join("ffprobe.exe"))
    }

    /// Descarga la última versión estable de yt-dlp desde GitHub (yt-dlp -U)
    pub fn actualizar(&self) -> Result<ResultadoActualizacion, String> {
        let antes = self.versiones().ytdlp;
        let salida = sin_ventana(&self.ytdlp)
            .args(["--update-to", "stable", "--color", "never"])
            .stdin(Stdio::null())
            .output()
            .map_err(texto)?;
        let texto_salida = format!(
            "{}{}",
            String::from_utf8_lossy(&salida.stdout),
            String::from_utf8_lossy(&salida.stderr)
        );
        if !salida.status.success() {
            return Err(linea_de_error(&texto_salida).unwrap_or_else(|| texto_salida.trim().to_string()));
        }
        let version = sin_ventana(&self.ytdlp)
            .arg("--version")
            .stdin(Stdio::null())
            .output()
            .map_err(texto)?;
        let despues = String::from_utf8_lossy(&version.stdout).trim().to_string();
        if despues.is_empty() {
            return Err(texto_salida.trim().to_string());
        }
        fs::write(self.datos.join(ARCHIVO_VERSION), &despues).map_err(texto)?;
        Ok(if despues != antes {
            ResultadoActualizacion::Actualizado { version: despues }
        } else {
            ResultadoActualizacion::AlDia
        })
    }
}

/// Los procesos del motor se lanzan sin abrir una consola negra
pub fn sin_ventana(programa: &Path) -> Command {
    let mut comando = Command::new(programa);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        comando.creation_flags(CREATE_NO_WINDOW);
    }
    comando
}

/// Compara versiones de yt-dlp (AAAA.MM.DD, a veces con un cuarto número de revisión)
fn es_mas_nueva(candidata: &str, actual: &str) -> bool {
    let partes = |v: &str| -> Vec<u64> { v.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    partes(candidata) > partes(actual)
}

/// La última línea "ERROR: …" de la salida de yt-dlp, para enseñársela al usuario
pub fn linea_de_error(salida: &str) -> Option<String> {
    salida
        .lines()
        .rev()
        .find_map(|l| l.trim().strip_prefix("ERROR:"))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

pub fn texto(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn versiones_de_yt_dlp() {
        assert!(es_mas_nueva("2026.10.26", "2026.9.30"));
        assert!(es_mas_nueva("2026.10.26.1", "2026.10.26"));
        assert!(es_mas_nueva("2026.10.26", ""));
        assert!(!es_mas_nueva("2026.10.26", "2026.10.26"));
        assert!(!es_mas_nueva("2026.10.26", "2026.11.02"));
    }

    #[test]
    fn mensaje_de_error() {
        let salida = "[youtube] abc: Downloading webpage\nERROR: [youtube] abc: Video unavailable\n";
        assert_eq!(linea_de_error(salida).as_deref(), Some("[youtube] abc: Video unavailable"));
        assert_eq!(linea_de_error("todo bien"), None);
    }
}
