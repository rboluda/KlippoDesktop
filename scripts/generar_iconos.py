# Genera los iconos de Windows a partir del diseño de docs/icono/klipo_icon.png.
# El original es un cuadrado a sangre; en el escritorio queda mejor con las esquinas
# redondeadas (como los iconos de Windows 11), así que se recorta con una máscara suave
# y después `tauri icon` saca todos los tamaños (.ico incluido).
#
# Uso:  python scripts/generar_iconos.py      (requiere Pillow)
import shutil
import subprocess
from pathlib import Path

from PIL import Image, ImageDraw

RAIZ = Path(__file__).resolve().parent.parent
ORIGEN = RAIZ / "docs" / "icono" / "klipo_icon.png"
REDONDEADO = RAIZ / "docs" / "icono" / "klippo_redondeado.png"
TAMANO = 1024
RADIO = 0.20  # proporción del lado
SUPERMUESTREO = 4  # la máscara se dibuja más grande para que el borde no salga dentado

icono = Image.open(ORIGEN).convert("RGBA").resize((TAMANO, TAMANO), Image.LANCZOS)

grande = TAMANO * SUPERMUESTREO
mascara = Image.new("L", (grande, grande), 0)
ImageDraw.Draw(mascara).rounded_rectangle(
    (0, 0, grande - 1, grande - 1), radius=int(grande * RADIO), fill=255
)
icono.putalpha(mascara.resize((TAMANO, TAMANO), Image.LANCZOS))
icono.save(REDONDEADO)
# El logo que usa la interfaz (cabecera, lista vacía, Acerca de)
icono.resize((256, 256), Image.LANCZOS).save(RAIZ / "static" / "klippo.png", optimize=True)

subprocess.run(
    ["npx", "tauri", "icon", str(REDONDEADO), "-o", str(RAIZ / "src-tauri" / "icons")],
    cwd=RAIZ,
    check=True,
    shell=True,
)

# Solo es para Windows: fuera los de Android, iOS, macOS y la tienda (MSIX)
CONSERVAR = {"32x32.png", "64x64.png", "128x128.png", "128x128@2x.png", "icon.png", "icon.ico"}
for ruta in (RAIZ / "src-tauri" / "icons").iterdir():
    if ruta.name not in CONSERVAR:
        shutil.rmtree(ruta) if ruta.is_dir() else ruta.unlink()

print("Iconos generados en src-tauri/icons")
