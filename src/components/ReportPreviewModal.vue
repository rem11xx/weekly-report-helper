<script setup lang="ts">
import { ref, watch } from "vue";
import { usePlanStore } from "@/stores/plan";
import { useReportStore } from "@/stores/report";
import {
  NModal,
  NButton,
  NSpace,
  NSpin,
  NEmpty,
} from "naive-ui";
import CarryOverModal from "./CarryOverModal.vue";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const plan = usePlanStore();
const report = useReportStore();

const weekId = ref<number | null>(null);

type Phase = "loading" | "carryover" | "preview" | "empty";
const phase = ref<Phase>("loading");

// 确认后「已复制周报到剪贴板」提示（点击任意处或 4 秒后自动关闭）
const copiedToast = ref(false);
let toastTimer: ReturnType<typeof setTimeout> | null = null;

watch(
  () => props.show,
  async (show) => {
    if (!show) {
      reset();
      return;
    }
    phase.value = "loading";
    weekId.value = null;
    report.previewText = "";
    if (!plan.currentWeek) {
      await plan.loadCurrentWeek();
    }
    if (!plan.currentWeek?.week) {
      phase.value = "empty";
      return;
    }
    weekId.value = plan.currentWeek.week.id;
    await report.loadReport(weekId.value);
    const tasks = report.reportData?.tasks ?? [];
    if (tasks.length === 0) {
      phase.value = "empty";
    } else if (report.showCarryOver) {
      // 有未完成任务：先弹「确认任务完成情况」
      phase.value = "carryover";
    } else {
      // 全部已完成：直接进预览
      await report.renderPreview(weekId.value);
      phase.value = "preview";
    }
  }
);

function reset() {
  report.markdown = "";
  report.previewText = "";
  report.reportData = null;
  report.showCarryOver = false;
  weekId.value = null;
  phase.value = "loading";
}

/** 确认顺延 → 渲染预览 */
async function onCarryOverConfirm() {
  if (!weekId.value) return;
  await report.renderPreview(weekId.value);
  phase.value = "preview";
}

/** 顺延弹窗关闭（遮罩 / 关闭按钮）→ 先保存顺延 → 关闭 */
async function onCarryOverClose() {
  if (weekId.value) {
    await report.saveCarryOver(weekId.value);
  }
  emit("update:show", false);
}

/** 预览「确认」→ 复制到剪贴板 → 关闭 → 提示 */
async function onPreviewConfirm() {
  await report.copyPreview();
  emit("update:show", false);
  showCopiedToast();
}

/** 关闭预览（遮罩 / 关闭按钮）*/
function close() {
  emit("update:show", false);
}

function showCopiedToast() {
  copiedToast.value = true;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    copiedToast.value = false;
    toastTimer = null;
  }, 4000);
}

function dismissToast() {
  if (toastTimer) {
    clearTimeout(toastTimer);
    toastTimer = null;
  }
  copiedToast.value = false;
}
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    title="周报预览"
    style="width: 560px; max-width: 90vw"
    :mask-closable="true"
    :closable="false"
    @update:show="(v: boolean) => { if (!v) close() }"
  >
    <div class="report-modal">
      <NSpin v-if="phase === 'loading'" style="padding: 40px 0">
        正在统计数据...
      </NSpin>

      <NEmpty
        v-else-if="phase === 'empty'"
        description="暂无本周记录，请先填写本周计划"
      />

      <pre v-else-if="phase === 'preview'" class="report-preview">{{ report.previewText }}</pre>
      <!-- phase === 'carryover"：内容在 CarryOverModal（叠加于上）-->
    </div>

    <template #footer>
      <NSpace justify="end">
        <NButton @click="close">关闭</NButton>
        <NButton v-if="phase === 'preview'" type="primary" @click="onPreviewConfirm">
          确认
        </NButton>
      </NSpace>
    </template>

    <CarryOverModal @confirm="onCarryOverConfirm" @close="onCarryOverClose" />
  </NModal>

  <!-- 已复制提示：点击任意处或 4 秒后自动关闭 -->
  <div v-if="copiedToast" class="copy-toast" @click="dismissToast">
    <div class="copy-toast__card">已复制周报到剪贴板</div>
  </div>
</template>

<style scoped>
.report-modal {
  max-height: 60vh;
  overflow-y: auto;
}

.report-preview {
  font-family: monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  padding: 4px 0;
}

.copy-toast {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.28);
  cursor: pointer;
}

.copy-toast__card {
  padding: 20px 32px;
  border-radius: 14px;
  background: rgba(30, 30, 30, 0.92);
  color: #fff;
  font-size: 15px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}
</style>
