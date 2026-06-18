<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useTimerStore } from "@/stores/timer";
import { useReportStore } from "@/stores/report";
import { useClockStore } from "@/stores/clock";
import CountdownRing from "@/components/CountdownRing.vue";
import TaskPickerModal from "@/components/TaskPickerModal.vue";
import {
  NButton,
  NSpace,
  NButtonGroup,
  NAlert,
  NTag,
  NDatePicker,
  NCard,
} from "naive-ui";

const timer = useTimerStore();
const report = useReportStore();
const clock = useClockStore();

const isDev = import.meta.env.DEV;

// ---- dev 模拟时间面板 ----
// NDatePicker(datetime) 的值是时间戳(ms)；null 表示未选
const pickerValue = ref<number | null>(
  clock.mockNow ? new Date(clock.mockNow).getTime() : null
);

const mockLabel = computed(() =>
  clock.mockNow ? `模拟时间：${clock.mockNow}` : "使用真实系统时间"
);

/** ms 时间戳 → 后端期望的 YYYY-MM-DDTHH:MM */
function toIso(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}

/** 生成"本周某天某时"的 ISO 字符串（基于真实今天） */
function isoThisWeek(weekday: number, hour: number, minute = 0): string {
  const now = new Date();
  const cur = now.getDay() === 0 ? 7 : now.getDay(); // 周日→7
  const diff = weekday - cur;
  const d = new Date(now);
  d.setDate(now.getDate() + diff);
  d.setHours(hour, minute, 0, 0);
  return toIso(d.getTime());
}

async function applyMock() {
  if (pickerValue.value == null) return;
  const iso = toIso(pickerValue.value);
  await clock.set(iso);
  await report.checkReminder();
}

async function quickSet(weekday: number, hour: number, minute = 0) {
  const iso = isoThisWeek(weekday, hour, minute);
  pickerValue.value = new Date(iso).getTime();
  await clock.set(iso);
  await report.checkReminder();
}

async function clearMock() {
  pickerValue.value = null;
  await clock.clear();
  await report.checkReminder();
}

onMounted(() => {
  report.checkReminder();
});
</script>

<template>
  <div class="timer-view">
    <!-- 周二未填计划提醒 banner -->
    <NAlert v-if="report.needsReminder" type="warning" style="margin-bottom: 16px" closable>
      还未填写本周计划！请前往「本周计划」页面填写后开始番茄钟。
    </NAlert>

    <!-- dev 模拟时间面板（仅 dev 构建，生产自动隐藏） -->
    <NCard
      v-if="isDev"
      size="small"
      title="模拟时间（仅 dev）"
      style="margin-bottom: 16px"
      :bordered="true"
    >
      <NSpace align="center" wrap>
        <NDatePicker
          v-model:value="pickerValue"
          type="datetime"
          clearable
          placeholder="选择模拟时间点"
          style="width: 220px"
        />
        <NButton size="small" type="primary" :disabled="pickerValue == null" @click="applyMock">
          应用
        </NButton>
        <NButton size="small" tertiary @click="clearMock">清除（恢复真实时间）</NButton>
        <NTag size="small" :type="clock.mockNow ? 'warning' : 'default'">
          {{ mockLabel }}
        </NTag>
      </NSpace>
      <NSpace style="margin-top: 8px" size="small">
        <NButton size="tiny" tertiary @click="quickSet(2, 12, 0)">周二 12:00（触发提醒）</NButton>
        <NButton size="tiny" tertiary @click="quickSet(5, 17, 0)">周五 17:00（生成周报）</NButton>
        <NButton size="tiny" tertiary @click="quickSet(1, 9, 0)">周一 09:00（归上周）</NButton>
        <NButton size="tiny" tertiary @click="quickSet(3, 10, 0)">周三 10:00</NButton>
      </NSpace>
    </NCard>

    <div class="timer-center">
      <!-- 预设切换 -->
      <div class="preset-row">
        <NButtonGroup size="small">
          <NButton
            v-for="(p, i) in timer.presets"
            :key="i"
            :type="timer.presetIndex === i ? 'primary' : 'default'"
            :tertiary="timer.presetIndex !== i"
            @click="timer.presetIndex = i"
            :disabled="timer.phase !== 'idle'"
          >
            {{ p.label }}
          </NButton>
        </NButtonGroup>
        <span class="preset-hint">
          {{ timer.preset.focusMin }}分钟专注 / {{ timer.preset.breakMin }}分钟休息
        </span>
      </div>

      <!-- 倒计时圆环 -->
      <CountdownRing
        :progress="timer.progress"
        :display="timer.minutesDisplay"
        :phase="timer.phase"
      />

      <!-- 控制按钮 -->
      <div class="control-row">
        <template v-if="timer.phase === 'idle'">
          <NButton type="primary" size="large" @click="timer.startFocus()">
            开始专注
          </NButton>
        </template>

        <template v-else-if="timer.phase === 'focus'">
          <NSpace>
            <NButton @click="timer.phase === 'focus' ? timer.pause() : timer.resume()">
              暂停
            </NButton>
            <NButton type="warning" @click="timer.reset()">
              结束
            </NButton>
          </NSpace>
        </template>

        <template v-else-if="timer.phase === 'break'">
          <NSpace>
            <NButton type="info" @click="timer.reset()">
              跳过休息
            </NButton>
          </NSpace>
        </template>
      </div>

      <!-- 上一个任务提示 -->
      <div v-if="timer.lastTaskId != null" class="last-task-hint">
        上一个时段：
        <NTag size="small" type="info">
          {{ timer.lastTaskSource === "planned" ? "计划内" : "计划外" }}
        </NTag>
      </div>
    </div>

    <!-- 选任务弹窗 -->
    <TaskPickerModal @confirm="() => {}" />
  </div>
</template>

<style scoped>
.timer-view {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.timer-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 24px;
}

.preset-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.preset-hint {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
}

.control-row {
  min-height: 48px;
  display: flex;
  align-items: center;
}

.last-task-hint {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.4);
  display: flex;
  align-items: center;
  gap: 6px;
}
</style>
