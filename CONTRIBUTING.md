# Contributing — Guía técnica de desarrollo

Guía técnica para trabajar sobre el proyecto **Overlays** (aplicación de escritorio Tauri v2 que sirve overlays HTML como Browser Source de OBS y los controla en tiempo real por WebSocket). Aquí está todo lo necesario para compilar, desarrollar, testear, publicar y entender la arquitectura.

Para el funcionamiento y la configuración orientada al usuario final, ver [README.md](README.md).

---

## Requisitos

| Dependencia | Versión                     | Notas                                                                                                                                                                                                     |
| ----------- | --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Node.js     | ≥ 21                        | En CI se usa Node 22.                                                                                                                                                                                     |
| pnpm        | cualquiera reciente (CI: 9) | Manager de paquetes del frontend.                                                                                                                                                                         |
| Rust        | stable (toolchain actual)   | Backend de Tauri.                                                                                                                                                                                         |
| Linux       | —                           | Dependencias de sistema: ver [prerrequisitos de Tauri](https://v2.tauri.app/start/prerequisites/). En CI: `libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libglib2.0-dev`. |

---

## Puesta en marcha

```bash
# 1. Instalar dependencias del frontend
pnpm install

# 2. Correr la app en modo desarrollo
pnpm tauri dev
```

El primer build de Rust compila desde cero y tarda. Al terminar se abre la ventana de la app y, en paralelo, el backend arranca el servidor de overlays en `127.0.0.1:4848`.

> En desarrollo `pnpm tauri dev` levanta el frontend con Vite (puerto `1420`) y la app Tauri. El servidor de overlays es independiente del frontend y lo arranca el backend Rust en `setup` (`src-tauri/src/lib.rs`).

---

## Comandos

| Tarea                 | Comando                                       |
| --------------------- | --------------------------------------------- |
| Lint + format (front) | `pnpm check` / auto-fix con `pnpm check:fix`  |
| Typecheck (front)     | `pnpm exec vue-tsc --noEmit`                  |
| Build (front)         | `pnpm build` (Biome + typecheck + Vite)       |
| Tests (backend)       | `cd src-tauri && cargo test`                  |
| Formato (backend)     | `cd src-tauri && cargo fmt --all`             |
| Lint (backend)        | `cd src-tauri && cargo clippy -- -D warnings` |

---

## Guía de contribución

- Use [conventional commits](https://www.conventionalcommits.org/) (ej. `feat:`, `fix:`, `docs:`, `refactor:`).
- No agregue atribución de IA ni "Co-Authored-By" a los commits.
- No saltee los hooks de lefthook: los pre-commit y pre-push corren siempre.
- Documente los cambios en [CHANGELOG.md](CHANGELOG.md) bajo la sección `[Unreleased]` manteniendo el formato [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
- Si edita textos de UI, agregue las claves en **ambos** locales (`src/i18n/locales/es.json` y `en.json`).
- Si agrega un comando Tauri nuevo, devuelva errores estructurados (`CommandError`), nunca strings crudos.

### Git hooks (lefthook)

Los hooks se gestionan con [lefthook](https://lefthook.dev) (`lefthook.yml`) y se instalan automáticamente con `pnpm install`:

- **pre-commit** (en paralelo): `pnpm check` (front), `cargo fmt --check --all` + `cargo clippy -- -D warnings` (back, solo si hay archivos `.rs` stageados).
- **pre-push**: `cargo test` (back).

Si modificás `lefthook.yml`, re-sincronizá los hooks:

```bash
pnpm exec lefthook install
```

---

## Estructura del proyecto

```
├── examples/                  # Plantillas de ejemplo (overlay.json + index.html + style.css + script.js)
├── src/                       # Frontend Vue 3 (TypeScript estricto)
│   ├── components/            # Componentes SFC (páginas + paneles)
│   ├── i18n/
│   │   └── locales/           # es.json / en.json
│   ├── lib/                   # Utilidades (mapeo de errores de comando a mensajes)
│   ├── services/              # Wrappers de IPC de Tauri (uno por dominio)
│   ├── stores/                # Stores Pinia (templates, instances, presets, config)
│   └── types/                 # Tipos compartidos, incl. commandError.ts
├── src-tauri/src/
│   ├── domain/                # Modelos núcleo (template, preset, overlay, config) y CommandError
│   ├── application/           # Servicios (config, presets, catálogo) y puertos (ports.rs)
│   └── infrastructure/        # HTTP+WS (http/), comandos Tauri (tauri/commands.rs),
│                              # persistencia JSON (json_store.rs), fuente de plantillas FS (fs_template_source.rs)
└── src-tauri/tauri.conf.json  # Configuración de Tauri
```

Convenciones del frontend:

- Los componentes **nunca llaman `invoke` directamente**; pasan por un service (`src/services/`) que wrappea la API de Tauri.
- El estado vive en stores Pinia (`src/stores/`): `templates`, `instances`, `presets`, `config`.
- La UI nunca muestra strings de error crudos: mapea los códigos de `CommandError` a mensajes i18n.

---

## Arquitectura

```
┌──────────────────────────────────────────────────┐
│  Tauri Window (Vue 3)                            │
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
└─────────────────────┬────────────────────────────┘
                      │ Tauri IPC (invoke)
                      ▼
┌─────────────────────────────────────────────────┐
│  Axum Server (Rust) — 127.0.0.1:4848-4851       │
│  ┌───────────┐ ┌──────────┐ ┌────────────────┐  │
│  │ Static    │ │ WebSocket│ │ /api/templates │  │
│  │ /overlay/ │ │ /ws      │ │                │  │
│  └─────┬─────┘ └─────┬────┘ └────────────────┘  │
└────────┼─────────────┼──────────────────────────┘
         │             │
         ▼             ▼
┌─────────────────────────────────────────────────┐
│  OBS Browser Sources                            │
│  ┌──────────────┐  ┌──────────────┐             │
│  │ overlay.html │  │ overlay.html │  ...        │
│  │ ws → show/   │  │ ws → show/   │             │
│  │   update/hide│  │   update/hide│             │
│  └──────────────┘  └──────────────┘             │
└─────────────────────────────────────────────────┘
```

### Protocolo WebSocket

- Endpoint `/ws`, broadcasting con canal `tokio::sync::broadcast` de **capacidad 128** (`BroadcastOverlayBus`).
- Al conectar, el cliente recibe un **snapshot** del estado actual (último payload por `instance_id`), de modo que un overlay recién abierto queda sincronizado.
- Si un cliente se atrasa (error `Lagged`), se le reenvía el snapshot en lugar de fallar.
- Cada overlay filtra los mensajes por su `TEMPLATE_ID` y su `instance_id`; el servidor hace broadcast a todos.

Payload:

```json
{
  "instance_id": "uuid-de-la-instancia",
  "template": "lower-third-basico",
  "action": "show",
  "fields": { "titulo": "Federico" }
}
```

`action` serializa en minúsculas: `show` | `update` | `hide`.

### Vista previa en vivo

En la página de detalle, el overlay real se carga en un `<iframe>` cross-origin y se maneja con un `instance_id` dedicado (`preview-<uuid>`), distinto del de la instancia real. Así los cambios de configuración se reflejan en la vista previa sin tocar la instancia que está en OBS. Los cambios de campos se envían con debounce de 200 ms. El botón **Abrir en ventana** crea una `WebviewWindow` con la etiqueta `preview-<uuid>` (una por instancia; reutiliza y enfoca si ya existe).

### Contrato de errores (IPC)

Los comandos Tauri devuelven un error estructurado `CommandError` (con código), espejado en el frontend como `src/types/commandError.ts`. La UI los mapea a mensajes i18n (`src/lib/errors.ts`). Nuevos comandos: no devolver strings crudos; usar los códigos existentes o agregar uno nuevo.

### Descubrimiento de plantillas

- Basado en archivos, sin manifiesto central. `TemplateCatalog::manifest()` → `FsTemplateSource::discover()` re-lee el directorio **en cada llamada** (stateless; no hay caché ni evento de filesystem).
- Una plantilla válida es un subdirectorio con `overlay.json` + `index.html`. Sin `index.html` se descarta.
- **El identificador de la plantilla es el nombre de la carpeta** (un campo `id` raíz en `overlay.json` es opcional y lo sobreescribe). El `path` del manifiesto es `<carpeta>/index.html`.
- Durante el descubrimiento se validan los campos:
  - `progress` requiere `min` y `max` con `min < max`; sin eso, el campo se descarta.
  - `boolean` requiere `default` `"true"`/`"false"` (o ausente); otro valor descarta el campo.
  - Los descartes se loguean con `[overlays]` en consola (carpeta, key y motivo).
- En la UI, el botón **Recargar** de la grilla llama `refreshTemplates()` para re-disparar el discovery sin reiniciar.

### Servidor HTTP

- `start_server` intenta `127.0.0.1` en los puertos `4848..=4851` y usa el primero libre.
- Rutas: `/overlay/{*path}` (archivos estáticos con protección de path traversal y tipos MIME), `/api/templates` (manifiesto), `/ws` (WebSocket).
- El directorio servido y el de descubrimiento son el mismo handle (`OverlaysDirHandle`), compartido entre catálogo y servidor; `set_overlays_dir` lo reemplaza en caliente.

---

## Configuración y persistencia

- **`config.json`**: contiene `overlays_dir` (opcional) y `language` (`"es"`/`"en"`, opcional). Se guarda con `serde_json` pretty en el `app_data_dir` de Tauri.
- **`presets.json`**: lista de presets `{name, template, fields}`. Guardar un preset con un nombre existente lo **reemplaza** (upsert por `name`). Saltar el nombre vacío está validado en dominio.
- Ambos archivos se crean automáticamente. Cualquier error de lectura/escritura JSON se degrada sin romper la app (fallback a valores por defecto).
- La validación de idioma vive en dominio (`SUPPORTED_LANGUAGES`); `overlays_dir` debe apuntar a un directorio existente (`OverlaysDirInvalid`).
- **Importante**: si `overlays_dir` no está configurado, el backend arranca con un directorio vacío y la grilla queda **sin plantillas**. No hay default a `src-tauri/overlays/`; el usuario elige la carpeta desde Ajustes (o un dev apunta la app a `examples/`).

---

## Tests

- Backend: `cd src-tauri && cargo test`. Cobertura principal:
  - `domain/`: modelos y serialización (`action` en minúsculas, validación de idioma, reject de valores desconocidos).
  - `application/`: servicios (config, presets: upsert, nombre vacío, delete).
  - `infrastructure/`: descubrimiento FS (validación de campos `progress`/`boolean`), bus (broadcast + snapshot/latest por instancia), rutas HTTP (manifiesto no hardcodea cantidad de plantillas, estáticos, WS broadcast y conteo de conexiones).
- Los tests de `examples/` usan el directorio real `examples/` del repo: el test de manifiesto valida la forma (ids, names, `fields` array) y que no esté vacío, por lo que **agregar plantillas de ejemplo no rompe `cargo test`**.
- Frontend: no hay suite de tests; el gate es Biome + `vue-tsc` + build de Vite.

---

## CI y releases

### CI (`ci.yml`)

Corre en push y PR hacia `develop` y `main`, con dos jobs en paralelo:

- **Frontend** (ubuntu): `pnpm install --frozen-lockfile` → `pnpm check` (Biome) → `vue-tsc --noEmit`.
- **Backend** (ubuntu): deps de sistema de Tauri → Rust stable (`clippy`, `rustfmt`) → `cargo fmt --check --all` → `cargo clippy -- -D warnings` → `cargo test`.

### Release (`release.yml`)

Disparada por un `push` de un tag `v*`. Usa `tauri-action` y publica un release de GitHub con los instaladores:

| Plataforma | Bundles             |
| ---------- | ------------------- |
| Linux      | `.deb`, `.AppImage` |
| Windows    | `.exe` (NSIS)       |
| macOS      | `.dmg`, `.app`      |

### Versionado

- SemVer. La versión vive en **tres lugares**: `package.json`, `src-tauri/tauri.conf.json` y `CHANGELOG.md`. Al hacer una release hay que actualizar los tres ([CHANGELOG](CHANGELOG.md) mantiene Keep a Changelog con secciones `Added`/`Changed`/`Fixed`).

---

## Notas técnicas y gotchas

- **CSP deshabilitada** (`csp: null` en `tauri.conf.json`): es necesario para las conexiones WebSocket locales (`ws://127.0.0.1`).
- **`bundle.resources` no está configurado**: los archivos de overlays **no** se incluyen en los instaladores de las releases. Los ejemplos viven en `examples/` solo para desarrollo y tests; un usuario que instale la release debe configurar su propia carpeta de overlays.
- **Fin de línea**: Biome exige LF. `.gitattributes` normaliza los archivos a LF en todas las plataformas para evitar que un checkout CRLF rompa el gate de formato (falla que ya ocurrió en el release de Windows).
- **Plugins de Tauri**: solo `tauri-plugin-dialog` está en uso. No agregar plugins sin necesidad.
- **Servidor en dev vs. producción**: el frontend corre por separado (Vite en `1420`) solo en dev; en build de producción se embute como `dist/`. El servidor Rust es el mismo en ambos casos.
- **Primera compilación**: `pnpm tauri dev` compila las crates desde cero; es lenta. Usá `pnpm build` para el gate completo (Biome + `vue-tsc` + Vite) antes de commitear.
