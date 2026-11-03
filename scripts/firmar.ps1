# Firma un ejecutable con el certificado personal de firma de código (CN=rboluda). Tauri lo
# llama al compilar (bundle.windows.signCommand) para klippo.exe, el instalador y el
# desinstalador.
#
# El certificado NO está en el repositorio: %USERPROFILE%\.firma\rboluda-firma-codigo.pfx y su
# contraseña en rboluda-firma-codigo.clave. Si no están (otra persona compilando el código),
# no se firma y la compilación sigue.
#
# Uso:  .\scripts\firmar.ps1 <archivo.exe>
param([Parameter(Mandatory)][string]$Archivo)

$ErrorActionPreference = "Stop"

# El motor (yt-dlp, FFmpeg, Deno) no es nuestro: se instala tal como lo publican sus autores
if ($Archivo -match '\\binarios\\') {
    exit 0
}

$carpeta = Join-Path $env:USERPROFILE ".firma"
$pfx = Join-Path $carpeta "rboluda-firma-codigo.pfx"
$clave = Join-Path $carpeta "rboluda-firma-codigo.clave"
if (-not (Test-Path $pfx) -or -not (Test-Path $clave)) {
    Write-Warning "Sin certificado de firma en $carpeta`: $Archivo queda sin firmar"
    exit 0
}

$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
    Where-Object FullName -like "*\x64\*" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $signtool) { throw "No se encuentra signtool.exe (Windows SDK)" }

# Con sello de tiempo: la firma sigue siendo válida después de que caduque el certificado
& $signtool.FullName sign /fd sha256 /f $pfx /p ([IO.File]::ReadAllText($clave).Trim()) `
    /tr "http://timestamp.digicert.com" /td sha256 /d "Klippo" /du "https://github.com/rboluda/KlippoDesktop" `
    /q $Archivo
if ($LASTEXITCODE -ne 0) { throw "No se pudo firmar $Archivo" }
