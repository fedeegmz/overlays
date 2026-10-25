<script setup lang="ts">
import { storeToRefs } from "pinia";
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useConfirmClick } from "../composables/useConfirmClick";
import { commandErrorMessage } from "../lib/errors";
import { useGroupPresetStore } from "../stores/groupPresets";
import { useInstanceStore } from "../stores/instances";
import type { GroupPreset, GroupStartResult } from "../types";

const { t } = useI18n();
const groupStore = useGroupPresetStore();
const instanceStore = useInstanceStore();
const { groups, groupsError } = storeToRefs(groupStore);
const { instances } = storeToRefs(instanceStore);

const groupName = ref("");
const busy = ref(false);
const actionError = ref<string | null>(null);
const startReport = ref<GroupStartResult[] | null>(null);

const trimmedName = computed(() => groupName.value.trim());
const saveDisabled = computed(
  () =>
    busy.value ||
    trimmedName.value.length === 0 ||
    instances.value.length === 0,
);
const failedResults = computed(
  () => startReport.value?.filter((r) => !r.ok) ?? [],
);
const conflictResults = computed(
  () => startReport.value?.filter((r) => r.ok && r.idConflict) ?? [],
);
const startedCount = computed(
  () => (startReport.value?.length ?? 0) - failedResults.value.length,
);

onMounted(() => {
  void groupStore.refreshGroups();
});

async function doSave(name: string) {
  busy.value = true;
  actionError.value = null;
  try {
    await groupStore.saveFromInstances(name);
    groupName.value = "";
  } catch (err) {
    actionError.value = commandErrorMessage(err);
  } finally {
    busy.value = false;
  }
}

const { pending: confirmingOverwrite, confirmClick: confirmOverwrite } =
  useConfirmClick<string>((name) => void doSave(name));

function handleSave() {
  if (saveDisabled.value) return;
  if (groupStore.hasGroup(trimmedName.value)) {
    confirmOverwrite(trimmedName.value);
    return;
  }
  void doSave(trimmedName.value);
}

const { pending: confirmingDelete, confirmClick: handleDelete } =
  useConfirmClick<string>((name) => void groupStore.deleteGroup(name));

async function handleStart(group: GroupPreset) {
  busy.value = true;
  actionError.value = null;
  startReport.value = null;
  try {
    startReport.value = await groupStore.startGroup(group);
  } catch (err) {
    actionError.value = commandErrorMessage(err);
  } finally {
    busy.value = false;
  }
}

function conflictText(result: GroupStartResult): string {
  return t("groups.idConflict", { template: result.template });
}

function failureText(result: GroupStartResult): string {
  const reason =
    !result.ok && result.reason === "templateMissing"
      ? t("errors.templateMissing", { template: result.template })
      : t("groups.sendFailed");
  return t("groups.entryFailed", { template: result.template, reason });
}
</script>

<template>
  <section class="groups-section">
    <div class="groups-title">{{ t("groups.title") }}</div>
    <div class="groups-subtitle">{{ t("groups.subtitle") }}</div>

    <p v-if="groupsError" class="error">{{ groupsError }}</p>
    <div v-else-if="groups.length > 0" class="group-list">
      <div v-for="group in groups" :key="group.name" class="group-row">
        <span class="group-row-name">
          {{ group.name }}
          <span class="group-row-count">({{ group.entries.length }})</span>
        </span>
        <div class="group-row-actions">
          <button
            type="button"
            class="btn"
            :disabled="busy"
            @click="handleStart(group)"
          >
            {{ t("groups.start") }}
          </button>
          <button
            type="button"
            class="btn btn-ghost"
            :class="{ 'confirm-delete': confirmingDelete === group.name }"
            @click="handleDelete(group.name)"
          >
            {{ confirmingDelete === group.name
                ? t("groups.confirmDelete")
                : t("groups.delete") }}
          </button>
        </div>
      </div>
    </div>
    <p v-else class="muted">{{ t("groups.empty") }}</p>

    <div class="save-group-row">
      <input
        v-model="groupName"
        class="field-input"
        type="text"
        :placeholder="t('groups.namePlaceholder')"
        :disabled="busy"
        @keydown.enter="handleSave"
      >
      <button
        type="button"
        class="btn"
        :class="{ 'confirm-delete': confirmingOverwrite === trimmedName }"
        :disabled="saveDisabled"
        @click="handleSave"
      >
        {{ confirmingOverwrite === trimmedName
            ? t("groups.confirmOverwrite", { name: trimmedName })
            : t("groups.saveCurrent") }}
      </button>
    </div>
    <p v-if="instances.length === 0" class="muted">
      {{ t("groups.noInstances") }}
    </p>

    <p v-if="actionError" class="error">{{ actionError }}</p>

    <div v-if="startReport" class="start-report">
      <p class="muted">
        {{ failedResults.length === 0
            ? t("groups.startDone", { count: startedCount })
            : t("groups.startPartial", {
                ok: startedCount,
                total: startReport.length,
              }) }}
      </p>
      <p v-for="(result, i) in failedResults" :key="i" class="error">
        {{ failureText(result) }}
      </p>
      <p
        v-for="(result, i) in conflictResults"
        :key="`conflict-${i}`"
        class="muted"
      >
        {{ conflictText(result) }}
      </p>
    </div>
  </section>
</template>

<style scoped>
.groups-section {
  margin-top: 36px;
}

.groups-title {
  font-size: 17px;
  font-weight: 650;
  letter-spacing: -0.01em;
}

.groups-subtitle {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 4px 0 14px;
}

.group-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.group-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 9px 11px;
  border-radius: var(--radius-sm);
  background: var(--surface-sunken);
  font-size: 13px;
}

.group-row-name {
  font-weight: 550;
}

.group-row-count {
  color: var(--text-faint);
  font-weight: 400;
}

.group-row-actions {
  display: flex;
  gap: 6px;
}

.confirm-delete,
.confirm-delete:hover {
  color: var(--danger);
  background: var(--danger-soft);
}

.save-group-row {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}

.save-group-row .field-input {
  flex: 1;
}

.start-report {
  margin-top: 8px;
}

.error {
  color: var(--danger);
  font-size: 13px;
}

.muted {
  color: var(--text-faint);
  font-size: 13px;
}
</style>
