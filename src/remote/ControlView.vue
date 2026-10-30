<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { commandErrorMessage } from "../lib/errors";
import InstanceCard from "./InstanceCard.vue";
import type { RemoteSession } from "./session";

defineProps<{ session: RemoteSession }>();

const { t } = useI18n();
</script>

<template>
  <p
    class="connection"
    :class="{ offline: !session.online.value }"
    role="status"
  >
    {{ session.online.value ? t("remote.connection.online") : t("remote.connection.offline") }}
  </p>
  <p v-if="session.actionError.value" class="error" role="alert">
    {{ commandErrorMessage(session.actionError.value) }}
  </p>

  <p v-if="session.isEmpty.value" class="empty">{{ t("remote.empty") }}</p>
  <InstanceCard
    v-for="instance in session.instances.value"
    :key="instance.id"
    :instance="instance"
    :fields="session.fieldsOf(instance)"
    :presets="session.presetsFor(instance.templateId)"
    @show="session.show(instance.id)"
    @hide="session.hide(instance.id)"
    @edit="(key, value) => session.setField(instance.id, key, value)"
    @preset="(name) => session.applyPreset(instance.id, name)"
  />

  <section class="groups">
    <h2 class="groups-title">{{ t("remote.groups.title") }}</h2>
    <p v-if="session.groups.value.length === 0" class="hint">
      {{ t("remote.groups.none") }}
    </p>
    <button
      v-for="group in session.groups.value"
      :key="group.name"
      class="btn group-btn"
      type="button"
      @click="session.startGroup(group.name)"
    >
      {{ t("remote.groups.start") }}: {{ group.name }}
    </button>
    <div v-if="session.notice.value" class="notice" role="status">
      <p>
        {{ t("remote.groups.started", { name: session.notice.value.name }) }}
      </p>
      <p v-if="session.notice.value.conflicts > 0">
        {{ t("remote.groups.conflicts", { count: session.notice.value.conflicts }) }}
      </p>
      <p v-if="session.notice.value.missing > 0">
        {{ t("remote.groups.missing", { count: session.notice.value.missing }) }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.connection {
  font-size: 12px;
  font-weight: 600;
  color: var(--success);
  margin-bottom: 10px;
}
.connection.offline {
  color: var(--text-secondary);
}
.error {
  font-size: 13px;
  color: var(--danger);
  background: var(--danger-soft);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  margin-bottom: 10px;
}
.empty {
  text-align: center;
  color: var(--text-secondary);
  padding: 32px 8px;
}
.groups {
  margin-top: 20px;
}
.groups-title {
  font-size: 14px;
  margin-bottom: 8px;
}
.hint {
  font-size: 13px;
  color: var(--text-secondary);
}
.group-btn {
  display: block;
  width: 100%;
  margin-bottom: 8px;
  padding: 11px;
  text-align: left;
}
.notice {
  font-size: 13px;
  background: var(--accent-soft);
  border: 1px solid var(--accent-soft-border);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
  margin-top: 8px;
}
</style>
