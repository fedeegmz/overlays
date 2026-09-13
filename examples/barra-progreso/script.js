const params = new URLSearchParams(location.search);
const INSTANCE_ID = params.get("instance");
const TEMPLATE_ID = "barra-progreso";
const WS_URL = `ws://${location.host}/ws`;

const contenedorEl = document.getElementById("barra-contenedor");
const etiquetaEl = document.getElementById("barra-etiqueta");
const valorEl = document.getElementById("barra-valor");
const rellenoEl = document.getElementById("barra-relleno");

function update(fields) {
  if (fields.etiqueta !== undefined) {
    etiquetaEl.textContent = fields.etiqueta;
  }
  if (fields.valor !== undefined) {
    const n = Number(fields.valor);
    if (Number.isFinite(n)) {
      rellenoEl.style.width = `${n}%`;
      valorEl.textContent = `${n}%`;
    }
  }
  if (fields.color_barra !== undefined) {
    rellenoEl.style.backgroundColor = fields.color_barra;
  }
  console.log(`[overlay] ${TEMPLATE_ID} update`);
}

function show(fields) {
  update(fields);
  contenedorEl.classList.add("entrar");
  console.log(`[overlay] ${TEMPLATE_ID} show`);
}

function hide() {
  contenedorEl.classList.remove("entrar");
  console.log(`[overlay] ${TEMPLATE_ID} hide`);
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
