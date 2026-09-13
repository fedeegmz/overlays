import type { TemplateField } from "./template";

/** Mirror of the Rust `GeneratedFiles` (raw contents for the escaped preview). */
export interface GeneratedFiles {
  overlay_json: string;
  index_html: string;
  style_css: string;
  script_js: string;
}

/** Mirror of the Rust `ValidationIssue` (i18n key: `errors.generation.issue.<code>`). */
export interface ValidationIssue {
  code: string;
  params: Record<string, string>;
}

/** Mirror of the Rust `GeneratedOverlaySummary` (staging + template identity). */
export interface GeneratedOverlaySummary {
  staging_id: string;
  directory: string;
  name: string;
  fields: TemplateField[];
  files: GeneratedFiles;
}
