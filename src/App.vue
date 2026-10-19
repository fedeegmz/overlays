<script setup lang="ts">
import { storeToRefs } from "pinia";
import { onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { bootstrapStores } from "./bootstrap";
import AppShell from "./components/AppShell.vue";
import ChecklistsPage from "./components/ChecklistsPage.vue";
import OverlayDetailPage from "./components/OverlayDetailPage.vue";
import OverlaysPage from "./components/OverlaysPage.vue";
import SettingsPage from "./components/SettingsPage.vue";
import { onCloseRequested } from "./services/appApi";
import { confirmWarning } from "./services/dialogApi";
import { useInstanceStore } from "./stores/instances";
import type { NavPage, Page } from "./types";

bootstrapStores();

const { t } = useI18n();
const { instances } = storeToRefs(useInstanceStore());

const unlistenClose = onCloseRequested(async () => {
  const liveCount = instances.value.filter((i) => i.visible).length;
  if (liveCount === 0) return true;
  return confirmWarning({
    title: t("dialog.confirmClose.title"),
    message: t("dialog.confirmClose.message", liveCount),
    okLabel: t("dialog.confirmClose.ok"),
    cancelLabel: t("dialog.confirmClose.cancel"),
  });
});

onUnmounted(() => {
  void unlistenClose.then((unlisten) => unlisten());
});

const currentPage = ref<Page>("overlays");

function navigate(page: NavPage) {
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
    <ChecklistsPage v-else-if="currentPage === 'checklists'" />
    <SettingsPage v-else-if="currentPage === 'settings'" />
  </AppShell>
</template>
