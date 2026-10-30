import { describe, expect, it } from "vitest";
import { createSSRApp, h } from "vue";
import { renderToString } from "vue/server-renderer";
import type { TemplateField } from "../types";
import FieldEditor from "./FieldEditor.vue";

const fields: TemplateField[] = [
  { key: "title", label: "Title", type: "text" },
  { key: "logo", label: "Logo", type: "file" },
];

function render(slots?: Record<string, () => unknown>): Promise<string> {
  const app = createSSRApp({
    render: () =>
      h(
        FieldEditor,
        {
          fields,
          values: { title: "Hello", logo: "img/logo.png" },
        },
        slots as never,
      ),
  });
  return renderToString(app);
}

describe("FieldEditor", () => {
  it("renders_text_input_with_current_value", async () => {
    const html = await render();
    expect(html).toContain('id="field-title"');
    expect(html).toContain('value="Hello"');
  });

  it("file_field_defaults_to_read_only_value_without_input", async () => {
    const html = await render();
    expect(html).toContain("img/logo.png");
    expect(html).not.toContain('id="field-logo"');
  });

  it("file_slot_replaces_the_default_file_rendering", async () => {
    const html = await render({ file: () => h("span", "custom-file-slot") });
    expect(html).toContain("custom-file-slot");
    expect(html).not.toContain("img/logo.png");
  });
});
