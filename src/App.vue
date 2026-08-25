<script setup lang="ts">
import { ref, watch } from "vue";
import { bootstrapStores } from "./bootstrap";
import AppShell from "./components/AppShell.vue";
import OverlayDetailPage from "./components/OverlayDetailPage.vue";
import OverlaysPage from "./components/OverlaysPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
import { useInstanceStore } from "./stores/instances";

bootstrapStores();

const { instances } = useInstanceStore();

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
