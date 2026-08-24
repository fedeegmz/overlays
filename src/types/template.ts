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
