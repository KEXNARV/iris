# Instalador de Iris para Windows:
#   irm https://iris.knarvaez.com/install.ps1 | iex
# Variables: $env:IRIS_SIN_VOZ = "1" para no bajar el modelo de voz (~550 MB),
# $env:IRIS_DESINSTALAR = "1" para quitarlo.
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"   # la barra de Invoke-WebRequest lo hace 10 veces más lento

$Repo = "KEXNARV/iris"
$Base = "https://github.com/$Repo/releases/latest/download"
$Paquete = "iris-windows-x86_64.zip"
$Destino = Join-Path $env:LOCALAPPDATA "Programs\Iris"
$Datos = Join-Path $env:LOCALAPPDATA "iris"
$Modelo = "ggml-large-v3-turbo-q5_0.bin"
$ModeloUrl = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/$Modelo"
$Vad = "ggml-silero-v5.1.2.bin"
$VadUrl = "https://huggingface.co/ggml-org/whisper-vad/resolve/main/$Vad"

function Dice($t) { Write-Host "› $t" -ForegroundColor Cyan }
function Ojo($t) { Write-Host "! $t" -ForegroundColor Yellow }

function Quitar-DelPath($dir) {
    $p = [Environment]::GetEnvironmentVariable("Path", "User")
    $nuevo = ($p -split ";" | Where-Object { $_ -and $_ -ne $dir }) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $nuevo, "User")
}

if ($env:IRIS_DESINSTALAR -eq "1") {
    Remove-Item -Recurse -Force $Destino -ErrorAction SilentlyContinue
    Quitar-DelPath $Destino
    Dice "Iris quitado. Tus ajustes y modelos siguen en $env:APPDATA\iris y $Datos."
    return
}

if (-not [Environment]::Is64BitOperatingSystem) { throw "Iris necesita Windows de 64 bits." }

$Tmp = Join-Path ([IO.Path]::GetTempPath()) ("iris-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Force $Tmp | Out-Null
try {
    Dice "Bajando la última versión de Iris…"
    Invoke-WebRequest "$Base/$Paquete" -OutFile "$Tmp\$Paquete" -UseBasicParsing
    Invoke-WebRequest "$Base/$Paquete.sha256" -OutFile "$Tmp\$Paquete.sha256" -UseBasicParsing
    $esperado = ((Get-Content "$Tmp\$Paquete.sha256" -Raw).Trim() -split "\s+")[0].ToLower()
    $real = (Get-FileHash "$Tmp\$Paquete" -Algorithm SHA256).Hash.ToLower()
    if ($esperado -ne $real) { throw "El archivo bajado no coincide con su sha256; no instalo nada." }
    Expand-Archive "$Tmp\$Paquete" -DestinationPath $Tmp -Force
    New-Item -ItemType Directory -Force $Destino | Out-Null
    Copy-Item "$Tmp\iris\iris.exe" $Destino -Force
    Dice "Iris quedó en $Destino\iris.exe"
} finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}

$p = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not (($p -split ";") -contains $Destino)) {
    [Environment]::SetEnvironmentVariable("Path", ($p.TrimEnd(";") + ";" + $Destino), "User")
    $env:Path += ";$Destino"
    Dice "Añadido al PATH (las terminales nuevas ya lo ven)."
}

if ($env:IRIS_SIN_VOZ -ne "1") {
    $dir = Join-Path $Datos "models"
    New-Item -ItemType Directory -Force $dir | Out-Null
    if (-not (Test-Path "$dir\$Modelo")) {
        Dice "Bajando el modelo de voz (Whisper, ~550 MB). Se salta con `$env:IRIS_SIN_VOZ = '1'."
        try {
            Invoke-WebRequest $ModeloUrl -OutFile "$dir\$Modelo.part" -UseBasicParsing
            Move-Item "$dir\$Modelo.part" "$dir\$Modelo" -Force
        } catch { Ojo "No pude bajar el modelo de voz; Iris funciona igual, sin dictado." }
    }
    if (-not (Test-Path "$dir\$Vad")) {
        try { Invoke-WebRequest $VadUrl -OutFile "$dir\$Vad" -UseBasicParsing } catch { Ojo "No pude bajar el detector de voz (opcional)." }
    }
}

if (-not (Get-Command claude -ErrorAction SilentlyContinue)) {
    Dice "Iris va encima de Claude Code, que no está: lo instalo con su instalador oficial."
    try { Invoke-RestMethod https://claude.ai/install.ps1 | Invoke-Expression }
    catch { Ojo "No pude instalar Claude Code; míralo en https://docs.claude.com/claude-code" }
}

Write-Host ""
Dice "Listo. Abre una terminal nueva (mejor Windows Terminal) y escribe: iris"
Dice "Si nunca usaste Claude Code, corre primero: claude   (para iniciar sesión)"
