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
  language: string | null;
}

export interface OverlayInstance {
  id: string;
  templateId: string;
  fields: Record<string, string>;
  visible: boolean;
}

export interface CommandError {
  code: string;
  params?: Record<string, string>;
}

export function isCommandError(value: unknown): value is CommandError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof (value as { code: unknown }).code === "string"
  );
}
