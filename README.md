# Overlays — Control de overlays para OBS

Aplicación de escritorio (Tauri + Rust + Vue 3) que sirve **overlays HTML/CSS/JS** como fuentes de navegador (Browser Source) de [OBS Studio](https://obsproject.com/) y permite controlar su contenido en tiempo real desde un panel integrado: mostrarlos, ocultarlos y actualizar sus textos, colores y valores sin recargar la fuente ni salir de la app.

## Características

- **Servidor local embebido**: HTTP + WebSocket en `127.0.0.1:4848`, con fallback automático a `4849–4851` si el puerto está ocupado. Todo el tráfico es local: la app funciona sin conexión a internet.
- **Panel de control integrado**: grilla de plantillas, vista de detalle con campos editables, vista previa 16:9 y botones para mostrar, actualizar y ocultar.
- **Vista previa en vivo**: los cambios en los campos se reflejan al instante en la vista previa, sin tocar lo que está en OBS.
- **Multi-instancia**: cada plantilla puede tener varias instancias activas a la vez, cada una con su propia URL y su propio estado.
- **Presets**: permite guardar combinaciones de plantilla + campos con un nombre y volver a aplicarlas en cualquier momento.
- **Tipos de campo**: texto, color (con canal alpha), progreso (con rango) y booleano (interruptor on/off).
- **Plantillas de ejemplo**: cinco overlays listos para usar.
- **Bilingüe**: la interfaz está en español e inglés, y el idioma se cambia desde Ajustes.
- **Sin configuración de red**: OBS y la app se comunican por `localhost`; no es necesario abrir puertos ni tocar el firewall.

---

## Cómo funciona

1. La app levanta un servidor local que **sirve los archivos HTML/CSS/JS** de cada overlay (`/overlay/...`).
2. En OBS se agrega cada overlay como una **fuente de navegador (Browser Source)** apuntando a su URL.
3. El overlay se conecta al WebSocket de la app (`/ws`) y queda escuchando mensajes.
4. Desde el panel de la app se disparan comandos `show`, `update` y `hide`, que viajan por WebSocket hasta el overlay correspondiente en OBS y ejecutan su animación de entrada, la actualización de su contenido o su animación de salida.

Cada overlay filtra los mensajes por su plantilla y por su `instance_id`, de modo que varias plantillas e instancias pueden convivir sin pisarse.

---

## Uso de la app

### 1. Elegir una plantilla

En la página **Overlays** se encuentra la grilla con las plantillas disponibles (ordenadas alfabéticamente):

| Plantilla                               | Qué hace                                                                                                                                              |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Barra de progreso**                   | Barra horizontal con etiqueta, valor porcentual y color configurables.                                                                                |
| **Layout Media 3 Abajo**                | Diseño 16:9 con un espacio superior y tres espacios inferiores para escenas de juegos o reacciones. Tamaño y color del área multimedia configurables. |
| **Marco de cámara**                     | Marco que rodea la imagen de cámara en todo el canvas; color, grosor y radio de esquinas configurables.                                               |
| **Título centrado (intro de segmento)** | Texto centrado con animación de entrada desde arriba.                                                                                                 |
| **Zócalo básico**                       | Barra con título y subtítulo abajo a la izquierda, con opciones de color.                                                                             |

Al hacer clic en una plantilla se crea una instancia nueva y se abre su página de detalle. Si la carpeta de overlays está vacía o no está configurada, la grilla lo indica.

### 2. Editar el contenido

En el panel **Contenido** se muestran los campos de la plantilla, cada uno con su tipo de control:

- **Texto**: campo de texto libre.
- **Color**: selector de color (formato `#rrggbb` o `#rrggbbaa`, con transparencia).
- **Progreso**: control deslizante + entrada numérica, limitado al rango definido por la plantilla.
- **Booleano**: interruptor on/off (por ejemplo, «Mostrar subtítulo»).

Los cambios se reflejan al instante en la **vista previa** del panel izquierdo, sin esperar.

### 3. Controlar la instancia en OBS

En el panel **Vista previa**:

- **Visible en OBS / Oculto** (interruptor): dispara la animación de entrada o de salida del overlay.
- **Actualizar**: aplica los campos actuales al overlay **sin re-animar** (ideal para contenido que cambia solo, como un cronómetro o un progreso).
- **Abrir en ventana**: abre el overlay en una ventana propia para verificarlo a escala real; si la ventana ya existe, recibe el foco.

La dirección que se muestra sobre el panel es la **URL del overlay** con su `instance_id`; es la que se pega en OBS.

### 4. Guardar presets

En el panel **Contenido**, sección Presets:

- **Guardar actual**: persiste la plantilla + los campos actuales con el nombre indicado. Si ya existe un preset con ese nombre, se reemplaza.
- **Aplicar** (ícono ▶ de cada preset): carga los campos guardados y dispara `show` de inmediato.
- **Eliminar** (ícono de basura): borra el preset. Para confirmar, hay que hacer doble clic en menos de 2 segundos.

Los presets se agrupan por plantilla y sobreviven al reinicio de la app (se guardan en `presets.json`).

### 5. Instancias múltiples

En la barra lateral, la sección **Instancias** lista todas las instancias abiertas:

- Cada una muestra un punto de estado: **verde** si está visible en OBS, **gris** si está oculta.
- Si hay varias instancias de la misma plantilla, se numeran (`Zócalo básico #1`, `#2`, ...).
- Haga clic en una instancia para abrir su detalle, o en **×** para cerrarla. Si estaba visible, primero se oculta. Al cerrar la última instancia la app vuelve a la grilla.

---

## Configurar OBS

1. Abra la aplicación **Overlays** (al iniciarse arranca el servidor local automáticamente).
2. Cree una instancia desde la grilla y copie la URL que aparece en el detalle, por ejemplo:
   `http://localhost:4848/overlay/lower-third-basico/index.html?instance=<uuid>`
3. En OBS: **Fuentes → + → Navegador**.
4. Pegue la URL y ajuste el tamaño de la fuente según el layout de la plantilla. Para un canvas de 1920×1080:

   | Plantilla                           | Tamaño de fuente recomendado         |
   | ----------------------------------- | ------------------------------------ |
   | Zócalo básico                       | 1920×300 (abajo a la izquierda)      |
   | Título centrado (intro de segmento) | 1920×1080                            |
   | Barra de progreso                   | 1920×200 (el elemento está centrado) |
   | Marco de cámara                     | 1920×1080                            |
   | Layout Media 3 Abajo                | 1920×1080                            |

5. Marque **«Actualizar navegador cuando la escena se active»** si desea que la fuente se recargue cada vez que entre a la escena.
6. El overlay debe verse sobre el fondo transparente de OBS; las plantillas de ejemplo usan fondo transparente en su CSS.

Para usar varias plantillas al mismo tiempo (zócalo + cámara, por ejemplo), agregue **una fuente de navegador por instancia**. Cada instancia tiene su propia URL con un `instance_id` distinto.

---

## Configurar la aplicación

Todo se configura desde **Ajustes** (engranaje en la barra lateral):

- **Idioma**: cambia la interfaz entre Español e Inglés al instante.
- **Carpeta de overlays**: el directorio desde donde la app lee las plantillas. Se elige con el botón **Elegir directorio**. Si no se configura ninguno, la grilla queda vacía (no es posible agregar plantillas hasta elegir carpeta).
- **Versión**: muestra la versión instalada de la app.

La configuración se persiste en `config.json` dentro del directorio de datos de la aplicación y sobrevive al reinicio.

Al cambiar la carpeta desde Ajustes, la grilla se recarga automáticamente. Si se modifican archivos directamente dentro de la carpeta ya configurada (por ejemplo, al agregar una plantilla a mano), use el botón **Recargar** de la grilla para volver a escanearla sin reiniciar la app.

---

## Agregar una plantilla nueva

No es necesario tocar el backend ni el código de la app. Solo hay que crear una **carpeta en el directorio de overlays** con cuatro archivos:

```
mi-plantilla/
├── overlay.json   # metadata: nombre y campos
├── index.html     # estructura base del overlay
├── style.css      # estilos
└── script.js      # lógica (WebSocket + show/update/hide)
```

### `overlay.json`

El **nombre de la carpeta es el identificador de la plantilla** (`TEMPLATE_ID`). Un campo `id` a nivel raíz es opcional y solo se usa para sobreescribir ese identificador desde el archivo; en los ejemplos no se usa.

```json
{
  "name": "Mi plantilla",
  "fields": [
    {
      "key": "titulo",
      "label": "Título",
      "type": "text",
      "default": "Texto de ejemplo"
    },
    {
      "key": "color_fondo",
      "label": "Color de fondo",
      "type": "color",
      "default": "#cc241dff"
    },
    {
      "key": "avance",
      "label": "Avance",
      "type": "progress",
      "min": 0,
      "max": 100,
      "default": "50"
    },
    {
      "key": "mostrar_extra",
      "label": "Mostrar extra",
      "type": "boolean",
      "default": "true"
    }
  ]
}
```

Tipos de campo soportados:

| Tipo       | Control en la UI      | Reglas                                                                                    |
| ---------- | --------------------- | ----------------------------------------------------------------------------------------- |
| `text`     | Campo de texto        | `default` opcional.                                                                       |
| `color`    | Selector de color     | Formato `#rrggbb` o `#rrggbbaa` (con alpha).                                              |
| `progress` | Deslizador + numérico | `min` y `max` **obligatorios** con `min < max`; `default` opcional (si falta, usa `min`). |
| `boolean`  | Interruptor on/off    | `default` opcional: `"true"` o `"false"` (si falta, arranca apagado).                     |

> Los campos inválidos se descartan al descubrir la plantilla (no rompen la app): un `progress` sin `min`/`max` o con rango invertido, o un `boolean` con `default` distinto de `"true"`/`"false"`. El backend registra en consola qué campo descartó y por qué.

### `script.js`

El script del overlay es el responsable de conectar y reaccionar. Contrato mínimo:

- `const TEMPLATE_ID = "mi-plantilla"` — debe coincidir con el nombre de la carpeta (es el id de la plantilla).
- `const INSTANCE_ID = new URLSearchParams(location.search).get("instance")` — para filtrar los mensajes de esta instancia.
- WebSocket en `ws://${location.host}/ws` con **reconexión cada 2 segundos** si se cae.
- Filtrar cada mensaje por `TEMPLATE_ID` y, si `INSTANCE_ID` está presente, por `instance_id`.
- Implementar los tres handlers:
  - `show(fields)` → actualiza el contenido y dispara la animación de entrada.
  - `update(fields)` → actualiza el contenido sin re-animar.
  - `hide()` → dispara la animación de salida.
- **Fondo transparente** en `body` (requisito de OBS).

Un ejemplo mínimo de conexión y filtrado:

```js
const INSTANCE_ID = new URLSearchParams(location.search).get("instance");
const TEMPLATE_ID = "mi-plantilla";

function connect() {
  const ws = new WebSocket(`ws://${location.host}/ws`);
  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.template !== TEMPLATE_ID) return;
    if (INSTANCE_ID && msg.instance_id !== INSTANCE_ID) return;
    switch (msg.action) {
      case "show":
        show(msg.fields || {});
        break;
      case "update":
        update(msg.fields || {});
        break;
      case "hide":
        hide();
        break;
    }
  };
  ws.onclose = () => setTimeout(connect, 2000);
}
connect();
```

### Ver la plantilla en la app

La detección es automática y basada en archivos: basta con crear la carpeta. Para que aparezca en la grilla, presione **Recargar** en la página Overlays o reinicie la app. Si algún campo o el nombre da problemas, revise la consola (la app registra qué archivos descarta y por qué).

Use las plantillas de la carpeta `examples/` como referencia de implementación completa.

---

## Servidor local y protocolo (resumen)

Con la app corriendo, el servidor expone:

| Recurso          | Descripción                                                      |
| ---------------- | ---------------------------------------------------------------- |
| `/overlay/...`   | Archivos de los overlays (HTML/CSS/JS/JSON/imágenes).            |
| `/api/templates` | Manifiesto JSON con las plantillas y sus campos.                 |
| `/ws`            | WebSocket por el que viajan los comandos `show`/`update`/`hide`. |

Cada mensaje del WebSocket tiene esta forma:

```json
{
  "instance_id": "uuid-de-la-instancia",
  "template": "lower-third-basico",
  "action": "show",
  "fields": { "titulo": "Federico Gomez", "subtitulo": "Dev Backend" }
}
```

Para verificar que el servidor está vivo y qué plantillas hay:

```bash
curl http://127.0.0.1:4848/api/templates
```

El detalle completo del servidor (broadcast, replay de estado, validaciones y arquitectura del backend) está en [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Soporte y desarrollo

¿Desea desarrollar sobre la app, compilarla desde el código o entender la arquitectura? Todo el detalle técnico (requisitos, puesta en marcha, comandos, estructura, CI y releases) está en **[CONTRIBUTING.md](CONTRIBUTING.md)**.

¿Encontró un bug o quiere proponer una función nueva? Las release notes están en [CHANGELOG.md](CHANGELOG.md).
