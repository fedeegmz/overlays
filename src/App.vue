<script setup lang="ts">
import { ref, watch } from "vue";
import { bootstrapStores } from "./bootstrap";
import AppShell from "./components/AppShell.vue";
import GeneratePage from "./components/GeneratePage.vue";
import OverlayDetailPage from "./components/OverlayDetailPage.vue";
import OverlaysPage from "./components/OverlaysPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
import { useConfigStore } from "./stores/config";
import { useInstanceStore } from "./stores/instances";

bootstrapStores();

const { instances } = useInstanceStore();
const configStore = useConfigStore();

type Page = "overlays" | "detail" | "settings" | "generate";

const currentPage = ref<Page>("overlays");

function navigate(page: "overlays" | "settings" | "generate") {
  // The generator page is only reachable when the feature flag is on
  // (nav entry and G1 button are hidden too — this is the last line).
  if (
    page === "generate" &&
    configStore.appConfig.ai_generator_enabled === false
  ) {
    return;
  }
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
    <OverlaysPage
      v-if="currentPage === 'overlays'"
      @open-detail="openDetail"
      @navigate="navigate"
    />
    <OverlayDetailPage
      v-else-if="currentPage === 'detail'"
      @back="backToOverlays"
    />
    <SettingsPage v-else-if="currentPage === 'settings'" />
    <GeneratePage v-else-if="currentPage === 'generate'" @navigate="navigate" />
  </AppShell>
</template>
