<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { commandErrorMessage } from "../lib/errors";
import { resolveOverlayAssetPath } from "../services/assetsApi";
import { pickFile } from "../services/dialogApi";
import { useConfigStore } from "../stores/config";

const props = defineProps<{
  modelValue: string;
  inputId?: string;
  disabled?: boolean;
  templateId: string;
  accept?: string[];
}>();

const emit = defineEmits<(e: "update:modelValue", value: string) => void>();

const { t } = useI18n();
const configStore = useConfigStore();

const picking = ref(false);
const pickError = ref<string | null>(null);

async function handlePick() {
  if (picking.value) return;
  pickError.value = null;
  picking.value = true;
  try {
    const selected = await pickFile({
      title: t("fileField.choose"),
      defaultPath: configStore.appConfig.overlays_dir,
      accept: props.accept,
    });
    if (!selected) return;
    const relative = await resolveOverlayAssetPath(props.templateId, selected);
    emit("update:modelValue", relative);
  } catch (err) {
    pickError.value = commandErrorMessage(err);
  } finally {
    picking.value = false;
  }
}

function handleClear() {
  pickError.value = null;
  emit("update:modelValue", "");
}
</script>

<template>
  <div class="file-field">
    <input
      :id="inputId"
      class="field-input file-value"
      type="text"
      :value="modelValue"
      :disabled="disabled"
      :placeholder="t('fileField.placeholder')"
      @input="
        emit('update:modelValue', ($event.target as HTMLInputElement).value)
      "
    >
    <div class="file-actions">
      <button
        type="button"
        class="btn file-btn"
        :disabled="disabled || picking"
        @click="handlePick"
      >
        {{ picking ? t("fileField.picking") : t("fileField.choose") }}
      </button>
      <button
        v-if="modelValue"
        type="button"
        class="btn btn-ghost file-btn"
        :disabled="disabled"
        @click="handleClear"
      >
        {{ t("fileField.clear") }}
      </button>
    </div>
    <p v-if="pickError" class="file-error">{{ pickError }}</p>
  </div>
</template>

<style scoped>
.file-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.file-value {
  flex: 1;
}

.file-actions {
  display: flex;
  gap: 8px;
}

.file-btn {
  padding: 7px 12px;
  font-size: 13px;
}

.file-error {
  color: var(--danger);
  font-size: 12.5px;
}
</style>
