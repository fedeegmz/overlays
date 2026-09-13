<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import GenericModal from "./GenericModal.vue";

const props = defineProps<{
  /** null → modal closed; "noDir" | "keyring" | "noKey" → the matching gate copy. */
  gate: "noDir" | "keyring" | "noKey" | null;
}>();

const emit = defineEmits<{
  close: [];
  openSettings: [];
}>();

const { t } = useI18n();

const copy = computed(() => {
  switch (props.gate) {
    case "noDir":
      return {
        title: t("gate.noDirTitle"),
        body: t("gate.noDirBody"),
      };
    case "keyring":
      return {
        title: t("gate.keyringTitle"),
        body: t("gate.keyringBody"),
      };
    case "noKey":
      return {
        title: t("gate.noKeyTitle"),
        body: t("gate.noKeyBody"),
      };
    default:
      return null;
  }
});
</script>

<template>
  <GenericModal
    v-if="gate"
    :title="copy!.title"
    :open="true"
    @close="emit('close')"
  >
    <p class="gate-body" data-testid="gate-body">{{ copy!.body }}</p>
    <template #footer>
      <button type="button" @click="emit('close')">
        {{ t("gate.close") }}
      </button>
      <button
        type="button"
        class="primary"
        data-testid="gate-open-settings"
        @click="emit('openSettings')"
      >
        {{ t("gate.openSettings") }}
      </button>
    </template>
  </GenericModal>
</template>

<style scoped>
.gate-body {
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-secondary);
}
</style>
