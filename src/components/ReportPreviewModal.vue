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

watch(
  () => props.show,
  async (show) => {
    if (!show) {
      report.markdown = "";
      report.reportData = null;
      weekId.value = null;
      return;
    }
    if (!plan.currentWeek) {
      await plan.loadCurrentWeek();
    }
    if (plan.currentWeek?.week) {
      weekId.value = plan.currentWeek.week.id;
      await report.loadReport(weekId.value);
    }
  }
);

async function onCarryOverConfirm() {
  if (!weekId.value) return;
  await report.confirmAndRender(weekId.value);
}

function close() {
  emit("update:show", false);
}
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    title="周报预览"
    style="width: 560px; max-width: 90vw"
    :mask-closable="false"
    :closable="false"
  >
    <div class="report-modal">
      <NSpin v-if="report.loading" style="padding: 40px 0">
        正在统计数据...
      </NSpin>

      <NEmpty
        v-else-if="!weekId && !report.loading"
        description="暂无本周记录，请先填写本周计划"
      />

      <div v-else-if="report.markdown" class="report-content">
        <pre class="report-markdown">{{ report.markdown }}</pre>
      </div>

      <NEmpty
        v-else
        description="点击生成周报"
      />
    </div>

    <template #footer>
      <NSpace justify="end">
        <NButton :disabled="!report.markdown" @click="report.copyToClipboard()">
          复制到剪贴板
        </NButton>
        <NButton :disabled="!report.markdown" @click="report.saveToFile()">
          保存为 .md
        </NButton>
        <NButton @click="close">关闭</NButton>
      </NSpace>
    </template>

    <CarryOverModal @confirm="onCarryOverConfirm" />
  </NModal>
</template>

<style scoped>
.report-modal {
  max-height: 60vh;
  overflow-y: auto;
}

.report-content {
  background: #f9fafb;
  border-radius: 12px;
  padding: 16px;
}

.report-markdown {
  font-family: monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  color: #374151;
}
</style>
