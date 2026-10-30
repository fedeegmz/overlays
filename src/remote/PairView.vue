<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import { commandErrorMessage } from "../lib/errors";
import type { CommandError } from "../types";

defineProps<{
  pairing: boolean;
  error: CommandError | null;
  needsRepair: boolean;
}>();

const emit = defineEmits<(e: "submit", pin: string) => void>();

const { t } = useI18n();
const pin = ref("");
</script>

<template>
  <form class="panel" @submit.prevent="emit('submit', pin)">
    <h2 class="panel-title">{{ t("remote.pair.title") }}</h2>
    <p v-if="needsRepair" class="notice" role="status">
      {{ t("remote.repair") }}
    </p>
    <p class="hint">{{ t("remote.pair.description") }}</p>
    <label class="field-label" for="pin">{{ t("remote.pair.pinLabel") }}</label>
    <input
      id="pin"
      v-model="pin"
      class="field-input pin-input"
      type="text"
      inputmode="numeric"
      autocomplete="one-time-code"
      maxlength="6"
      :disabled="pairing"
    >
    <p v-if="error" class="error" role="alert">
      {{ commandErrorMessage(error) }}
    </p>
    <button class="btn btn-primary submit" type="submit" :disabled="pairing">
      {{ pairing ? t("remote.pair.pairing") : t("remote.pair.submit") }}
    </button>
  </form>
</template>

<style scoped>
.panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 18px;
  box-shadow: var(--shadow-panel);
}
.panel-title {
  font-size: 16px;
  margin-bottom: 8px;
}
.hint {
  font-size: 13px;
  color: var(--text-secondary);
  margin-bottom: 14px;
}
.notice {
  font-size: 13px;
  background: var(--accent-soft);
  border: 1px solid var(--accent-soft-border);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  margin-bottom: 12px;
}
.error {
  font-size: 13px;
  color: var(--danger);
  background: var(--danger-soft);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  margin-top: 10px;
}
.pin-input {
  font-size: 22px;
  letter-spacing: 0.35em;
  text-align: center;
}
.submit {
  width: 100%;
  margin-top: 14px;
  padding: 11px;
}
</style>
