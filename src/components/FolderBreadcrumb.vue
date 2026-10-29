<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { breadcrumbSegments } from "../lib/folderTree";

const props = defineProps<{
  current: string;
}>();

const emit = defineEmits<{
  navigate: [path: string];
}>();

const { t } = useI18n();
const segments = computed(() => breadcrumbSegments(props.current));
</script>

<template>
  <nav class="breadcrumb" :aria-label="t('overlaysPage.breadcrumbLabel')">
    <button
      type="button"
      class="crumb"
      :class="{ active: segments.length === 0 }"
      @click="emit('navigate', '')"
    >
      {{ t("overlaysPage.rootFolder") }}
    </button>
    <template v-for="(segment, i) in segments" :key="segment.path">
      <span class="sep" aria-hidden="true">/</span>
      <button
        type="button"
        class="crumb"
        :class="{ active: i === segments.length - 1 }"
        @click="emit('navigate', segment.path)"
      >
        {{ segment.name }}
      </button>
    </template>
  </nav>
</template>

<style scoped>
.breadcrumb {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 16px;
  font-size: 13px;
}

.crumb {
  padding: 3px 6px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-family: inherit;
  font-size: inherit;
  cursor: pointer;
}

.crumb:hover {
  color: var(--text-primary);
}

.crumb.active {
  color: var(--text-primary);
  font-weight: 600;
}

.sep {
  color: var(--text-faint);
}
</style>
