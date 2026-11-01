<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useConfirmClick } from "../composables/useConfirmClick";
import { qrSvg } from "../lib/qr";
import { useRemoteStore } from "../stores/remote";
import BooleanField from "./BooleanField.vue";

const POLL_MS = 2000;
const TICK_MS = 1000;

const { t } = useI18n();
const remoteStore = useRemoteStore();
const {
  enabled,
  pairedDevices,
  lanUrls,
  pairing,
  selectedUrl,
  error,
  qrPayload,
} = storeToRefs(remoteStore);

const now = ref(Date.now());
const secondsLeft = computed(() =>
  pairing.value
    ? Math.max(0, Math.ceil((pairing.value.expiresAtMs - now.value) / 1000))
    : 0,
);
const expired = computed(
  () => pairing.value !== null && secondsLeft.value <= 0,
);
const qr = computed(() =>
  qrPayload.value && !expired.value ? qrSvg(qrPayload.value) : null,
);
const currentUrl = computed(() => selectedUrl.value ?? lanUrls.value[0] ?? "");

const { pending: confirmingRevoke, confirmClick } = useConfirmClick<"revoke">(
  () => void remoteStore.revoke(),
  4000,
);

let pollTimer: ReturnType<typeof setInterval> | null = null;
let tickTimer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  void remoteStore.refresh();
  // Pairing happens on the phone, so the paired-device count is polled.
  pollTimer = setInterval(() => {
    if (enabled.value) void remoteStore.refresh();
  }, POLL_MS);
  tickTimer = setInterval(() => {
    now.value = Date.now();
  }, TICK_MS);
});

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
  if (tickTimer) clearInterval(tickTimer);
  if (pairing.value) void remoteStore.cancelPairing();
});

function onToggle(value: string) {
  now.value = Date.now();
  void remoteStore.setEnabled(value === "true");
}

function onSelectUrl(event: Event) {
  remoteStore.selectUrl((event.target as HTMLSelectElement).value);
}
</script>

<template>
  <div class="settings-section">
    <div class="settings-row">
      <div>
        <div class="settings-row-label">
          {{ t("settings.remote.enabled.label") }}
        </div>
        <div class="settings-row-desc">
          {{ t("settings.remote.enabled.description") }}
        </div>
      </div>
      <BooleanField
        :model-value="enabled ? 'true' : 'false'"
        @update:model-value="onToggle"
      />
    </div>

    <template v-if="enabled">
      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.remote.pairing.title") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.remote.pairing.scan") }}
          </div>
          <template v-if="pairing">
            <div class="pin-label">{{ t("settings.remote.pairing.pin") }}</div>
            <div class="pin" data-testid="remote-pin">{{ pairing.pin }}</div>
            <div class="settings-row-desc">
              {{ expired
                  ? t("settings.remote.pairing.expired")
                  : t("settings.remote.pairing.expiresIn", { seconds: secondsLeft }) }}
            </div>
          </template>
          <div class="pairing-actions">
            <button
              type="button"
              class="btn"
              @click="remoteStore.startPairing()"
            >
              {{ t("settings.remote.pairing.newPin") }}
            </button>
            <button
              v-if="pairing"
              type="button"
              class="btn btn-ghost"
              @click="remoteStore.cancelPairing()"
            >
              {{ t("settings.remote.pairing.cancel") }}
            </button>
          </div>
        </div>
        <!-- The SVG is generated locally from the pairing URL; no user markup. -->
        <div v-if="qr" class="qr" v-html="qr" />
      </div>

      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.remote.url.label") }}
          </div>
        </div>
        <div class="settings-row-right">
          <select
            v-if="lanUrls.length > 1"
            class="settings-select"
            :value="currentUrl"
            @change="onSelectUrl"
          >
            <option v-for="url in lanUrls" :key="url" :value="url">
              {{ url }}
            </option>
          </select>
          <span v-else-if="lanUrls.length === 1" class="settings-row-value">
            {{ lanUrls[0] }}
          </span>
          <span v-else class="settings-row-value settings-row-empty">
            {{ t("settings.remote.url.unavailable") }}
          </span>
        </div>
      </div>

      <div class="settings-row">
        <div>
          <div class="settings-row-label">
            {{ t("settings.remote.devices.label") }}
          </div>
          <div class="settings-row-desc">
            {{ t("settings.remote.devices.count", { count: pairedDevices }) }}
          </div>
          <div v-if="confirmingRevoke" class="settings-row-desc warning">
            {{ t("settings.remote.revoke.warning") }}
          </div>
        </div>
        <button
          type="button"
          class="btn btn-danger-ghost"
          :disabled="pairedDevices === 0"
          @click="confirmClick('revoke')"
        >
          {{ confirmingRevoke
              ? t("settings.remote.revoke.confirm")
              : t("settings.remote.revoke.action") }}
        </button>
      </div>
    </template>

    <p v-if="error" class="config-error">{{ error }}</p>
  </div>
</template>

<style scoped>
.settings-section {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-panel);
  margin-bottom: 18px;
}

.settings-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px;
  gap: 20px;
}

.settings-row-label {
  font-size: 13.5px;
  font-weight: 600;
}

.settings-row-desc {
  font-size: 12.5px;
  color: var(--text-secondary);
  margin-top: 3px;
}

.settings-row-desc.warning {
  color: var(--danger);
}

.settings-row-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.settings-row-value {
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
  color: var(--text-secondary);
  background: var(--surface-sunken);
  padding: 6px 10px;
  border-radius: var(--radius-sm);
}

.settings-row-empty {
  color: var(--text-faint);
  font-style: italic;
  font-family: inherit;
}

.settings-select {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  color: var(--text);
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 12px;
}

.pin-label {
  margin-top: 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.pin {
  font-family: "SF Mono", "JetBrains Mono", monospace;
  font-size: 28px;
  font-weight: 650;
  letter-spacing: 0.2em;
}

.pairing-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.qr {
  width: 180px;
  height: 180px;
  flex-shrink: 0;
  background: #fff;
  border-radius: var(--radius-sm);
  padding: 4px;
}

.qr :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}

.config-error {
  padding: 0 18px 14px;
  color: var(--danger);
  font-size: 12.5px;
}
</style>
