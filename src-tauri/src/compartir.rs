// Guardar y compartir un vídeo descargado:
// - Guardar: copia en Descargas\Klippo\klippo_<fecha>.mp4 (o .m4a). Es lo único que sale de
//   la carpeta temporal.
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

/// Pone el archivo en el portapapeles, como "Copiar" en el Explorador
pub fn copiar(archivo: &Path) -> Result<(), String> {
    let ruta = archivo.to_string_lossy().into_owned();
    let _abierto = clipboard_win::Clipboard::new_attempts(10).map_err(texto)?;
    // Se vacía antes: si quedara el texto de un enlace, algunas apps pegarían el enlace
    clipboard_win::raw::empty().map_err(texto)?;
    clipboard_win::raw::set_file_list(&[ruta.as_str()]).map_err(texto)
}

/// ¿Está instalada la app de WhatsApp (registra el protocolo whatsapp:)?
fn hay_whatsapp_de_escritorio() -> bool {
    windows_registry::CLASSES_ROOT.open("whatsapp").is_ok()
}

/// Copia el vídeo y abre WhatsApp. Devuelve true si se abrió la app, false si WhatsApp Web.
pub fn whatsapp(app: &AppHandle, archivo: &Path) -> Result<bool, String> {
    copiar(archivo)?;
    let escritorio = hay_whatsapp_de_escritorio();
    let destino = if escritorio { "whatsapp://" } else { "https://web.whatsapp.com/" };
    app.opener().open_url(destino, None::<&str>).map_err(texto)?;
    Ok(escritorio)
}

/// Lo que se comparte la próxima vez que Windows lo pida (evento DataRequested)
struct Peticion {
    archivo: StorageFile,
    titulo: HSTRING,
}

// Todo ocurre en el hilo de la ventana: los objetos de Windows se guardan en ese hilo
thread_local! {
    static PETICION: RefCell<Option<Peticion>> = const { RefCell::new(None) };
    /// El gestor de la ventana y su manejador solo se crean una vez
    static GESTOR: RefCell<Option<DataTransferManager>> = const { RefCell::new(None) };
}

/// Abre el menú Compartir de Windows con el vídeo. Tiene que llamarse desde el hilo de la
/// ventana.
pub fn compartir(hwnd: isize, archivo: &Path, titulo: &str) -> windows::core::Result<()> {
    let archivo = StorageFile::GetFileFromPathAsync(&HSTRING::from(archivo.as_os_str()))?.join()?;
    PETICION.set(Some(Peticion { archivo, titulo: HSTRING::from(titulo) }));

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
                let elemento: IStorageItem = peticion.archivo.cast()?;
                datos.SetStorageItemsReadOnly(&IIterable::<IStorageItem>::from(vec![Some(elemento)]))?;
                Ok(())
            },
        ))?;
        GESTOR.set(Some(nuevo));
    }
    unsafe { interop.ShowShareUIForWindow(hwnd) }
}
