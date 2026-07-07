<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTimerStore } from "@/stores/timer";
import { useReportStore } from "@/stores/report";
import { usePlanStore } from "@/stores/plan";
import { useClockStore } from "@/stores/clock";
import CountdownRing from "@/components/CountdownRing.vue";
import TaskPickerModal from "@/components/TaskPickerModal.vue";
import PlanInputModal from "@/components/PlanInputModal.vue";
import ReportPreviewModal from "@/components/ReportPreviewModal.vue";
import SettingsModal from "@/components/SettingsModal.vue";
import {
  NButton,
  NSpace,
  NButtonGroup,
  NTag,
  NDatePicker,
  NCard,
  NAlert,
  NIcon,
  useDialog,
  useMessage,
} from "naive-ui";
import {
  PlayOutline,
  StopOutline,
  RefreshOutline,
} from "@vicons/ionicons5";

const timer = useTimerStore();
const report = useReportStore();
const plan = usePlanStore();
const clock = useClockStore();

const isDev = import.meta.env.DEV;
/** dev 模式下也可隐藏「模拟时间」面板（用于验证生产界面）。环境变量 VITE_HIDE_DEV_PANEL=1 时隐藏。 */
const showDevPanel = isDev && !import.meta.env.VITE_HIDE_DEV_PANEL;

const showPlanModal = ref(false);
const showReportModal = ref(false);
const showSettingsModal = ref(false);

const pickerValue = ref<number | null>(
  clock.mockNow ? new Date(clock.mockNow).getTime() : null
);

const mockLabel = computed(() =>
  clock.mockNow ? `模拟时间：${clock.mockNow}` : "使用真实系统时间"
);

function toIso(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
    d.getHours()
  )}:${pad(d.getMinutes())}`;
}

function isoThisWeek(weekday: number, hour: number, minute = 0): string {
  const now = new Date();
  const cur = now.getDay() === 0 ? 7 : now.getDay();
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

const dialog = useDialog();
const message = useMessage();

/** 【dev】清空本周计划：确认后删除本周全部任务及番茄钟记录 */
function confirmClearPlan() {
  dialog.warning({
    title: "清空本周计划",
    content: "将清空本周全部计划内/计划外任务及番茄钟记录，不可恢复，确认？",
    positiveText: "确认清空",
    negativeText: "取消",
    onPositiveClick: async () => {
      await plan.clearWeekDataAll();
      message.success("已清空");
    },
  });
}

const weekInfo = computed(() => {
  const now = clock.nowDate();
  const start = new Date(now);
  const day = start.getDay() === 0 ? 7 : start.getDay();
  start.setDate(start.getDate() - day + 2);
  const end = new Date(start);
  end.setDate(start.getDate() + 6);
  const format = (d: Date) => `${d.getMonth() + 1}/${d.getDate()}`;
  return {
    weekNum: getWeekNumber(now),
    year: now.getFullYear(),
    range: `${format(start)} ~ ${format(end)}`,
  };
});

function getWeekNumber(d: Date) {
  const date = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  const dayNum = date.getUTCDay() || 7;
  date.setUTCDate(date.getUTCDate() + 4 - dayNum);
  const yearStart = new Date(Date.UTC(date.getUTCFullYear(), 0, 1));
  return Math.ceil(((+date - +yearStart) / 86400000 + 1) / 7);
}

const totalTimeText = computed(() => {
  const totalMin = timer.total ? Math.round(timer.total / 60) : 0;
  const h = Math.floor(totalMin / 60);
  const m = totalMin % 60;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
});

const completedCount = computed(() => {
  return report.reportData?.tasks?.filter((t) => t.status === "done").length ?? 0;
});

function openPlan() {
  showPlanModal.value = true;
}

function openReport() {
  showReportModal.value = true;
}

function openSettings() {
  showSettingsModal.value = true;
}

/** 点击倒计时圆环：按当前阶段触发与控制按钮一致的动作（常态专用；
 *  浮球态由 onMiniMouseDown 处理拖动/双击恢复，不走此 @click） */
function onRingClick() {
  if (timer.miniMode) return; // 浮球态兜底，防误结束专注
  if (timer.phase === "idle") timer.startFocus();
  else if (timer.phase === "focus") timer.manualEnd();
  else if (timer.phase === "break") timer.reset();
}

/** 浮球交互：左键按下即拖动移窗（macOS 需在 mousedown 同步触发 startDragging）；
 *  双击（detail≥2）恢复主界面、专注继续。
 *  不用 data-tauri-drag-region：它会把双击劫持为窗口最大化。 */
function onMiniMouseDown(e: MouseEvent) {
  if (!timer.miniMode || e.button !== 0) return;
  e.preventDefault(); // 防文字选中
  if (e.detail >= 2) {
    timer.exitMini(); // 双击恢复主界面
    return;
  }
  void getCurrentWindow().startDragging(); // 单击按下即拖动
}

onMounted(() => {
  report.checkReminder();
  plan.loadCurrentWeek();
});

</script>

<template>
  <div class="timer-view" :class="{ mini: timer.miniMode }">
    <!-- dev 模拟时间面板（仅 dev 构建且未设置 VITE_HIDE_DEV_PANEL 时显示，生产自动隐藏） -->
    <NCard
      v-if="showDevPanel"
      v-show="!timer.miniMode"
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
      <NSpace style="margin-top: 8px" size="small">
        <NButton
          size="tiny"
          type="warning"
          tertiary
          :disabled="timer.phase === 'idle' || timer.remaining <= 5"
          @click="timer.fastForwardToEnd()"
        >
          快进到结束前5秒
        </NButton>
        <NButton size="tiny" type="error" tertiary @click="confirmClearPlan">
          清空本周计划（含番茄钟记录）
        </NButton>
      </NSpace>
    </NCard>

    <!-- 本周未录入计划提醒（周二 12:00 后且本周无 planned_tasks） -->
    <NAlert
      v-if="report.needsReminder"
      v-show="!timer.miniMode"
      type="warning"
      :bordered="false"
      class="plan-reminder"
      @click="openPlan"
    >
      📅 本周尚未录入计划，点击填写
    </NAlert>

    <div class="main-card">
      <div
        class="ring-wrapper"
        :class="{ mini: timer.miniMode }"
        @click="onRingClick"
        @mousedown="onMiniMouseDown"
      >
        <CountdownRing
          :progress="timer.phase === 'idle' ? 1 : timer.progress"
          :display="timer.minutesDisplay"
          :phase="timer.phase"
          :radius="timer.miniMode ? 50 : 110"
          :mini="timer.miniMode"
        />
      </div>

      <!-- 预设切换 -->
      <div class="preset-row" v-show="!timer.miniMode">
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
      </div>

      <!-- 控制按钮 -->
      <div class="control-row" v-show="!timer.miniMode">
        <template v-if="timer.phase === 'idle'">
          <NButton type="primary" size="large" round @click="timer.startFocus()">
            <template #icon>
              <NIcon><PlayOutline /></NIcon>
            </template>
            开始
          </NButton>
        </template>

        <template v-else-if="timer.phase === 'focus'">
          <NButton type="warning" round @click="timer.manualEnd()">
            <template #icon>
              <NIcon><StopOutline /></NIcon>
            </template>
            结束
          </NButton>
          <NButton tertiary round @click="timer.enterMini()">收起为浮球</NButton>
        </template>

        <template v-else-if="timer.phase === 'break'">
          <NSpace>
            <NButton type="info" round @click="timer.reset()">
              <template #icon>
                <NIcon><RefreshOutline /></NIcon>
              </template>
              跳过休息
            </NButton>
          </NSpace>
        </template>
      </div>

      <!-- 操作按钮 -->
      <div class="action-buttons" v-show="!timer.miniMode">
        <button class="action-btn" :class="{ active: showPlanModal }" @click="openPlan">
          <span class="action-icon">📅</span>
          <span class="action-label">周计划</span>
        </button>
        <button class="action-btn" :class="{ active: showReportModal }" @click="openReport">
          <span class="action-icon">📝</span>
          <span class="action-label">生成周报</span>
        </button>
        <button class="action-btn" :class="{ active: showSettingsModal }" @click="openSettings">
          <span class="action-icon">⚙️</span>
          <span class="action-label">设置</span>
        </button>
      </div>

      <!-- 底部统计 -->
      <div class="stats-bar" v-show="!timer.miniMode">
        <div class="stat-item">
          <span class="stat-label">第 {{ weekInfo.weekNum }} 周, {{ weekInfo.year }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ completedCount }}</span>
          <span class="stat-label">已完成</span>
        </div>
        <div class="stat-item">
          <span class="stat-value">{{ totalTimeText }}</span>
          <span class="stat-label">总用时</span>
        </div>
      </div>
    </div>

    <!-- 弹窗 -->
    <TaskPickerModal @confirm="() => {}" />
    <PlanInputModal v-model:show="showPlanModal" />
    <ReportPreviewModal v-model:show="showReportModal" />
    <SettingsModal v-model:show="showSettingsModal" />
  </div>
</template>

<style scoped>
.timer-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-start;
  padding: 20px;
  overflow-y: auto;
  background: #f5f7fa;
}

.plan-reminder {
  width: 100%;
  max-width: 420px;
  margin-bottom: 16px;
  cursor: pointer;
}

.main-card {
  width: 100%;
  max-width: 420px;
  flex: 1;
  background: #ffffff;
  border-radius: 24px;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.06);
  padding: 32px 24px 20px;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.ring-wrapper {
  margin-bottom: 20px;
}

/* ---- 浮球模式：专注中收起为仅圆环的悬浮小球 ---- */
.timer-view.mini {
  background: transparent;
  padding: 0;
  justify-content: center;
  overflow: hidden;
}

.timer-view.mini .main-card {
  background: transparent;
  box-shadow: none;
  padding: 0;
  border-radius: 0;
  max-width: none;
  width: 100%;
  flex: 1;
  justify-content: center;
}

/* 浮球态：圆环容器脱离文档流、铺满窗口并居中——无论窗口实际缩到多大，
   圆环都恒定居中可见，且不会因常态布局残留产生滚动条。
   data-tauri-drag-region="deep" 使整个球可拖动（macOS 下由 Tauri 在 mousedown
   同步触发 start_dragging，可靠移窗）。圆环外全透明露出桌面。 */
.ring-wrapper.mini {
  position: fixed;
  inset: 0;
  margin: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: grab;
}

.ring-wrapper.mini:active {
  cursor: grabbing;
}

/* drop-shadow 跟随环形描边，在透明背景下提供层次感 */
.ring-wrapper.mini :deep(.ring-svg) {
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.25));
}

.ring-wrapper.mini :deep(.countdown-ring) {
  cursor: grab;
}

.preset-row {
  margin-bottom: 20px;
}

.control-row {
  min-height: 48px;
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 28px;
}

.action-buttons {
  display: flex;
  gap: 12px;
  margin-bottom: 24px;
}

.action-btn {
  width: 88px;
  height: 72px;
  border-radius: 16px;
  border: 1px solid #e5e7eb;
  background: #ffffff;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  cursor: pointer;
  transition: all 0.2s ease;
  color: #374151;
}

.action-btn:hover {
  background: #f9fafb;
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.action-btn.active {
  background: #3b82f6;
  border-color: #3b82f6;
  color: #ffffff;
}

.action-btn.active:hover {
  background: #2563eb;
}

.action-btn:disabled,
.action-btn[disabled] {
  opacity: 0.5;
  cursor: not-allowed;
  background: #f3f4f6;
  border-color: #e5e7eb;
  color: #9ca3af;
}

.action-btn:disabled:hover {
  transform: none;
  box-shadow: none;
  background: #f3f4f6;
}

.action-icon {
  font-size: 22px;
  line-height: 1;
}

.action-label {
  font-size: 13px;
  font-weight: 500;
}

.stats-bar {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 16px;
  background: #f9fafb;
  border-radius: 14px;
  font-size: 13px;
  color: #6b7280;
}

.stat-item {
  display: flex;
  align-items: center;
  gap: 4px;
}

.stat-value {
  font-weight: 700;
  color: #111827;
}
</style>
