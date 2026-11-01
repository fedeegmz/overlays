import qrcode from "qrcode-generator";

/** Renders `text` as a scalable SVG string (generated locally, no user markup). */
export function qrSvg(text: string): string {
  const qr = qrcode(0, "M");
  qr.addData(text);
  qr.make();
  return qr.createSvgTag({ cellSize: 4, margin: 2, scalable: true });
}
