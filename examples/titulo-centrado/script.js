const params = new URLSearchParams(location.search);
const INSTANCE_ID = params.get("instance");
const TEMPLATE_ID = "titulo-centrado";
const WS_URL = `ws://${location.host}/ws`;

const tituloEl = document.getElementById("titulo");

function onTransitionEnd(event) {
  if (event.target !== tituloEl || event.propertyName !== "opacity") return;
  tituloEl.classList.remove("salir");
}

function show(fields) {
  update(fields);
  if (tituloEl.classList.contains("entrar")) return;
  tituloEl.classList.remove("salir");
  tituloEl.classList.add("entrar");
  console.log(`[overlay] ${TEMPLATE_ID} show`);
}

function hide() {
  if (tituloEl.classList.contains("salir") || !tituloEl.classList.contains("entrar")) return;
  tituloEl.classList.remove("entrar");
  tituloEl.classList.add("salir");
  tituloEl.addEventListener("transitionend", onTransitionEnd, { once: true });
  console.log(`[overlay] ${TEMPLATE_ID} hide`);
}

function update(fields) {
  if (fields.texto !== undefined) tituloEl.textContent = fields.texto;
  if (fields.color_texto !== undefined) tituloEl.style.color = fields.color_texto;
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
