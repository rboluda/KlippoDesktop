// Guardar y compartir vídeos descargados (uno o los de una lista entera):
// - Guardar: copia en Descargas\Klippo\<título>.mp4 (o .m4a, .mp3); los de una lista, en
//   Descargas\Klippo\<lista>\<título>. Es lo único que sale de la carpeta temporal.
// - Copiar: el archivo al portapapeles, para pegarlo con Ctrl+V en WhatsApp, Telegram, un
//   correo o una carpeta.
// - WhatsApp: copia y abre WhatsApp (la app si está instalada; si no, WhatsApp Web).
// - Compartir: el menú Compartir de Windows.
use std::fs;
use std::path::{Path, PathBuf};
use std::cell::RefCell;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;
use windows::ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager};
use windows::Foundation::TypedEventHandler;
use windows::Storage::{IStorageItem, StorageFile};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::IDataTransferManagerInterop;
use windows::core::{factory, Interface, HSTRING};
use windows_collections::IIterable;

use crate::motor::texto;

/// Copia el archivo en Descargas\Klippo con el título del vídeo como nombre (o
/// klippo_<fecha> si no tiene título), sin pisar nada
pub fn guardar(app: &AppHandle, archivo: &Path, titulo: Option<&str>) -> Result<PathBuf, String> {
    let carpeta = app.path().download_dir().map_err(texto)?.join("Klippo");
    let nombre = titulo
        .map(|t| nombre_valido(t, ""))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("klippo_{}", chrono::Local::now().format("%Y%m%d_%H%M%S")));
    copiar_en(archivo, &carpeta, &nombre)
}

/// Copia el archivo en Descargas\Klippo\<carpeta>\<nombre>.<ext> (los de una lista)
pub fn guardar_como(app: &AppHandle, archivo: &Path, carpeta: &str, nombre: &str) -> Result<PathBuf, String> {
    let carpeta = app.path().download_dir().map_err(texto)?.join("Klippo").join(nombre_valido(carpeta, "Klippo"));
    copiar_en(archivo, &carpeta, &nombre_valido(nombre, "klippo"))
}

/// Mueve una copia ya guardada a Descargas\Klippo\<carpeta>\<nombre>.<ext> (tras corregir sus
/// etiquetas). Si no cambia nada, la deja; si la carpeta antigua queda vacía, la borra.
pub fn mover_a(app: &AppHandle, archivo: &Path, carpeta: &str, nombre: &str) -> Result<PathBuf, String> {
    let destino_carpeta = app.path().download_dir().map_err(texto)?.join("Klippo").join(nombre_valido(carpeta, "Klippo"));
    fs::create_dir_all(&destino_carpeta).map_err(texto)?;
    let extension = archivo.extension().and_then(|e| e.to_str()).unwrap_or("m4a");
    let nombre = nombre_valido(nombre, "klippo");
    let mut destino = destino_carpeta.join(format!("{nombre}.{extension}"));
    if destino.as_os_str().eq_ignore_ascii_case(archivo.as_os_str()) {
        return Ok(archivo.to_path_buf());
    }
    let mut n = 1;
    while destino.exists() {
        destino = destino_carpeta.join(format!("{nombre} ({n}).{extension}"));
        n += 1;
    }
    if fs::rename(archivo, &destino).is_err() {
        fs::copy(archivo, &destino).map_err(texto)?;
        let _ = fs::remove_file(archivo);
    }
    if let Some(antigua) = archivo.parent() {
        // Solo se borra si está vacía (remove_dir falla si no)
        let _ = fs::remove_dir(antigua);
    }
    Ok(destino)
}

/// Copia `archivo` en `carpeta` como <nombre>.<ext>; si ya existe, «<nombre> (1)», « (2)»…
fn copiar_en(archivo: &Path, carpeta: &Path, nombre: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(carpeta).map_err(texto)?;
    let extension = archivo.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let mut destino = carpeta.join(format!("{nombre}.{extension}"));
    let mut n = 1;
    while destino.exists() {
        destino = carpeta.join(format!("{nombre} ({n}).{extension}"));
        n += 1;
    }
    fs::copy(archivo, &destino).map_err(texto)?;
    Ok(destino)
}

/// Nombres que Windows reserva para dispositivos y no admite como archivo, con o sin extensión
const RESERVADOS: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Nombre de archivo o carpeta válido en Windows (el mismo criterio que en Android):
/// \ / : * ? " < > | y los caracteres de control pasan a espacios, se juntan los espacios, sin
/// puntos ni espacios al principio ni al final, como mucho 100 caracteres y nunca un nombre
/// reservado (CON, PRN, AUX, NUL, COM1…: se les añade "_")
pub fn nombre_valido(texto: &str, si_vacio: &str) -> String {
    let limpio: String = texto
        .chars()
        .map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { ' ' } else { c })
        .collect();
    let limpio = limpio.split_whitespace().collect::<Vec<_>>().join(" ");
    let recortar = |t: &str| t.trim_matches(['.', ' ']).to_string();
    let mut nombre = recortar(&limpio);
    nombre = recortar(&nombre.chars().take(100).collect::<String>());
    if nombre.is_empty() {
        return si_vacio.to_string();
    }
    let base = nombre.split('.').next().unwrap_or_default().trim_end();
    if RESERVADOS.iter().any(|r| r.eq_ignore_ascii_case(base)) {
        nombre.push('_');
    }
    nombre
}

/// Pone los archivos en el portapapeles, como "Copiar" en el Explorador
pub fn copiar(archivos: &[PathBuf]) -> Result<(), String> {
    let rutas: Vec<String> = archivos.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    let _abierto = clipboard_win::Clipboard::new_attempts(10).map_err(texto)?;
    // Se vacía antes: si quedara el texto de un enlace, algunas apps pegarían el enlace
    clipboard_win::raw::empty().map_err(texto)?;
    clipboard_win::raw::set_file_list(&rutas).map_err(texto)
}

/// ¿Está instalada la app de WhatsApp (registra el protocolo whatsapp:)?
fn hay_whatsapp_de_escritorio() -> bool {
    windows_registry::CLASSES_ROOT.open("whatsapp").is_ok()
}

/// Copia los vídeos y abre WhatsApp. Devuelve true si se abrió la app, false si WhatsApp Web.
pub fn whatsapp(app: &AppHandle, archivos: &[PathBuf]) -> Result<bool, String> {
    copiar(archivos)?;
    let escritorio = hay_whatsapp_de_escritorio();
    let destino = if escritorio { "whatsapp://" } else { "https://web.whatsapp.com/" };
    app.opener().open_url(destino, None::<&str>).map_err(texto)?;
    Ok(escritorio)
}

/// Lo que se comparte la próxima vez que Windows lo pida (evento DataRequested)
struct Peticion {
    archivos: Vec<StorageFile>,
    titulo: HSTRING,
}

// Todo ocurre en el hilo de la ventana: los objetos de Windows se guardan en ese hilo
thread_local! {
    static PETICION: RefCell<Option<Peticion>> = const { RefCell::new(None) };
    /// El gestor de la ventana y su manejador solo se crean una vez
    static GESTOR: RefCell<Option<DataTransferManager>> = const { RefCell::new(None) };
}

/// Abre el menú Compartir de Windows con los vídeos. Tiene que llamarse desde el hilo de la
/// ventana.
pub fn compartir(hwnd: isize, archivos: &[PathBuf], titulo: &str) -> windows::core::Result<()> {
    let archivos = archivos
        .iter()
        .map(|a| StorageFile::GetFileFromPathAsync(&HSTRING::from(a.as_os_str()))?.join())
        .collect::<windows::core::Result<Vec<_>>>()?;
    PETICION.set(Some(Peticion { archivos, titulo: HSTRING::from(titulo) }));

    let interop = factory::<DataTransferManager, IDataTransferManagerInterop>()?;
    let hwnd = HWND(hwnd as *mut _);
    if GESTOR.with_borrow(Option::is_none) {
        let nuevo: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };
        nuevo.DataRequested(&TypedEventHandler::new(
            |_, args: windows::core::Ref<DataRequestedEventArgs>| {
                let (Some(args), Some(peticion)) = (args.as_ref(), PETICION.take()) else {
                    return Ok(());
                };
                let datos = args.Request()?.Data()?;
                datos.Properties()?.SetTitle(&peticion.titulo)?;
                let elementos = peticion
                    .archivos
                    .iter()
                    .map(|a| a.cast::<IStorageItem>().map(Some))
                    .collect::<windows::core::Result<Vec<_>>>()?;
                datos.SetStorageItemsReadOnly(&IIterable::<IStorageItem>::from(elementos))?;
                Ok(())
            },
        ))?;
        GESTOR.set(Some(nuevo));
    }
    unsafe { interop.ShowShareUIForWindow(hwnd) }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn nombres_validos_en_windows() {
        assert_eq!(nombre_valido("CA/CC: Negro sobre negro?", "x"), "CA CC Negro sobre negro");
        assert_eq!(nombre_valido("  ..The Memes Of 2010s.....  ", "x"), "The Memes Of 2010s");
        assert_eq!(nombre_valido("a\tb\nc   d", "x"), "a b c d");
        assert_eq!(nombre_valido("con", "x"), "con_");
        assert_eq!(nombre_valido("COM1.txt", "x"), "COM1.txt_");
        assert_eq!(nombre_valido("Concierto", "x"), "Concierto");
        assert_eq!(nombre_valido("???", "klippo"), "klippo");
        assert_eq!(nombre_valido(&"á".repeat(150), "x").chars().count(), 100);
    }

    #[test]
    fn no_pisa_lo_que_ya_existe() {
        let carpeta = std::env::temp_dir().join(format!("klippo-copiar-{}", std::process::id()));
        let origen = std::env::temp_dir().join(format!("klippo-origen-{}.mp4", std::process::id()));
        fs::write(&origen, b"x").unwrap();
        let nombres: Vec<String> = (0..3)
            .map(|_| copiar_en(&origen, &carpeta, "Vídeo").unwrap().file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(nombres, ["Vídeo.mp4", "Vídeo (1).mp4", "Vídeo (2).mp4"]);
        fs::remove_dir_all(carpeta).unwrap();
        fs::remove_file(origen).unwrap();
    }
}
