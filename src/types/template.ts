export interface TemplateField {
  key: string;
  label: string;
  type: string;
  default?: string;
  min?: number;
  max?: number;
  accept?: string[];
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
