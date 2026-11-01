export interface OverlayInstance {
  id: string;
  templateId: string;
  /** Fixed per-template number assigned at creation; drives the default label. */
  number: number;
  customName?: string | null;
  /** Label computed by the backend (custom name or auto-numbered default). */
  displayName: string;
  fields: Record<string, string>;
  visible: boolean;
}

export interface EventSource {
  kind: "desktop" | "remote";
  clientId: string;
}

/** Full registry state pushed by the backend on every mutation. */
export interface InstancesSnapshot {
  revision: number;
  source: EventSource;
  instances: OverlayInstance[];
}
