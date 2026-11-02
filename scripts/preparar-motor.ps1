# Descarga el motor de Klippo en src-tauri/binarios (no se sube al repositorio):
#   yt-dlp.exe              descargas (se copia a los datos de la app para poder actualizarlo)
#   deno.exe                intérprete de JavaScript que yt-dlp necesita para YouTube (EJS)
#   ffmpeg/                 ffmpeg.exe + ffprobe.exe + DLL (build GPL de gyan.dev con libx264)
#   versiones.json          versiones empaquetadas, para que la app sepa si su yt-dlp es más viejo
#
# Cada archivo tiene la versión y la huella SHA-256 fijadas aquí: para cambiar de versión se
# actualizan las tres líneas de esa pieza. Lo descargado se guarda en binarios/.descargas y
# no se vuelve a bajar mientras la huella coincida.
#
# Uso:  .\scripts\preparar-motor.ps1
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$piezas = @(
    @{
        nombre = "yt-dlp.exe"; version = "2026.10.26"
        url    = "https://github.com/yt-dlp/yt-dlp/releases/download/2026.10.26/yt-dlp.exe"
        sha256 = "f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d"
    },
    @{
        nombre = "deno.zip"; version = "2.9.8"
        url    = "https://github.com/denoland/deno/releases/download/v2.9.8/deno-x86_64-pc-windows-msvc.zip"
        sha256 = "cba84b26e0df5ae7f74fc66b86a95b866872a86292ebd84fa1e959927edd93bf"
    },
    @{
        nombre = "ffmpeg.zip"; version = "9.0.2"
        url    = "https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-full_build-shared.zip"
        sha256 = "8d31e162f1616e37aab3fa2db991b97e1b8dbeb1c8465fd81a81be16ff91b328"
    }
)

$binarios = Join-Path (Split-Path $PSScriptRoot -Parent) "src-tauri\binarios"
$cache = Join-Path $binarios ".descargas"
New-Item -ItemType Directory -Force $cache | Out-Null

foreach ($pieza in $piezas) {
    $destino = Join-Path $cache $pieza.nombre
    $valida = (Test-Path $destino) -and ((Get-FileHash $destino -Algorithm SHA256).Hash -eq $pieza.sha256)
    if (-not $valida) {
        Write-Output "Descargando $($pieza.nombre) $($pieza.version)…"
        Invoke-WebRequest $pieza.url -OutFile $destino
        $huella = (Get-FileHash $destino -Algorithm SHA256).Hash
        if ($huella -ne $pieza.sha256) {
            Remove-Item $destino
            throw "La huella de $($pieza.nombre) no coincide: $huella"
        }
    }
}

# yt-dlp tal cual
Copy-Item (Join-Path $cache "yt-dlp.exe") (Join-Path $binarios "yt-dlp.exe") -Force

# deno.exe, sacado del zip
$temporal = Join-Path $cache "extraido"
Remove-Item $temporal -Recurse -Force -ErrorAction SilentlyContinue
Expand-Archive (Join-Path $cache "deno.zip") $temporal
Copy-Item (Join-Path $temporal "deno.exe") (Join-Path $binarios "deno.exe") -Force
Remove-Item $temporal -Recurse -Force

# FFmpeg: solo ffmpeg, ffprobe y sus DLL (ffplay sobra), más la licencia
Expand-Archive (Join-Path $cache "ffmpeg.zip") $temporal
$origen = Get-ChildItem $temporal -Directory | Select-Object -First 1
$ffmpeg = Join-Path $binarios "ffmpeg"
Remove-Item $ffmpeg -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $ffmpeg | Out-Null
Get-ChildItem (Join-Path $origen.FullName "bin") | Where-Object { $_.Name -ne "ffplay.exe" } |
    Copy-Item -Destination $ffmpeg
Copy-Item (Join-Path $origen.FullName "LICENSE") (Join-Path $ffmpeg "LICENSE.txt")
Remove-Item $temporal -Recurse -Force

$versiones = [ordered]@{}
foreach ($pieza in $piezas) { $versiones[$pieza.nombre -replace "\.(exe|zip)$", ""] = $pieza.version }
[IO.File]::WriteAllText((Join-Path $binarios "versiones.json"), ($versiones | ConvertTo-Json),
    (New-Object Text.UTF8Encoding $false))

Write-Output "Motor listo en $binarios"
