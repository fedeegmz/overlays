<script setup lang="ts">
import type { TemplateInfo } from "../types";

defineProps<{
  template: TemplateInfo;
}>();

const emit = defineEmits<{
  click: [];
}>();
</script>

<template>
  <button class="overlay-card" type="button" @click="emit('click')">
    <div class="overlay-preview">
      <div class="preview-lowerthird">
        <div class="lt-title">{{ template.name }}</div>
        <div class="lt-sub">{{ template.fields.length }} campo{{ template.fields.length === 1 ? '' : 's' }}</div>
      </div>
    </div>
    <div class="overlay-card-body">
      <div class="overlay-card-name">{{ template.name }}</div>
    </div>
  </button>
</template>

<style scoped>
.overlay-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  overflow: hidden;
  cursor: pointer;
  box-shadow: var(--shadow-card);
  transition: box-shadow 0.15s ease, transform 0.15s ease, border-color 0.15s ease;
  text-align: left;
  font-family: inherit;
}

.overlay-card:hover {
  box-shadow: var(--shadow-card-hover);
  transform: translateY(-2px);
  border-color: var(--border-strong);
}

.overlay-preview {
  height: 118px;
  background: #1C1C22
    url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='40' height='40'%3E%3Crect width='40' height='40' fill='none'/%3E%3Cpath d='M0 40L40 0' stroke='%232A2A32' stroke-width='1'/%3E%3C/svg%3E");
  position: relative;
  display: flex;
  align-items: flex-end;
  padding: 14px;
}

.preview-lowerthird {
  background: rgba(20, 20, 24, 0.72);
  backdrop-filter: blur(2px);
  border-left: 3px solid var(--accent);
  padding: 6px 10px;
  border-radius: 3px;
}

.lt-title {
  color: #fff;
  font-size: 11px;
  font-weight: 650;
  line-height: 1.3;
}

.lt-sub {
  color: #C9C9D1;
  font-size: 9.5px;
  margin-top: 1px;
}

.overlay-card-body {
  padding: 12px 14px 14px;
}

.overlay-card-name {
  font-size: 13.5px;
  font-weight: 600;
}
</style>
