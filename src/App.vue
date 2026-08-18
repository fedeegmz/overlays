<script setup lang="ts">
import { ref, watch } from "vue";
import { initOverlayControl, useOverlayControl } from "./composables/useOverlayControl";
import AppShell from "./components/AppShell.vue";
import OverlaysPage from "./components/OverlaysPage.vue";
import OverlayDetailPage from "./components/OverlayDetailPage.vue";
import SettingsPage from "./components/SettingsPage.vue";

initOverlayControl();

const { instances } = useOverlayControl();

type Page = "overlays" | "detail" | "settings";

const currentPage = ref<Page>("overlays");

function navigate(page: "overlays" | "settings") {
  currentPage.value = page;
}

function openDetail() {
  currentPage.value = "detail";
}

function backToOverlays() {
  currentPage.value = "overlays";
}

watch(instances, (list) => {
  if (list.length === 0 && currentPage.value === "detail") {
    currentPage.value = "overlays";
  }
});
</script>

<template>
  <AppShell
    :current-page="currentPage === 'detail' ? 'overlays' : currentPage"
    @navigate="navigate"
    @open-detail="openDetail"
  >
    <OverlaysPage v-if="currentPage === 'overlays'" @open-detail="openDetail" />
    <OverlayDetailPage
      v-else-if="currentPage === 'detail'"
      @back="backToOverlays"
    />
    <SettingsPage v-else-if="currentPage === 'settings'" />
  </AppShell>
</template>
