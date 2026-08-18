export interface TemplateField {
  key: string;
  label: string;
  type: string;
  default?: string;
}

export interface TemplateInfo {
  id: string;
  name: string;
  path: string;
  fields: TemplateField[];
}

export interface Manifest {
  templates: TemplateInfo[];
}

export interface Preset {
  nombre: string;
  template: string;
  fields: Record<string, string>;
}

export interface AppConfig {
  overlays_dir: string | null;
}

export interface OverlayInstance {
  id: string;
  templateId: string;
  fields: Record<string, string>;
  visible: boolean;
}
