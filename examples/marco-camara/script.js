const params = new URLSearchParams(location.search);
const INSTANCE_ID = params.get("instance");
const TEMPLATE_ID = "marco-camara";
const WS_URL = `ws://${location.host}/ws`;

const marco = document.getElementById("marco");

const state = {
  color: "#38bdf8",
  grosor: 6,
  radio: 0,
};

function roundedRectPath(x, y, w, h, r) {
  const rr = Math.min(r, w / 2, h / 2);
  return [
    `M ${x + rr} ${y}`,
    `H ${x + w - rr}`,
    `A ${rr} ${rr} 0 0 1 ${x + w} ${y + rr}`,
    `V ${y + h - rr}`,
    `A ${rr} ${rr} 0 0 1 ${x + w - rr} ${y + h}`,
    `H ${x + rr}`,
    `A ${rr} ${rr} 0 0 1 ${x} ${y + h - rr}`,
    `V ${y + rr}`,
    `A ${rr} ${rr} 0 0 1 ${x + rr} ${y}`,
    "Z",
  ].join(" ");
}

function renderFrame() {
  const width = marco.offsetWidth;
  const height = marco.offsetHeight;
  const g = state.grosor;
  const outer = `M 0 0 H ${width} V ${height} H 0 Z`;
  const inner = roundedRectPath(
    g,
    g,
    width - g * 2,
    height - g * 2,
    state.radio,
  );
  const svg = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}"`,
    ` viewBox="0 0 ${width} ${height}">`,
    `<path fill="${state.color}" fill-rule="evenodd" d="${outer} ${inner}"/>`,
    "</svg>",
  ].join("");
  marco.style.backgroundImage = `url("data:image/svg+xml;utf8,${encodeURIComponent(svg)}")`;
}

function onTransitionEnd(event) {
  if (event.target !== marco || event.propertyName !== "opacity") return;
  marco.classList.remove("salir");
}

function show(fields) {
  update(fields);
  if (marco.classList.contains("entrar")) return;
  marco.classList.remove("salir");
  marco.classList.add("entrar");
  console.log(`[overlay] ${TEMPLATE_ID} show`);
}

function hide() {
  if (marco.classList.contains("salir") || !marco.classList.contains("entrar"))
    return;
  marco.classList.remove("entrar");
  marco.classList.add("salir");
  marco.addEventListener("transitionend", onTransitionEnd, { once: true });
  console.log(`[overlay] ${TEMPLATE_ID} hide`);
}

function update(fields) {
  if (fields.color !== undefined) state.color = fields.color;
  if (fields.grosor !== undefined) state.grosor = Number(fields.grosor);
  if (fields.radio !== undefined) state.radio = Number(fields.radio);
  renderFrame();
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

renderFrame();
window.addEventListener("resize", renderFrame);
connect();
