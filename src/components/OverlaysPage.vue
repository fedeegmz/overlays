<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useConfigStore } from "../stores/config";
import { useInstanceStore } from "../stores/instances";
import { useTemplateStore } from "../stores/templates";
import GateModal from "./GateModal.vue";
import OverlayGrid from "./OverlayGrid.vue";

const { t } = useI18n();
const templateStore = useTemplateStore();
const instanceStore = useInstanceStore();
const configStore = useConfigStore();

const { templates, templatesError } = storeToRefs(templateStore);
const { providerKeys, keyringAvailable, appConfig } = storeToRefs(configStore);
const { createInstance } = instanceStore;

const emit = defineEmits<{
  openDetail: [];
  navigate: [page: "overlays" | "settings" | "generate"];
}>();

const ready = ref(false);
const dismissed = ref(false);

onMounted(async () => {
  await Promise.all([
    configStore.refreshProviderKeys(),
    configStore.refreshConfig(),
  ]);
  ready.value = true;
});

/** The G1 button is only visible when the generator is enabled (item 15). */
const generatorVisible = computed(
  () => ready.value && appConfig.value.ai_generator_enabled !== false,
);

/** Same G2/G3 gate the Generate page uses — the entry button leads with it. */
const gate = computed<"noDir" | "keyring" | "noKey" | null>(() => {
  if (!ready.value) return null;
  if (!appConfig.value.overlays_dir) return "noDir";
  if (keyringAvailable.value === false) return "keyring";
  if (providerKeys.value.length === 0) return "noKey";
  return null;
});

watch(gate, () => {
  dismissed.value = false;
});

const gateOpen = computed(() => gate.value !== null && !dismissed.value);

function handleGenerate(): void {
  if (gate.value !== null) return; // modal is already on screen
  emit("navigate", "generate");
}

function openSettings(): void {
  dismissed.value = true;
  emit("navigate", "settings");
}

function handleSelect(id: string) {
  createInstance(id);
  emit("openDetail");
}
</script>

<template>
  <div class="overlays-page">
    <div class="page-header">
      <div>
        <div class="page-title">Overlays</div>
        <div class="page-subtitle">{{ t("overlaysPage.subtitle") }}</div>
      </div>
      <button
        v-if="generatorVisible"
        type="button"
        class="btn primary"
        data-testid="generate-entry"
        @click="handleGenerate"
      >
        {{ t("overlaysPage.generateButton") }}
      </button>
    </div>

    <p v-if="templatesError" class="error">{{ templatesError }}</p>
    <p v-else-if="templates === null" class="muted">
      {{ t("overlaysPage.loading") }}
    </p>
    <p v-else-if="templates.length === 0" class="muted">
      {{ t("overlaysPage.empty") }}
    </p>
    <OverlayGrid v-else :templates="templates" @select="handleSelect" />

    <GateModal
      :gate="gateOpen ? gate : null"
      @close="dismissed = true"
      @open-settings="openSettings"
    />
  </div>
</template>

<style scoped>
.overlays-page {
  max-width: 980px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 28px;
}

.page-title {
  font-size: 22px;
  font-weight: 650;
  letter-spacing: -0.015em;
}

.page-subtitle {
  font-size: 13.5px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.error {
  color: var(--danger);
  font-size: 13.5px;
}

.muted {
  color: var(--text-faint);
  font-size: 13.5px;
}
</style>
