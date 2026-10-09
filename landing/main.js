const detectOs = () => {
  const ua = navigator.userAgent || "";

  if (/Windows/i.test(ua)) {
    return { os: "windows", label: "Windows" };
  }
  if (/Macintosh|Mac OS X/i.test(ua)) {
    return { os: "mac", label: "macOS" };
  }
  return { os: "linux", label: "Linux" };
};

const { os, label } = detectOs();

document
  .querySelector(`.os-card[data-os="${os}"]`)
  ?.classList.add("recommended");

const heroBtn = document.getElementById("hero-download-btn");
if (heroBtn) {
  heroBtn.textContent = `Descargar para ${label}`;
}
