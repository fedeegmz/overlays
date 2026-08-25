<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useI18n } from "vue-i18n";
import logoSvg from "../assets/logo.svg";
import { useInstanceStore } from "../stores/instances";

defineProps<{
  currentPage: "overlays" | "settings";
}>();

const emit = defineEmits<{
  navigate: [page: "overlays" | "settings"];
  openDetail: [];
}>();

const { t } = useI18n();

const instanceStore = useInstanceStore();
const { instances, activeInstanceId } = storeToRefs(instanceStore);
const { instanceDisplayName, selectInstance, removeInstance } = instanceStore;

function handleInstanceClick(id: string) {
  selectInstance(id);
  emit("openDetail");
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <img :src="logoSvg" alt="Overlays" class="brand-mark">
      <div class="brand-name">Overlays</div>
    </div>

    <nav class="nav">
      <button
        type="button"
        class="nav-item"
        :class="{ active: currentPage === 'overlays' }"
        @click="emit('navigate', 'overlays')"
      >
        <svg
          aria-hidden="true"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <rect x="3" y="3" width="7" height="7" rx="1.5" />
          <rect x="14" y="3" width="7" height="7" rx="1.5" />
          <rect x="3" y="14" width="7" height="7" rx="1.5" />
          <rect x="14" y="14" width="7" height="7" rx="1.5" />
        </svg>
        {{ t("sidebar.overlays") }}
      </button>
    </nav>

    <div v-if="instances.length > 0" class="instances-section">
      <div class="instances-header">{{ t("sidebar.instances") }}</div>
      <div class="instances-list">
        <!-- biome-ignore lint/a11y/useSemanticElements: row contains a nested close button, so a real button element would be invalid HTML -->
        <div
          v-for="instance in instances"
          :key="instance.id"
          class="instance-item"
          :class="{ active: instance.id === activeInstanceId }"
          role="button"
          tabindex="0"
          @click="handleInstanceClick(instance.id)"
          @keydown.enter.prevent="handleInstanceClick(instance.id)"
        >
          <span
            class="instance-dot"
            :class="instance.visible ? 'dot-live' : 'dot-idle'"
          ></span>
          <span class="instance-name">{{ instanceDisplayName(instance) }}</span>
          <button
            type="button"
            class="instance-close"
            :title="t('sidebar.closeInstance')"
            @click.stop="removeInstance(instance.id)"
          >
            ×
          </button>
        </div>
      </div>
    </div>

    <div class="sidebar-spacer"></div>

    <button
      type="button"
      class="nav-item"
      :class="{ active: currentPage === 'settings' }"
      @click="emit('navigate', 'settings')"
    >
      <svg
        aria-hidden="true"
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <circle cx="12" cy="12" r="3" />
        <path
          d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"
        />
      </svg>
      {{ t("sidebar.settings") }}
    </button>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 232px;
  flex-shrink: 0;
  background: var(--surface);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 18px 12px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 4px 8px 20px 8px;
}

.brand-mark {
  width: 22px;
  height: 22px;
  border-radius: 6px;
  flex-shrink: 0;
}

.brand-name {
  font-weight: 650;
  font-size: 14.5px;
  letter-spacing: -0.01em;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  font-size: 13.5px;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  border: none;
  background: none;
  text-align: left;
  width: 100%;
  transition:
    background 0.12s ease,
    color 0.12s ease;
}

.nav-item svg {
  opacity: 0.75;
  flex-shrink: 0;
}

.nav-item:hover {
  background: var(--surface-sunken);
  color: var(--text);
}

.nav-item.active {
  background: var(--accent-soft);
  color: var(--accent-hover);
}

.nav-item.active svg {
  opacity: 1;
}

.instances-section {
  margin-top: 18px;
  border-top: 1px solid var(--border);
  padding-top: 14px;
  max-height: 50%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.instances-header {
  font-size: 11px;
  font-weight: 650;
  color: var(--text-faint);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 0 10px 8px;
  flex-shrink: 0;
}

.instances-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
}

.instance-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: var(--radius-sm);
  font-size: 12.5px;
  font-weight: 500;
  color: var(--text-secondary);
  cursor: pointer;
  transition:
    background 0.12s ease,
    color 0.12s ease;
}

.instance-item:hover {
  background: var(--surface-sunken);
  color: var(--text);
}

.instance-item.active {
  background: var(--accent-soft);
  color: var(--accent-hover);
}

.instance-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dot-live {
  background: var(--success);
  box-shadow: 0 0 0 2px var(--success-soft);
}

.dot-idle {
  background: var(--text-faint);
}

.instance-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.instance-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  border-radius: 4px;
  border: none;
  background: none;
  color: var(--text-faint);
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  opacity: 0;
  transition:
    opacity 0.12s ease,
    background 0.12s ease,
    color 0.12s ease;
}

.instance-item:hover .instance-close {
  opacity: 1;
}

.instance-close:hover {
  background: var(--danger-soft);
  color: var(--danger);
}

.sidebar-spacer {
  flex: 1;
}
</style>
