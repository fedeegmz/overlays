/** Pages reachable from the side navigation. */
export type NavPage = "overlays" | "checklists" | "settings";

/** Every page the app can render; `detail` is reached from `overlays`. */
export type Page = NavPage | "detail";
