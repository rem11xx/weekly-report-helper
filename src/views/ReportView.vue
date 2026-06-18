<script setup lang="ts">
import { ref, onMounted } from "vue";
import { usePlanStore } from "@/stores/plan";
import { useReportStore } from "@/stores/report";
import CarryOverModal from "@/components/CarryOverModal.vue";
import {
  NButton,
  NSpin,
  NEmpty,
  NCard,
} from "naive-ui";

const plan = usePlanStore();
const report = useReportStore();

const weekId = ref<number | null>(null);

onMounted(async () => {
  // 先获取当前周
  if (!plan.currentWeek) {
    await plan.loadCurrentWeek();
  }
  if (plan.currentWeek?.week) {
    weekId.value = plan.currentWeek.week.id;
  }
});

/** 手动加载周报 */
async function loadReport() {
  if (!weekId.value) {
    // 如果当前周不存在，先加载
    await plan.loadCurrentWeek();
    if (plan.currentWeek?.week) {
      weekId.value = plan.currentWeek.week.id;
    } else {
      return;
    }
  }
  await report.loadReport(weekId.value);
}

/** 勾选顺延确认 → 渲染 Markdown */
async function onCarryOverConfirm() {
  if (!weekId.value) return;
  await report.confirmAndRender(weekId.value);
}
</script>

<template>
  <div class="report-view">
    <h2 class="page-title">本周周报</h2>

    <!-- 当前周信息 -->
    <div v-if="plan.currentWeek?.week" class="week-info">
      {{ plan.currentWeek.week.week_start }} ~ {{ plan.currentWeek.week.week_end }}
    </div>

    <!-- 加载按钮 -->
    <div v-if="!report.markdown && !report.loading" class="action-row">
      <NButton type="primary" @click="loadReport">
        生成周报
      </NButton>
    </div>

    <!-- 加载中 -->
    <NSpin v-if="report.loading" style="margin-top: 40px">
      正在统计数据...
    </NSpin>

    <!-- 无周报提示 -->
    <NEmpty
      v-if="!weekId && !report.loading"
      description="暂无本周记录，请先填写本周计划"
      style="margin-top: 40px"
    />

    <!-- CarryOver 弹窗 -->
    <CarryOverModal @confirm="onCarryOverConfirm" />

    <!-- Markdown 预览 -->
    <div v-if="report.markdown" class="report-section">
      <div class="report-actions">
        <NButton @click="report.copyToClipboard()">复制到剪贴板</NButton>
        <NButton @click="report.saveToFile()">保存为 .md</NButton>
      </div>
      <NCard class="report-card">
        <pre class="report-markdown">{{ report.markdown }}</pre>
      </NCard>
    </div>
  </div>
</template>

<style scoped>
.report-view {
  max-width: 800px;
  margin: 0 auto;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin-bottom: 12px;
}

.week-info {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.5);
  margin-bottom: 16px;
}

.action-row {
  margin-top: 20px;
  display: flex;
  gap: 12px;
}

.report-section {
  margin-top: 24px;
}

.report-actions {
  display: flex;
  gap: 8px;
  margin-bottom: 12px;
}

.report-card {
  background: rgba(128, 128, 128, 0.05);
}

.report-markdown {
  font-family: monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}
</style>
