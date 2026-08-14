# Plan de Acción — App de Escritorio para Overlays de Stream (Tauri)

## Objetivo

Construir una aplicación de escritorio (Tauri) que:

1. Sirve overlays HTML/CSS/JS (plantillas de zócalos, títulos, etc.) para usar como **Browser Source** en OBS, corriendo en `localhost`.
2. Permite controlar el contenido de esos overlays **en tiempo real** desde un panel de control dentro de la misma app.
3. Soporta **múltiples plantillas** de overlay, seleccionables/configurables por el usuario.
4. Todo en un único codebase (Rust + frontend web), sin dependencias externas como Python/FastAPI.

---

## 1. Arquitectura general

```
┌─────────────────────────────────────────────────────────┐
│                      Tauri App (proceso único)            │
│                                                             │
│  ┌───────────────────┐        ┌──────────────────────┐   │
│  │  Webview (UI)       │       │  Rust backend          │  │
│  │  Panel de control    │◄─────►│  - tokio runtime       │  │
│  │  (React/Svelte)      │invoke │  - servidor axum        │  │
│  └───────────────────┘        │    (HTTP + WS)          │  │
│                                 │  - estado compartido      │
│                                 │    (Arc<Mutex<...>>)     │
│                                 └──────────┬───────────┘   │
└────────────────────────────────────────────┼───────────────┘
                                              │ HTTP :4848/overlay/<template>
                                              │ WS   :4848/ws
                                              ▼
                                   ┌─────────────────────┐
                                   │   OBS Browser Source   │
                                   │  (renderiza el overlay) │
                                   └─────────────────────┘
```

**Flujo de datos:**

- El panel de control (dentro de la ventana Tauri) llama a **comandos Tauri** (`invoke`) para actualizar texto/estado.
- El comando Rust actualiza un estado compartido y lo **retransmite por WebSocket** a todos los overlays conectados (OBS puede tener varias fuentes de navegador abiertas al mismo tiempo, cada una escuchando).
- El servidor Rust (axum) sirve además los archivos estáticos de cada plantilla de overlay vía HTTP, para que OBS los cargue como URL (`http://localhost:4848/overlay/lower-third-1`).

---

## 2. Stack tecnológico

| Capa                           | Tecnología                                                                        |
| ------------------------------ | --------------------------------------------------------------------------------- |
| Shell de escritorio            | Tauri v2                                                                          |
| Backend embebido               | Rust + `axum` (HTTP/WS) + `tokio`                                                 |
| Comunicación interna UI ↔ Rust | Tauri `invoke` / `emit` (eventos)                                                 |
| Comunicación Rust ↔ Overlay    | WebSocket (`axum::extract::ws`)                                                   |
| Frontend panel de control      | React + Vite + TypeScript (o Svelte si preferís algo más liviano)                 |
| Overlays (plantillas)          | HTML + CSS + JS vanilla (sin build step, para que sean fáciles de editar/agregar) |
| Persistencia local             | Archivo JSON o SQLite embebido (`rusqlite`) para presets y configuración          |

---

## 3. Estructura de carpetas propuesta

```
overlay-app/
├── src-tauri/                     # Backend Rust
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs
│   │   ├── server/
│   │   │   ├── mod.rs             # arranque del servidor axum
│   │   │   ├── routes.rs          # rutas HTTP (servir overlays)
│   │   │   ├── ws.rs              # handler de WebSocket + broadcast
│   │   │   └── state.rs           # estado compartido (AppState)
│   │   ├── commands.rs            # comandos invocables desde el frontend (Tauri commands)
│   │   ├── templates.rs           # registro/metadata de plantillas disponibles
│   │   └── storage.rs             # persistencia de presets (JSON/SQLite)
│   └── overlays/                  # PLANTILLAS de overlay (servidas estáticamente)
│       ├── lower-third-basico/
│       │   ├── index.html
│       │   ├── style.css
│       │   └── script.js
│       ├── titulo-centrado/
│       │   ├── index.html
│       │   ├── style.css
│       │   └── script.js
│       └── manifest.json          # lista de plantillas + campos configurables de cada una
│
├── src/                            # Frontend del panel de control (React/Vite)
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── TemplateSelector.tsx
│   │   ├── TextEditorPanel.tsx
│   │   ├── PresetList.tsx
│   │   └── ConnectionStatus.tsx
│   ├── hooks/
│   │   └── useOverlayControl.ts   # wrapper sobre invoke() para mandar updates
│   └── types.ts
│
├── package.json
└── README.md
```

---

## 4. Backend Rust — detalle de implementación

### 4.1 Dependencias (`Cargo.toml`)

Agregar (además de las que genera `tauri init`):

- `axum` — servidor HTTP + WebSocket
- `tokio` (features: `full` o al menos `rt-multi-thread`, `net`, `sync`)
- `tower-http` (feature `fs`) — para servir archivos estáticos de `overlays/`
- `serde`, `serde_json` — serialización de mensajes
- `uuid` — IDs de mensajes/overlays si hace falta
- `rusqlite` (opcional, si se elige SQLite para presets) o simplemente `serde_json` + `std::fs` si se usa un JSON plano

### 4.2 Estado compartido (`state.rs`)

Definir un `AppState` con:

```rust
pub struct AppState {
    pub tx: tokio::sync::broadcast::Sender<String>, // canal de broadcast a todos los overlays conectados
    pub current: Arc<Mutex<OverlayPayload>>,          // último estado enviado (para overlays que se conectan tarde)
}
```

- `broadcast::Sender<String>` permite que cada conexión WebSocket entrante se suscriba (`tx.subscribe()`) y reciba todo lo que se publique.
- `current` guarda el último payload enviado por plantilla, para que si OBS recarga la fuente de navegador (o se agrega una nueva escena), el overlay pueda pedir el estado actual al conectarse (mensaje inicial "hello, dame el estado").

### 4.3 Servidor HTTP + estáticos (`routes.rs`)

- Servir la carpeta `overlays/` como estático en la ruta `/overlay/*` usando `tower_http::services::ServeDir`.
  - Ejemplo: `http://localhost:4848/overlay/lower-third-basico/index.html`
- Endpoint `GET /api/templates` → devuelve el contenido de `manifest.json` (lista de plantillas disponibles y sus campos configurables), para que el panel de control arme la UI dinámicamente sin hardcodear nada del lado del frontend.

### 4.4 WebSocket (`ws.rs`)

- Endpoint `GET /ws` con upgrade a WebSocket.
- Al conectar un cliente (el overlay dentro de OBS):
  1. Se suscribe al `broadcast::Sender`.
  2. Inmediatamente se le envía el último `current` (para que muestre el estado correcto si se conecta después de que ya se seteó texto).
  3. Loop: reenviar cada mensaje que llegue por el canal broadcast al cliente WS.
- **Formato de mensaje** (JSON), pensado para que sirva para cualquier plantilla:

```json
{
  "template": "lower-third-basico",
  "action": "show", // "show" | "hide" | "update"
  "fields": {
    "titulo": "Federico Pérez",
    "subtitulo": "Dev Backend"
  }
}
```

- `template`: identifica a qué plantilla va dirigido el mensaje (el JS de cada overlay filtra si el mensaje es para él, comparando con su propio nombre embebido).
- `action`: `show` dispara la animación de entrada, `hide` la de salida, `update` cambia el texto sin re-animar (útil para contenido que cambia solo, como un cronómetro).
- `fields`: objeto libre con los campos de texto que la plantilla necesite (cada plantilla define sus propios campos en el `manifest.json`).

### 4.5 Comandos Tauri (`commands.rs`)

Exponer comandos invocables desde el frontend con `#[tauri::command]`:

- `list_templates()` → lee `manifest.json` y devuelve la lista de plantillas + campos.
- `send_overlay_update(template: String, action: String, fields: HashMap<String, String>)` → arma el JSON y lo publica en el `broadcast::Sender` del `AppState`. También actualiza `current`.
- `save_preset(name: String, template: String, fields: HashMap<String, String>)` → persiste un preset.
- `list_presets()` / `delete_preset(name: String)` → gestión de presets guardados.
- `get_server_status()` → devuelve el puerto activo y si el servidor está corriendo (útil para mostrar la URL a copiar en OBS).

Registrar todos los comandos en `main.rs` dentro de `tauri::generate_handler![...]`.

### 4.6 Arranque del servidor (`main.rs` / `server/mod.rs`)

- Al iniciar la app (`tauri::Builder::default().setup(|app| { ... })`), lanzar el servidor axum en un `tokio::spawn` de fondo, escuchando en `127.0.0.1:4848` (puerto configurable).
- Guardar el `AppState` en el estado gestionado de Tauri (`app.manage(state)`) para que los comandos puedan acceder al mismo `broadcast::Sender`.
- Manejar el caso de puerto ocupado (reintentar con el siguiente puerto libre y exponer el puerto real vía `get_server_status`).

---

## 5. Sistema de plantillas de overlay

### 5.1 `manifest.json` (dentro de `overlays/`)

Define qué plantillas existen y qué campos tiene cada una, para que el panel de control se arme dinámicamente:

```json
{
  "templates": [
    {
      "id": "lower-third-basico",
      "nombre": "Zócalo básico",
      "path": "lower-third-basico/index.html",
      "campos": [
        { "key": "titulo", "label": "Título", "tipo": "texto" },
        { "key": "subtitulo", "label": "Subtítulo", "tipo": "texto" }
      ]
    },
    {
      "id": "titulo-centrado",
      "nombre": "Título centrado (intro de segmento)",
      "path": "titulo-centrado/index.html",
      "campos": [
        { "key": "texto", "label": "Texto principal", "tipo": "texto" }
      ]
    }
  ]
}
```

### 5.2 Convención dentro de cada plantilla (`index.html` + `script.js`)

Cada plantilla debe:

1. Conectarse al WebSocket (`ws://localhost:4848/ws`) al cargar.
2. Filtrar mensajes por su propio `id` de plantilla (constante hardcodeada en su `script.js`, ej. `const TEMPLATE_ID = "lower-third-basico"`).
3. Implementar 3 funciones/comportamientos:
   - `show(fields)` → setea el texto en el DOM y agrega clase CSS que dispara la animación de entrada (ej. `.entrar`).
   - `hide()` → agrega clase de salida (ej. `.salir`) y remueve del DOM tras el tiempo de la transición.
   - `update(fields)` → cambia el texto sin animar (para contenido dinámico).
4. Mantener el fondo transparente (`background: transparent` en `body`) — crítico para que funcione como Browser Source en OBS.
5. Reconexión automática de WebSocket (si OBS recarga la fuente o el server se reinicia), con backoff simple (reintentar cada 2s).

**Plantilla de ejemplo mínima a crear primero:** `lower-third-basico` — un zócalo simple con título + subtítulo, animación de entrada tipo slide-in desde la izquierda con fade, y salida tipo fade-out. Sirve como referencia para clonar y crear nuevas plantillas después.

### 5.3 Agregar nuevas plantillas a futuro

El sistema está pensado para que agregar una plantilla nueva sea:

1. Crear una carpeta en `overlays/nueva-plantilla/` con `index.html`, `style.css`, `script.js` siguiendo la convención de la sección 5.2.
2. Agregar la entrada correspondiente en `manifest.json`.
3. No requiere tocar el backend Rust ni el frontend del panel — ambos leen la lista dinámicamente.

---

## 6. Frontend — Panel de control

### 6.1 Pantallas / componentes

- **`TemplateSelector`**: dropdown o lista de tarjetas con las plantillas disponibles (viene de `list_templates()`).
- **`TextEditorPanel`**: al elegir una plantilla, renderiza inputs dinámicamente según `campos` del manifest. Botones: **Mostrar**, **Actualizar**, **Ocultar**.
- **`PresetList`**: lista de presets guardados (ej. nombres de invitados frecuentes) con botón "aplicar" que carga los campos y dispara `show` de una.
- **`ConnectionStatus`**: muestra la URL a pegar en OBS (`http://localhost:4848/overlay/<id>/index.html`) con botón de copiar, y un indicador de cuántos overlays están conectados actualmente por WS (útil para saber si OBS ya está escuchando).

### 6.2 Hook `useOverlayControl.ts`

Wrapper simple sobre `invoke` de Tauri:

```ts
import { invoke } from "@tauri-apps/api/core";

export function useOverlayControl() {
  const show = (template: string, fields: Record<string, string>) =>
    invoke("send_overlay_update", { template, action: "show", fields });

  const hide = (template: string) =>
    invoke("send_overlay_update", { template, action: "hide", fields: {} });

  const update = (template: string, fields: Record<string, string>) =>
    invoke("send_overlay_update", { template, action: "update", fields });

  return { show, hide, update };
}
```

---

## 7. Persistencia de presets

- Empezar simple: un archivo JSON en el directorio de datos de la app (`tauri::api::path::app_data_dir`), ej. `presets.json`:

```json
[
  {
    "nombre": "Federico - Dev",
    "template": "lower-third-basico",
    "fields": { "titulo": "Federico Pérez", "subtitulo": "Dev Backend" }
  }
]
```

- Si más adelante se necesita algo más robusto (historial, búsqueda, muchas plantillas con muchos campos), migrar a `rusqlite`. No es necesario para el MVP.

---

## 8. Orden de implementación sugerido (para el agente de código)

1. **Bootstrap del proyecto Tauri** con frontend React + TypeScript (`npm create tauri-app@latest`).
2. **Servidor axum embebido**: levantar servidor HTTP básico en `127.0.0.1:4848` que sirva un "hello world" estático, arrancado desde `setup()` de Tauri. Verificar que corre en paralelo a la ventana.
3. **Servir la carpeta `overlays/`** como estático vía `tower_http::ServeDir`, con una plantilla de prueba (`lower-third-basico`) hardcodeada, cargable en el navegador en `http://localhost:4848/overlay/lower-third-basico/index.html`.
4. **WebSocket + broadcast**: implementar `/ws`, y un comando Tauri de prueba (`send_overlay_update`) que mande un mensaje fijo. Verificar en el navegador (con la consola JS) que el mensaje llega.
5. **Lógica de la plantilla de overlay** (`script.js` de `lower-third-basico`): conectar WS, mostrar/ocultar/actualizar con animación CSS.
6. **Manifest de plantillas + endpoint `/api/templates`** y comando `list_templates`.
7. **Panel de control en React**: `TemplateSelector` + `TextEditorPanel` conectados a `useOverlayControl`. Probar el flujo completo mostrando/ocultando el zócalo desde la UI.
8. **Persistencia de presets** (JSON simple) + `PresetList`.
9. **`ConnectionStatus`**: mostrar URL para copiar en OBS y cantidad de overlays conectados (contar subscripciones activas al canal broadcast).
10. **Pulir plantilla base y crear una segunda plantilla** (`titulo-centrado`) para validar que el sistema de manifest + convención de plantillas escala sin tocar el backend.
11. **Empaquetado**: `tauri build` para generar el instalador de escritorio (Windows/Linux/Mac según target).

---

## 9. Configuración a definir antes de empezar (decisiones abiertas)

- **Puerto del servidor**: fijo (`4848`) vs. configurable desde la UI. Recomendado: fijo con fallback automático a otro puerto si está ocupado.
- **Framework de frontend del panel**: React+Vite (más ecosistema) vs. Svelte (más liviano). Ambos funcionan igual de bien con Tauri.
- **Multi-instancia**: si en algún momento se quiere correr varios zócalos distintos _simultáneamente_ en pantalla (ej. zócalo + cronómetro), el sistema ya lo soporta al tener cada plantilla su propio `id` — solo hay que agregar cada una como fuente de navegador separada en OBS.

---

## 10. Uso final en OBS (una vez implementado)

1. Abrir la app de escritorio (arranca el servidor Rust automáticamente).
2. En OBS: `Agregar fuente` → `Navegador` → URL: `http://localhost:4848/overlay/lower-third-basico/index.html` → tamaño según el overlay (definido en el CSS de la plantilla) → tildar "Actualizar navegador cuando la escena se active" según necesidad.
3. Desde el panel de control de la app: elegir plantilla, completar campos, click en "Mostrar". El zócalo aparece animado en OBS en tiempo real.
4. Guardar como preset si se va a reutilizar (ej. nombre de invitado recurrente).
