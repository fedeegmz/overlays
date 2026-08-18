<script setup lang="ts">
import Sidebar from "./Sidebar.vue";

defineProps<{
  currentPage: "overlays" | "settings";
}>();

const emit = defineEmits<{
  navigate: [page: "overlays" | "settings"];
  openDetail: [];
}>();
</script>

<template>
  <div class="app-shell">
    <Sidebar
      :current-page="currentPage"
      @navigate="(p) => emit('navigate', p)"
      @open-detail="emit('openDetail')"
    />
    <main class="content">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  width: 100%;
  height: 100vh;
}

.content {
  flex: 1;
  overflow-y: auto;
  padding: 34px 44px 60px;
}
</style>
