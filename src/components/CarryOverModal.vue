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
      <div v-if="unfinishedTasks.length > 0" class="guide">
        <p class="guide-lead">本周尚未完成的任务如下，请逐个确认其去向：</p>
        <div class="legend">
          <div class="legend-item">
            <span class="legend-box checked" aria-hidden="true">
              <svg viewBox="0 0 16 16" width="11" height="11">
                <path
                  d="M3 8l3 3 7-7"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.4"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                />
              </svg>
            </span>
            <span class="legend-text">
              <b>勾选</b> · 计划下周一完成 —— 按预估工时计入本周周报，不再顺延到下周。
            </span>
          </div>
          <div class="legend-item">
            <span class="legend-box" aria-hidden="true"></span>
            <span class="legend-text">
              <b>不勾选</b> · 顺延到下周 —— 本周按已耗番茄钟记录（未开始不记用时），并自动加入下周计划。
            </span>
          </div>
        </div>
        <p class="guide-foot">
          列表中「进行中 / 未开始」为当前状态，旁附已耗工时；点「确认」保存勾选并生成本周周报。
        </p>
      </div>

      <p v-else class="empty-tip">
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

.guide {
  background: rgba(59, 130, 246, 0.08);
  border: 1px solid rgba(59, 130, 246, 0.22);
  border-radius: 6px;
  padding: 10px 12px;
  margin-bottom: 16px;
}

.guide-lead {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.78);
  margin-bottom: 8px;
}

.legend {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.legend-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  font-size: 12px;
  line-height: 1.5;
  color: rgba(255, 255, 255, 0.62);
}

.legend-text b {
  color: rgba(255, 255, 255, 0.92);
  font-weight: 600;
}

.legend-box {
  flex: 0 0 14px;
  width: 14px;
  height: 14px;
  margin-top: 2px;
  border-radius: 3px;
  border: 1.5px solid rgba(255, 255, 255, 0.4);
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.legend-box.checked {
  background: #3b82f6;
  border-color: #3b82f6;
  color: #fff;
}

.guide-foot {
  margin-top: 10px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.45);
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
