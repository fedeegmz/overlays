export interface OverlayInstance {
  id: string;
  templateId: string;
  /** Fixed per-template number assigned at creation; drives the default label. */
  number: number;
  customName?: string;
  fields: Record<string, string>;
  visible: boolean;
}
