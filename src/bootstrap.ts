import { useConfigStore } from "./stores/config";
import { useInstanceStore } from "./stores/instances";
import { usePresetStore } from "./stores/presets";
import { useTemplateStore } from "./stores/templates";

export function bootstrapStores(): void {
  const config = useConfigStore();
  const templates = useTemplateStore();
  const presets = usePresetStore();
  const instances = useInstanceStore();

  void config.refreshConfig();
  void templates.refreshTemplates();
  void presets.refreshPresets();
  void instances.fetchServerPort();
}
