<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { createBrowserTransport } from "../services/transport";
import ControlView from "./ControlView.vue";
import PairView from "./PairView.vue";
import { createRemoteSession, type RemoteSession } from "./session";

const { t } = useI18n();

// The transport reports a rejected token through the session, which needs the
// transport to be built: the callback only runs later, after both exist.
const session: RemoteSession = createRemoteSession(
  createBrowserTransport(() => session.handleUnauthorized()),
);

onMounted(() => {
  const hash = location.hash;
  // Do not leave the single-use PIN in the address bar or history.
  if (hash) history.replaceState(null, "", location.pathname + location.search);
  void session.start(hash);
});

onUnmounted(() => session.dispose());
</script>

<template>
  <h1 class="remote-title">{{ t("remote.title") }}</h1>
  <PairView
    v-if="session.phase.value === 'pair'"
    :pairing="session.pairing.value"
    :error="session.pairError.value"
    :needs-repair="session.needsRepair.value"
    @submit="session.pairWithPin"
  />
  <ControlView v-else :session="session" />
</template>

<style scoped>
.remote-title {
  font-size: 18px;
  font-weight: 700;
  margin-bottom: 14px;
}
</style>
