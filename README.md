# Overlay App — Control de overlays para OBS

Aplicación de escritorio (Tauri + Rust + Vue 3) que sirve **overlays HTML/CSS/JS** como Browser Source para OBS y permite controlar su contenido en tiempo real desde un panel de control integrado.

- Servidor HTTP + WebSocket embebido en `127.0.0.1:4848` (con fallback automático a `4849–4851` si el puerto está ocupado).
- Plantillas listas: `lower-third-basico` (zócalo) y `titulo-centrado` (intro de segmento), con animaciones de entrada/salida.
- Multi-instancia: cada plantilla puede tener varias instancias abiertas al mismo tiempo, cada una con su propio `instance_id`.
- Presets: guardá combinaciones de plantilla + campos para reutilizarlas.

---

## Requisitos

| Dependencia | Versión | Notas |
| ----------- | ------- | ----- |
| Node.js | ≥ 21 | Para el script de prueba de overlays (`WebSocket` global) |
| pnpm | cualquier reciente | Manager de paquetes del frontend |
| Rust | stable (toolchain actual) | Para el backend de Tauri |
| Linux | — | Dependencias de sistema: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, `libgtk-3-dev` (ver [prerrequisitos de Tauri](https://v2.tauri.app/start/prerequisites/)) |

---

## Puesta en marcha

```bash
# 1. Instalar dependencias del frontend
pnpm install

# 2. Correr la app en modo desarrollo
pnpm tauri dev
```

El primer build de Rust tarda un rato. Al terminar se abre la ventana de la app y, en paralelo, queda corriendo el servidor de overlays en `http://localhost:4848`.

> En dev, `pnpm tauri dev` levanta el frontend (Vite, puerto `1420`) y la app Tauri; el servidor de overlays es independiente y lo arranca el backend Rust.

---

## Uso de la app — paso a paso

### 1. Elegir la plantilla

Desde el **OverlaysPage** (grilla de plantillas), seleccioná la que querés usar:

- **Zócalo básico** — barra con título + subtítulo abajo a la izquierda.
- **Título centrado (intro de segmento)** — texto centrado con entrada desde arriba.

Al seleccionar una plantilla se crea una nueva instancia y se abre el **OverlayDetailPage**.

### 2. Completar los campos

En el **ContentPanel** de la página de detalle, completá los campos según la plantilla:

| Campo | Ejemplo |
| ----- | ------- |
| Título | `Federico Pérez` |
| Subtítulo | `Dev Backend` |

### 3. Mostrar / Actualizar / Ocultar

Desde el **PreviewPanel** (vista 16:9):

- **Mostrar** → dispara la animación de entrada del overlay en OBS.
- **Actualizar** → cambia el texto sin re-animar (útil para contenido que cambia solo, como un cronómetro).
- **Ocultar** → dispara la animación de salida.

Todo se refleja en OBS en tiempo real vía WebSocket, sin recargar la fuente.

### 4. Guardar presets (opcional)

Desde el **ContentPanel**:

- **Guardar actual** → persiste la plantilla + campos con un nombre.
- **Aplicar** (en la lista de presets) → carga los campos guardados y dispara `show` de una.
- **Eliminar** (botón × en cada preset) → borra el preset.

Los presets se guardan en `presets.json` dentro del directorio de datos de la app y sobreviven al reinicio.

### 5. Multi-instancia

Cada plantilla puede tener varias instancias simultáneas. Las instancias aparecen en el **Sidebar** con un indicador de estado (live/idle) y un botón para cerrarlas. Cada overlay filtra los mensajes WebSocket por su propio `instance_id` (UUID), además del `TEMPLATE_ID`.

---

## Configurar OBS

1. Abrí la app de escritorio (arranca el servidor automáticamente).
2. En OBS: **Fuentes → + → Navegador**.
3. Pegá la URL que aparece en el subtítulo del **OverlayDetailPage**, por ejemplo:
   `http://localhost:4848/overlay/lower-third-basico/index.html?instance_id=<uuid>`
4. Ajustá el tamaño de la fuente según el overlay (definido en el CSS de la plantilla). Para un zócalo de 1920×1080 de canvas, un ancho/alto de `1920×300` suele funcionar.
5. Marcá **"Actualizar navegador cuando la escena se active"** si querés que se recargue al cambiar de escena.

Para usar varias plantillas a la vez (zócalo + título), agregá cada una como fuente de navegador separada.

---

## Configuración del directorio de overlays

El directorio donde se buscan las plantillas es configurable desde la **SettingsPage** de la app. Por defecto busca en `src-tauri/overlays/`. Si configurás otro directorio, se persiste en `config.json` dentro del directorio de datos de la app.

---

## Verificar que todo funciona (opcional)

Con la app corriendo, desde otra terminal:

```bash
# Estado del servidor y plantillas disponibles
curl http://127.0.0.1:4848/api/templates
```

---

## Agregar una plantilla nueva

No hace falta tocar el backend ni el panel de control:

1. Creá una carpeta en el directorio de overlays con la siguiente estructura:
   - `overlay.json` — metadata de la plantilla (id, nombre, campos).
   - `index.html` — HTML base del overlay.
   - `style.css` — estilos del overlay.
   - `script.js` — lógica del overlay.

2. En `overlay.json`:

   ```json
   {
     "id": "mi-plantilla",
     "name": "Mi Plantilla",
     "fields": [
       { "key": "titulo", "label": "Título", "type": "text", "default": "Texto de ejemplo" }
     ]
   }
   ```

3. En `script.js`:
   - `const TEMPLATE_ID = "mi-plantilla"` — id que matchea con `overlay.json`.
   - `const INSTANCE_ID = new URLSearchParams(window.location.search).get("instance_id")` — para filtrar mensajes de esta instancia.
   - WebSocket: `ws://${location.host}/ws`, con reconexión cada 2s.
   - Implementar `show(fields)`, `update(fields)` y `hide()`.
   - Fondo transparente en `body` (crítico para OBS).

4. El backend descubre automáticamente la plantilla al escanear el directorio. Reiniciá la app o volvé a abrir la grilla de overlays para verla.

---

## Arquitectura

```
┌─────────────────────────────────────────────────┐
│  Tauri Window (Vue 3)                           │
│  ┌──────────┐ ┌────────────────────────────────┐ │
│  │ Sidebar  │ │  OverlaysPage / OverlayDetail  │ │
│  │ (nav +   │ │  ┌──────────┐ ┌─────────────┐  │ │
│  │  insts)  │ │  │ Preview  │ │ Content     │  │ │
│  │          │ │  │ Panel    │ │ Panel       │  │ │
│  │          │ │  │ (16:9)   │ │ (fields +   │  │ │
│  │          │ │  │          │ │  presets)   │  │ │
│  │          │ │  └──────────┘ └─────────────┘  │ │
│  └──────────┘ └────────────────────────────────┘ │
│  ┌──────────┐                                    │
│  │ Settings │                                    │
│  └──────────┘                                    │
└─────────────────────┬───────────────────────────┘
                      │ Tauri IPC (invoke)
                      ▼
┌─────────────────────────────────────────────────┐
│  Axum Server (Rust) — 127.0.0.1:4848-4851       │
│  ┌───────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Static    │ │ WebSocket│ │ /api/templates │  │
│  │ /overlay/ │ │ /ws      │ │                │  │
│  └─────┬─────┘ └─────┬────┘ └────────────────┘  │
└────────┼──────────────┼─────────────────────────┘
         │              │
         ▼              ▼
┌─────────────────────────────────────────────────┐
│  OBS Browser Sources                            │
│  ┌──────────────┐  ┌──────────────┐             │
│  │ overlay.html │  │ overlay.html │  ...        │
│  │ ws → show/   │  │ ws → show/   │             │
│  │   update/hide│  │   update/hide│             │
│  └──────────────┘  └──────────────┘             │
└─────────────────────────────────────────────────┘
```

---

## Empaquetado (pendiente — F5)

El instalador de escritorio aún no está configurado: `bundle.resources` no está declarado en `tauri.conf.json`, así que los overlays no viajan todavía en el build. Hasta que se cierre esa fase, usá la app en modo desarrollo (`pnpm tauri dev`).

Cuando esté listo, el flujo será: `pnpm tauri build` → instalar → abrir la app → usar exactamente como en modo dev.

---

## Notas técnicas

- **Puerto**: fijo `4848`, con fallback automático a `4849–4851` si está ocupado. La app muestra la URL y el puerto real en el detalle de cada overlay.
- **Protocolo**: la UI envía comandos Tauri al backend; este publica un payload JSON por WebSocket (broadcast channel, capacidad 128 mensajes) a todos los overlays conectados. Cada plantilla filtra los mensajes por `TEMPLATE_ID` + `INSTANCE_ID`.
- **Descubrimiento**: el backend escanea el directorio de overlays buscando subdirectorios con `overlay.json` + `index.html`. No hay un manifiesto centralizado.
- **Persistencia**: presets en `presets.json` y config en `config.json`, ambos en el directorio de datos de la app.
- **Tauri plugins**: `opener`, `clipboard-manager`, `dialog`.

## Referencia

- Plan general de la app: `plan-overlay-app.md`
- Plan de implementación por fases: `plan-fases.md`
