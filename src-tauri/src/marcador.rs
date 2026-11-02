// Marcador "Enviar a Klippo": una página local que se abre en el navegador del usuario con el
// botón que hay que arrastrar a la barra de marcadores. El marcador abre
// klippo://descargar?url=<pestaña actual>, que Windows entrega a Klippo.
use std::fs;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::motor::texto;
use crate::traducir;

pub const CODIGO: &str =
    "javascript:void(location.href='klippo://descargar?url='+encodeURIComponent(location.href))";

pub fn abrir_pagina(app: &AppHandle) -> Result<(), String> {
    let carpeta = app.path().app_local_data_dir().map_err(texto)?;
    fs::create_dir_all(&carpeta).map_err(texto)?;
    let ruta = carpeta.join("marcador.html");
    fs::write(&ruta, pagina()).map_err(texto)?;
    app.opener().open_path(ruta.to_string_lossy(), None::<&str>).map_err(texto)
}

fn pagina() -> String {
    let idioma = traducir("es", "en");
    let titulo = traducir("Marcador «Enviar a Klippo»", "“Send to Klippo” bookmark");
    let boton = traducir("Enviar a Klippo", "Send to Klippo");
    let pasos = traducir(
        "<li>Si no ves la barra de marcadores, muéstrala: <kbd>Ctrl</kbd>+<kbd>Mayús</kbd>+<kbd>B</kbd> en Chrome, Edge y Firefox; en Opera, desde su Configuración.</li>\
         <li>Arrastra el botón naranja hasta la barra de marcadores.</li>\
         <li>Cuando estés viendo un vídeo de YouTube, TikTok, X, Instagram o Facebook, pulsa el marcador. La primera vez el navegador pregunta si quieres abrir Klippo: marca que lo permita siempre.</li>",
        "<li>If you can't see the bookmarks bar, show it: <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>B</kbd> in Chrome, Edge and Firefox; in Opera, from its Settings.</li>\
         <li>Drag the orange button to the bookmarks bar.</li>\
         <li>When you're watching a video on YouTube, TikTok, X, Instagram or Facebook, click the bookmark. The first time, the browser asks whether to open Klippo: allow it always.</li>",
    );
    let nota = traducir(
        "El marcador solo le pasa a Klippo la dirección de la pestaña. No instala nada en el navegador.",
        "The bookmark only passes the tab's address to Klippo. Nothing is installed in the browser.",
    );
    format!(
        r#"<!doctype html>
<html lang="{idioma}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{titulo}</title>
<style>
  :root {{ color-scheme: light dark; --fondo: #fff8f5; --texto: #231914; --suave: #53433c; --tarjeta: #fceae3; }}
  @media (prefers-color-scheme: dark) {{ :root {{ --fondo: #1a110d; --texto: #f1dfd8; --suave: #d8c2b9; --tarjeta: #271d18; }} }}
  body {{ margin: 0; min-height: 100vh; display: grid; place-items: center; background: var(--fondo); color: var(--texto);
         font: 16px/1.5 "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif; }}
  main {{ max-width: 560px; margin: 32px 16px; padding: 32px; border-radius: 24px; background: var(--tarjeta); }}
  h1 {{ margin: 0 0 24px; font-size: 1.5rem; }}
  .boton {{ display: inline-block; padding: 14px 28px; border-radius: 28px; color: #fff; font-weight: 700; font-size: 1.1rem;
           text-decoration: none; background: linear-gradient(135deg, #fc9703, #fb2716); cursor: grab; }}
  ol {{ margin: 28px 0 16px; padding-left: 22px; }}
  li {{ margin-bottom: 10px; }}
  kbd {{ padding: 1px 6px; border-radius: 4px; border: 1px solid currentColor; font: inherit; font-size: .85em; }}
  p {{ color: var(--suave); font-size: .9rem; margin: 0; }}
</style>
</head>
<body>
<main>
  <h1>{titulo}</h1>
  <a class="boton" href="{CODIGO}">{boton}</a>
  <ol>{pasos}</ol>
  <p>{nota}</p>
</main>
</body>
</html>
"#
    )
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn pagina_con_el_boton_del_marcador() {
        let html = pagina();
        assert!(html.contains(&format!("href=\"{CODIGO}\"")));
        assert!(!CODIGO.contains('"'), "el código va dentro de un atributo entre comillas dobles");
        // Para mirarla en un navegador: target/marcador-prueba.html
        let _ = fs::write(concat!(env!("CARGO_MANIFEST_DIR"), "/target/marcador-prueba.html"), html);
    }
}
