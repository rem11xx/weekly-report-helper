import { defineStore } from "pinia";
import { ref } from "vue";
import {
  getReportData,
  renderReportMarkdown,
  carryOverTasks,
  saveReportFile,
  needsPlanReminder,
} from "@/api";
import type { ReportData } from "@/types";

export const useReportStore = defineStore("report", () => {
  const reportData = ref<ReportData | null>(null);
  const markdown = ref("");
  const loading = ref(false);
  const showCarryOver = ref(false);
  const needsReminder = ref(false);

  // 周五勾选状态：task_id → 是否勾选 plan_next_monday
  const nextMondayChecks = ref<Record<number, boolean>>({});

  /** 加载周报原始数据（未勾选前） */
  async function loadReport(weekId: number) {
    loading.value = true;
    try {
      reportData.value = await getReportData(weekId);
      // 初始化勾选状态：进行中+未开始的任务默认 false
      const checks: Record<number, boolean> = {};
      for (const t of reportData.value.tasks) {
        if (t.status === "in_progress" || t.status === "not_started") {
          checks[t.task_id] = false;
        }
      }
      nextMondayChecks.value = checks;
      // 顺延任务弹窗：如果有未完成的就弹
      const unfinished = reportData.value.tasks.filter(
        (t) => t.status === "in_progress" || t.status === "not_started"
      );
      showCarryOver.value = unfinished.length > 0;
    } catch (e) {
      console.error("加载周报数据失败", e);
    } finally {
      loading.value = false;
    }
  }

  /** 确认顺延勾选并渲染最终 Markdown */
  async function confirmAndRender(weekId: number) {
    // 收集勾选 plan_next_monday 的 task_id
    const ids = Object.entries(nextMondayChecks.value)
      .filter(([, v]) => v)
      .map(([k]) => Number(k));

    try {
      // 调用后端顺延
      await carryOverTasks({ week_id: weekId, next_monday_task_ids: ids });
      showCarryOver.value = false;

      // 重新渲染（后端已更新 plan_next_monday 状态）
      markdown.value = await renderReportMarkdown(weekId);
    } catch (e) {
      console.error("顺延失败", e);
    }
  }

  /** 复制 Markdown 到剪贴板 */
  async function copyToClipboard() {
    try {
      const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
      await writeText(markdown.value);
    } catch (e) {
      // fallback
      await navigator.clipboard.writeText(markdown.value);
    }
  }

  /** 保存为 .md 文件 */
  async function saveToFile() {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const filePath = await save({
        filters: [{ name: "Markdown", extensions: ["md"] }],
        defaultPath: `周报-${reportData.value?.week?.week_start || "unknown"}.md`,
      });
      if (filePath) {
        await saveReportFile(filePath, markdown.value);
      }
    } catch (e) {
      console.error("保存文件失败", e);
    }
  }

  /** 检查是否需要提醒填计划 */
  async function checkReminder() {
    try {
      needsReminder.value = await needsPlanReminder();
    } catch (e) {
      console.error("检查提醒失败", e);
    }
  }

  return {
    reportData,
    markdown,
    loading,
    showCarryOver,
    nextMondayChecks,
    needsReminder,
    loadReport,
    confirmAndRender,
    copyToClipboard,
    saveToFile,
    checkReminder,
  };
});
