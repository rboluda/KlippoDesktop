# Compila el instalador firmado y lo publica en GitHub Releases (rboluda/KlippoDesktop).
# Klippo instalado lo detecta al abrirse (una vez al día) o con "Buscar" en Ajustes.
#
# Versiones semánticas MAYOR.MENOR.PARCHE ("version" en package.json, copiada a Cargo.toml):
#   -Subir parche   0.1.0 -> 0.1.1   correcciones
#   -Subir menor    0.1.0 -> 0.2.0   funciones nuevas
#   -Subir mayor    0.1.0 -> 1.0.0   cambios grandes
# Sin -Subir publica la versión actual (si ya existe en GitHub, se detiene).
#
# La clave del actualizador está en ~/.tauri/klippo-desktop.key (sin contraseña) y NUNCA
# va al repositorio. Si se pierde, las versiones instaladas no aceptarán las nuevas:
# haz copia de seguridad.
#
# Uso:  .\scripts\publicar.ps1 -Subir menor -Notas "Qué cambia en esta versión"
param(
    [string]$Notas = "",
    [ValidateSet("", "parche", "menor", "mayor")]
    [string]$Subir = ""
)

$ErrorActionPreference = "Stop"
$raiz = Split-Path $PSScriptRoot -Parent
$repo = "rboluda/KlippoDesktop"
$clave = Join-Path $HOME ".tauri\klippo-desktop.key"
$env:PATH = "$(Join-Path $HOME '.cargo\bin');$env:PATH"
$paquete = Join-Path $raiz "package.json"
$cargo = Join-Path $raiz "src-tauri\Cargo.toml"
$patronPaquete = '"version": "(\d+)\.(\d+)\.(\d+)"'
$patronCargo = '(?m)^version = "\d+\.\d+\.\d+"'
$utf8 = New-Object Text.UTF8Encoding $false

Push-Location $raiz
try {
    if (-not (Test-Path $clave)) { throw "No se encuentra la clave del actualizador en $clave" }
    if (git status --porcelain) { throw "Hay cambios sin confirmar: haz commit antes de publicar" }

    $textoPaquete = [IO.File]::ReadAllText($paquete)
    $m = [regex]::Match($textoPaquete, $patronPaquete)
    if (-not $m.Success) { throw "No se encuentra la versión en package.json" }
    $mayor = [int]$m.Groups[1].Value; $menor = [int]$m.Groups[2].Value; $parche = [int]$m.Groups[3].Value

    switch ($Subir) {
        "mayor" { $mayor++; $menor = 0; $parche = 0 }
        "menor" { $menor++; $parche = 0 }
        "parche" { $parche++ }
    }
    $version = "$mayor.$menor.$parche"

    # Nunca se publica dos veces el mismo número
    gh release view "v$version" --repo $repo *> $null
    if ($LASTEXITCODE -eq 0) { throw "La versión $version ya está publicada: usa -Subir parche, menor o mayor" }

    if ($Subir) {
        $textoPaquete = ([regex]$patronPaquete).Replace($textoPaquete, "`"version`": `"$version`"", 1)
        [IO.File]::WriteAllText($paquete, $textoPaquete, $utf8)
        $textoCargo = ([regex]$patronCargo).Replace([IO.File]::ReadAllText($cargo), "version = `"$version`"", 1)
        [IO.File]::WriteAllText($cargo, $textoCargo, $utf8)
        cargo update --workspace --manifest-path $cargo --quiet
        git add $paquete $cargo (Join-Path $raiz "src-tauri\Cargo.lock")
        git commit -q -m "Versión $version"
        git push -q
        if ($LASTEXITCODE -ne 0) { throw "No se pudo subir el cambio de versión" }
    }

    # Firma del actualizador (la clave no tiene contraseña)
    $env:TAURI_SIGNING_PRIVATE_KEY = [IO.File]::ReadAllText($clave)
    npm run tauri build
    if ($LASTEXITCODE -ne 0) { throw "La compilación ha fallado" }

    $carpeta = Join-Path $raiz "src-tauri\target\release\bundle\nsis"
    $nombre = "Klippo_${version}_x64-setup.exe"
    $instalador = Join-Path $carpeta $nombre
    if (-not (Test-Path $instalador)) { throw "No se encuentra $instalador" }
    if (-not $Notas) { $Notas = "Klippo para Windows $version" }

    # latest.json: lo que consulta el actualizador de las versiones instaladas
    $latest = [ordered]@{
        version   = $version
        notes     = $Notas
        pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        platforms = [ordered]@{
            "windows-x86_64" = [ordered]@{
                signature = [IO.File]::ReadAllText("$instalador.sig").Trim()
                url       = "https://github.com/$repo/releases/download/v$version/$nombre"
            }
        }
    }
    $rutaLatest = Join-Path $carpeta "latest.json"
    [IO.File]::WriteAllText($rutaLatest, ($latest | ConvertTo-Json -Depth 5), $utf8)

    gh release create "v$version" $instalador $rutaLatest --repo $repo `
        --title "Klippo para Windows $version" --notes $Notas --latest
    if ($LASTEXITCODE -ne 0) { throw "No se pudo crear la release en GitHub" }
    Write-Output "Publicada la versión $version"
}
finally {
    Pop-Location
}
