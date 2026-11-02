// Selección de formatos de yt-dlp: la misma que en Klippo para Android
// (MotorDescarga.formato y formatoConAudio), con lo aprendido allí sobre cada plataforma.

/// Mejor calidad en MP4, prefiriendo H.264 (se comparte y edita sin problemas); si no existe,
/// cualquier MP4 y, en último caso, lo mejor disponible. Con `altura_max` se limita la
/// resolución; si nada cumple el límite, se baja lo que haya.
/// H.264 aparece como "avc1…" en casi todas las webs y como "h264" en TikTok.
///
/// TikTok es distinto: su formato de "audio" no es el sonido del vídeo sino la canción de la
/// publicación, y algunos vídeos H.265 vienen sin sonido. Por eso ahí solo se eligen formatos
/// que ya traen imagen y sonido juntos ("b"), nunca vídeo + audio aparte.
pub fn formato(altura_max: Option<u32>, tiktok: bool) -> String {
    let limite = altura_max.map(|a| format!("[height<={a}]")).unwrap_or_default();
    // Los archivos completos de Facebook ("sd"/"hd") no dicen su resolución: con "<=?" no se
    // descartan por no saberla
    let limite_completo = altura_max.map(|a| format!("[height<=?{a}]")).unwrap_or_default();
    let opciones = if tiktok {
        vec![
            format!("b[vcodec^=h264]{limite}"),
            format!("b[vcodec^=avc]{limite}"),
            format!("b{limite}"),
            "b[vcodec^=h264]".into(),
            "b".into(),
        ]
    } else {
        vec![
            // 1. H.264 con el audio aparte (YouTube, Instagram, X)
            format!("bv*[ext=mp4][vcodec^=avc]{limite}+ba[ext=m4a]"),
            format!("bv*[ext=mp4][vcodec^=h264]{limite}+ba[ext=m4a]"),
            // 2. Un MP4 completo con imagen y sonido (el "hd" de Facebook es H.264)
            format!("b[ext=mp4][vcodec^=avc]{limite_completo}"),
            format!("b[ext=mp4][vcodec^=h264]{limite_completo}"),
            format!("b[ext=mp4]{limite_completo}"),
            // 3. Otros códecs (VP9, AV1) con el audio aparte, remuxados a MP4
            format!("bv*[ext=mp4]{limite}+ba[ext=m4a]"),
            format!("bv*{limite}+ba"),
            format!("b{limite_completo}"),
            "b".into(),
        ]
    };
    unir(opciones)
}

/// Segundo intento cuando el vídeo llega sin sonido: imagen y sonido en un solo formato,
/// H.264 primero (TikTok, Instagram y X siempre lo ofrecen con audio)
pub fn formato_con_audio(altura_max: Option<u32>) -> String {
    let limite = altura_max.map(|a| format!("[height<={a}]")).unwrap_or_default();
    unir(vec![
        format!("b[vcodec^=h264]{limite}"),
        format!("b[vcodec^=avc]{limite}"),
        "b[vcodec^=h264]".into(),
        "b[vcodec^=avc]".into(),
        "b".into(),
    ])
}

/// Solo audio: M4A (AAC) tal cual si existe; si no, FFmpeg lo convierte
pub const FORMATO_AUDIO: &str = "bestaudio[ext=m4a]/bestaudio";

// Une las alternativas con "/" quitando las repetidas (sin límite salen iguales)
fn unir(opciones: Vec<String>) -> String {
    let mut vistas: Vec<String> = Vec::new();
    for opcion in opciones {
        if !vistas.contains(&opcion) {
            vistas.push(opcion);
        }
    }
    vistas.join("/")
}

/// ¿Es un enlace de TikTok? (tiktok.com o cualquier subdominio, como vm.tiktok.com)
pub fn es_tiktok(url: &str) -> bool {
    crate::enlaces::host(url).is_some_and(|h| h == "tiktok.com" || h.ends_with(".tiktok.com"))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    // Las cadenas deben ser idénticas a las de Android (MotorDescarga.kt)
    #[test]
    fn general_sin_limite() {
        assert_eq!(
            formato(None, false),
            "bv*[ext=mp4][vcodec^=avc]+ba[ext=m4a]/bv*[ext=mp4][vcodec^=h264]+ba[ext=m4a]/\
             b[ext=mp4][vcodec^=avc]/b[ext=mp4][vcodec^=h264]/b[ext=mp4]/\
             bv*[ext=mp4]+ba[ext=m4a]/bv*+ba/b"
        );
    }

    #[test]
    fn general_720() {
        assert_eq!(
            formato(Some(720), false),
            "bv*[ext=mp4][vcodec^=avc][height<=720]+ba[ext=m4a]/\
             bv*[ext=mp4][vcodec^=h264][height<=720]+ba[ext=m4a]/\
             b[ext=mp4][vcodec^=avc][height<=?720]/b[ext=mp4][vcodec^=h264][height<=?720]/\
             b[ext=mp4][height<=?720]/bv*[ext=mp4][height<=720]+ba[ext=m4a]/\
             bv*[height<=720]+ba/b[height<=?720]/b"
        );
    }

    #[test]
    fn tiktok_solo_formatos_completos() {
        assert_eq!(formato(None, true), "b[vcodec^=h264]/b[vcodec^=avc]/b");
        assert_eq!(
            formato(Some(1080), true),
            "b[vcodec^=h264][height<=1080]/b[vcodec^=avc][height<=1080]/b[height<=1080]/b[vcodec^=h264]/b"
        );
        assert!(!formato(Some(480), true).contains('+'));
    }

    #[test]
    fn con_audio() {
        assert_eq!(formato_con_audio(None), "b[vcodec^=h264]/b[vcodec^=avc]/b");
        assert_eq!(
            formato_con_audio(Some(480)),
            "b[vcodec^=h264][height<=480]/b[vcodec^=avc][height<=480]/b[vcodec^=h264]/b[vcodec^=avc]/b"
        );
    }

    #[test]
    fn reconoce_tiktok() {
        assert!(es_tiktok("https://www.tiktok.com/@a/video/1"));
        assert!(es_tiktok("https://vm.tiktok.com/ZM123/"));
        assert!(!es_tiktok("https://www.youtube.com/watch?v=tiktok.com"));
        assert!(!es_tiktok("https://notiktok.com/x"));
    }
}
