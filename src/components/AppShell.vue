<script setup lang="ts">
import type { NavPage } from "../types";
import SideNav from "./SideNav.vue";

defineProps<{
  currentPage: NavPage;
}>();

const emit = defineEmits<{
  navigate: [page: NavPage];
  openDetail: [];
}>();
</script>

<template>
  <div class="app-shell">
    <SideNav
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
