export interface OverlayInstance {
  id: string;
  templateId: string;
  customName?: string;
  fields: Record<string, string>;
  visible: boolean;
}
