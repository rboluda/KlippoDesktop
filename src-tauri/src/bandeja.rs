// Icono de Klippo en la bandeja del sistema: clic para abrir y menú con Abrir / Salir
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use crate::{mostrar_ventana, traducir};

pub fn crear(app: &AppHandle) -> tauri::Result<()> {
    let abrir = MenuItem::with_id(app, "abrir", traducir("Abrir Klippo", "Open Klippo"), true, None::<&str>)?;
    let salir = MenuItem::with_id(app, "salir", traducir("Salir", "Quit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&abrir, &salir])?;

    let mut constructor = TrayIconBuilder::with_id("principal")
        .tooltip("Klippo")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, evento| match evento.id.as_ref() {
            "abrir" => mostrar_ventana(app),
            "salir" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|bandeja, evento| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = evento
            {
                mostrar_ventana(bandeja.app_handle());
            }
        });
    if let Some(icono) = app.default_window_icon() {
        constructor = constructor.icon(icono.clone());
    }
    constructor.build(app)?;
    Ok(())
}
