const params = new URLSearchParams(location.search);
const INSTANCE_ID = params.get("instance");
const TEMPLATE_ID = "lower-third-basico";
const WS_URL = `ws://${location.host}/ws`;

const zocalo = document.getElementById("zocalo");
const tituloEl = document.getElementById("titulo");
const subtituloEl = document.getElementById("subtitulo");

function onTransitionEnd(event) {
  if (event.target !== zocalo || event.propertyName !== "opacity") return;
  zocalo.classList.remove("salir");
}

function show(fields) {
  update(fields);
  if (zocalo.classList.contains("entrar")) return;
  zocalo.classList.remove("salir");
  zocalo.classList.add("entrar");
  console.log(`[overlay] ${TEMPLATE_ID} show`);
}

function hide() {
  if (
    zocalo.classList.contains("salir") ||
    !zocalo.classList.contains("entrar")
  )
    return;
  zocalo.classList.remove("entrar");
  zocalo.classList.add("salir");
  zocalo.addEventListener("transitionend", onTransitionEnd, { once: true });
  console.log(`[overlay] ${TEMPLATE_ID} hide`);
}

function update(fields) {
  if (fields.titulo !== undefined) tituloEl.textContent = fields.titulo;
  if (fields.subtitulo !== undefined)
    subtituloEl.textContent = fields.subtitulo;
  if (fields.color_titulo !== undefined)
    tituloEl.style.color = fields.color_titulo;
  if (fields.color_subtitulo !== undefined)
    subtituloEl.style.color = fields.color_subtitulo;
  if (fields.color_acento !== undefined)
    zocalo.style.borderLeftColor = fields.color_acento;
  console.log(`[overlay] ${TEMPLATE_ID} update`);
}

function handleMessage(raw) {
  let msg;
  try {
    msg = JSON.parse(raw);
  } catch {
    return;
  }
  if (msg.template !== TEMPLATE_ID) return;
  if (INSTANCE_ID && msg.instance_id !== INSTANCE_ID) return;

  switch (msg.action) {
    case "show":
      show(msg.fields || {});
      break;
    case "hide":
      hide();
      break;
    case "update":
      update(msg.fields || {});
      break;
  }
}

function connect() {
  const ws = new WebSocket(WS_URL);

  ws.onmessage = (event) => handleMessage(event.data);

  ws.onclose = () => setTimeout(connect, 2000);
}

connect();
