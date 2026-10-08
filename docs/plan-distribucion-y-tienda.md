# Plan: instalar Iris con un comando y la tienda de piezas

2026-10-08. Lo que pidió Kevin: que alguien escriba un comando en la terminal (Windows, Arch,
Ubuntu) y tenga Iris, y una tienda en su VPS donde cualquiera sube piezas y buddies (WASM con
sandbox) que se publican solo cuando él las aprueba.

## Lo que hoy lo impide

| Hoy | Por qué estorba | Arreglo |
|---|---|---|
| El binario se llama `jarvis` | Se publicaría con el nombre viejo | Renombrar a `iris` (binario, carpetas `~/.config/iris`, comandos) antes de publicar, migrando `~/.config/jarvis` |
| Solo corre en Linux con Wayland | `wl-copy`/`wl-paste`, `notify-send`, sockets UNIX (surco, ghubd), `hyprctl` (flotante), `cava` | Portapapeles con `arboard`, avisos con `notify-rust`; surco, ghubd, cava y el flotante quedan como extras de Linux (`cfg(unix)`), que en Windows simplemente no están |
| Se actualiza compilando el repo | Exige Rust, git y el SDK de Vulkan en cada máquina | Bajar el binario ya compilado de la última versión |
| Iris es una interfaz sobre `claude` | Sin Claude Code instalado y con sesión iniciada no hace nada | El instalador instala Claude Code con su instalador oficial si falta; el primer arranque pide `claude` para iniciar sesión |
| La voz pesa | Whisper large-v3-turbo q5 son ~550 MB; la voz que habla (kokoro) es Python | El modelo se baja en el primer uso de la voz, con barra de progreso; kokoro queda opcional (`--con-voz`) |
| Las piezas son Rust compilado adentro | Nada se puede cargar mientras Iris corre | Host WASM (fase 3) |

## Fases

**0. Portable.** Renombre a Iris, los arreglos de la tabla, y que compile y arranque en
Windows (Windows Terminal ya entiende Sixel desde la 1.22; si no, braille). Se prueba en una
VM de Windows.

**1. Binarios.** GitHub Actions al crear una etiqueta `v*`: Linux x86_64 compilado en Ubuntu
22.04 (glibc 2.35, así corre en Ubuntu 22.04+ y en Arch) y Windows x86_64 (MSVC, SDK de
Vulkan para whisper). Se suben a la Release con su sha256.

**2. Instalador.**
- Linux: `curl -fsSL https://<dominio>/install.sh | sh`. Detecta Arch o Ubuntu, instala lo que
  falte (ffmpeg y lo opcional), deja `iris` en `~/.local/bin`, verifica el sha256.
- Windows: `irm https://<dominio>/install.ps1 | iex`. Deja `iris.exe` en
  `%LOCALAPPDATA%\Iris` y lo añade al PATH.
- Los dos: instalan Claude Code si no está, y `iris --desinstalar` deshace todo.
- Se prueba en contenedores limpios de Arch y Ubuntu y en la VM de Windows.

**3. Host WASM en Iris.** Cada pieza o buddy es un módulo WASM que exporta lo mismo que hoy
los traits `Pieza` y `Buddy` (`step`, `evento`/`cambio`/`aviso`, `paint`, `paleta`). Iris le
pasa el contexto (estado, voz, música, mirada) y la rejilla; el módulo no tiene archivos, red
ni reloj propio, con tope de memoria y de combustible por cuadro (si se pasa, se descarga y
vuelve el buddy original). Un buddy puede usar las piezas que trae Iris (partículas, hijos de
los subagentes, ondas) y pintar sprites que traiga su paquete. Motor: se mide `wasmtime`
(rápido, binario más pesado) contra `wasmi` (intérprete, chico) con una pieza real. Prueba de
que funciona: las partículas pasadas a WASM se ven igual que las nativas.

**4. Paquete y kit.** Un `.iris` (zip): `iris.toml` (nombre, tipo, autor, versión, versión del
ABI, licencia), el `.wasm`, `sprites/` y una vista previa. Un crate `iris-pieza` con los tipos
y una plantilla, y `iris pieza probar <carpeta>` para verla en vivo mientras se escribe.

**5. La tienda en el VPS.** Una API (Rust con axum, SQLite y los paquetes en disco) detrás de
Caddy con HTTPS:
- subir sin cuenta → queda en revisión y no se puede descargar;
- el panel de Kevin para aprobar o rechazar (con la vista previa animada);
- listar, buscar y descargar lo aprobado, con su sha256;
- límites por IP para subir y tamaño máximo por paquete.
El mismo dominio sirve `install.sh`, `install.ps1` y una página para navegar la tienda.

**6. `/tienda` en Iris.** Buscar, ver la vista previa, instalar, actualizar y quitar. Lo
instalado aparece en `/buddy` y en las piezas como lo de fábrica.

## Lo que falta para empezar

- El VPS: IP o dominio, usuario SSH, qué tiene instalado, y el dominio de la tienda.
- Si se aceptan personajes con dueño (Kirby es de Nintendo) o solo originales.
- Una VM de Windows para probar (o una máquina con Windows a la que pueda entrar).
