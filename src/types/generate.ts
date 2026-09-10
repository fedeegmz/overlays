import type { TemplateField } from "./template";

/** Mirror of the Rust `GeneratedOverlaySummary` (staging + template identity). */
export interface GeneratedOverlaySummary {
  staging_id: string;
  directory: string;
  name: string;
  fields: TemplateField[];
}
