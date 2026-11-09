// Guardar y compartir vídeos descargados (uno o los de una lista entera):
// - Guardar: copia en Descargas\Klippo\klippo_<fecha>.mp4 (o .m4a); los de una lista, en
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

/// Copia el archivo en Descargas\Klippo con un nombre que no pise nada
pub fn guardar(app: &AppHandle, archivo: &Path) -> Result<PathBuf, String> {
    let carpeta = app.path().download_dir().map_err(texto)?.join("Klippo");
    fs::create_dir_all(&carpeta).map_err(texto)?;
    let fecha = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let extension = archivo.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let mut destino = carpeta.join(format!("klippo_{fecha}.{extension}"));
    let mut n = 2;
    while destino.exists() {
        destino = carpeta.join(format!("klippo_{fecha}_{n}.{extension}"));
        n += 1;
    }
    fs::copy(archivo, &destino).map_err(texto)?;
    Ok(destino)
}

/// Copia el archivo en Descargas\Klippo\<carpeta>\<nombre>.<ext> (sin pisar nada: "nombre (2)")
pub fn guardar_como(app: &AppHandle, archivo: &Path, carpeta: &str, nombre: &str) -> Result<PathBuf, String> {
    let carpeta = app.path().download_dir().map_err(texto)?.join("Klippo").join(nombre_valido(carpeta, "Klippo"));
    fs::create_dir_all(&carpeta).map_err(texto)?;
    let nombre = nombre_valido(nombre, "klippo");
    let extension = archivo.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let mut destino = carpeta.join(format!("{nombre}.{extension}"));
    let mut n = 2;
    while destino.exists() {
        destino = carpeta.join(format!("{nombre} ({n}).{extension}"));
        n += 1;
    }
    fs::copy(archivo, &destino).map_err(texto)?;
    Ok(destino)
}

/// Nombre de archivo o carpeta válido en Windows: \ / : * ? " < > | pasan a espacios, sin
/// espacios ni puntos al final y como mucho 100 caracteres
pub fn nombre_valido(texto: &str, si_vacio: &str) -> String {
    let limpio: String = texto
        .chars()
        .map(|c| if matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { ' ' } else { c })
        .collect();
    let limpio = limpio.split_whitespace().collect::<Vec<_>>().join(" ");
    let cortado: String = limpio.chars().take(100).collect();
    let cortado = cortado.trim_end_matches(['.', ' ']).to_string();
    if cortado.is_empty() { si_vacio.to_string() } else { cortado }
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
