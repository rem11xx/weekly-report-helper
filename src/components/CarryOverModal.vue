<script setup lang="ts">
import { computed } from "vue";
import { useReportStore } from "@/stores/report";
import {
  NModal,
  NButton,
  NSpace,
  NCheckbox,
  NTag,
} from "naive-ui";

const store = useReportStore();

const emit = defineEmits<{
  (e: "confirm"): void;
  (e: "close"): void;
}>();

/** 未完成的任务列表（可勾选 / 不勾选） */
const unfinishedTasks = computed(() => {
  if (!store.reportData) return [];
  return store.reportData.tasks.filter(
    (t) => t.status === "in_progress" || t.status === "not_started"
  );
});

function confirm() {
  emit("confirm");
}
</script>

<template>
  <NModal
    :show="store.showCarryOver"
    preset="card"
    title="确认任务完成情况"
    style="width: 560px; max-width: 90vw"
    :mask-closable="true"
    :closable="false"
    @update:show="(v: boolean) => { if (!v) emit('close') }"
  >
    <div class="carryover-modal">
      <p class="hint">
        以下未完成任务，勾选表示「计划下周一完成」（计入本周），未勾选将顺延到下周计划。
      </p>

      <p v-if="unfinishedTasks.length === 0" class="empty-tip">
        本周任务均已完结，无需顺延。
      </p>

      <div
        v-for="t in unfinishedTasks"
        :key="t.task_id"
        class="carryover-item"
      >
        <NCheckbox
          :checked="store.nextMondayChecks[t.task_id]"
          @update:checked="(v: boolean) => (store.nextMondayChecks[t.task_id] = v)"
        >
          <div class="carryover-content">
            <span class="carryover-title">{{ t.sort_order }}.{{ t.title }}</span>
            <NTag :type="t.status === 'in_progress' ? 'warning' : 'default'" size="small">
              {{ t.status === "in_progress" ? "进行中" : "未开始" }}
            </NTag>
            <span v-if="t.actual_min > 0" class="carryover-time">
              已耗 {{ (t.actual_min / 60).toFixed(1) }}h
            </span>
          </div>
        </NCheckbox>
      </div>
    </div>

    <template #footer>
      <NSpace justify="end">
        <NButton @click="emit('close')">关闭</NButton>
        <NButton type="primary" @click="confirm">确认</NButton>
      </NSpace>
    </template>
  </NModal>
</template>

<style scoped>
.carryover-modal {
  max-height: 400px;
  overflow-y: auto;
}

.hint {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.6);
  margin-bottom: 16px;
}

.empty-tip {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.5);
  margin: 24px 0;
  text-align: center;
}

.carryover-item {
  padding: 8px 4px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.1);
}

.carryover-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.carryover-title {
  font-size: 13px;
}

.carryover-time {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
}
</style>
