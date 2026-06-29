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

  /** 收集当前勾选 plan_next_monday 的 task_id */
  function collectCheckedIds(): number[] {
    return Object.entries(nextMondayChecks.value)
      .filter(([, v]) => v)
      .map(([k]) => Number(k));
  }

  /** 写入剪贴板（Tauri 插件优先，失败回退 Web API） */
  async function writeClipboard(text: string) {
    try {
      const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
      await writeText(text);
    } catch {
      await navigator.clipboard.writeText(text);
    }
  }

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
      // 有任务即弹顺延弹窗，使「确认」在所有任务已完结时也可达
      showCarryOver.value = reportData.value.tasks.length > 0;
    } catch (e) {
      console.error("加载周报数据失败", e);
    } finally {
      loading.value = false;
    }
  }

  /** 保存顺延勾选到库（关闭前调用，需求 #3） */
  async function saveCarryOver(weekId: number) {
    try {
      await carryOverTasks({ week_id: weekId, next_monday_task_ids: collectCheckedIds() });
    } catch (e) {
      console.error("顺延保存失败", e);
    }
    showCarryOver.value = false;
  }

  /** 确认顺延 → 渲染纯文本周报并复制到剪贴板（需求 #5） */
  async function confirmAndCopy(weekId: number): Promise<boolean> {
    try {
      // 1. 保存顺延
      await carryOverTasks({ week_id: weekId, next_monday_task_ids: collectCheckedIds() });
      // 2. 重新渲染（后端已更新 plan_next_monday），并去掉 Markdown 标题标记转纯文本
      const md = await renderReportMarkdown(weekId);
      const plain = md.replace(/^#{1,6}\s+/gm, "");
      // 3. 复制到剪贴板
      await writeClipboard(plain);
      showCarryOver.value = false;
      return true;
    } catch (e) {
      console.error("生成/复制周报失败", e);
      return false;
    }
  }

  /** 确认顺延勾选并渲染最终 Markdown（ReportView 页用） */
  async function confirmAndRender(weekId: number) {
    try {
      // 调用后端顺延
      await carryOverTasks({ week_id: weekId, next_monday_task_ids: collectCheckedIds() });
      showCarryOver.value = false;

      // 重新渲染（后端已更新 plan_next_monday 状态）
      markdown.value = await renderReportMarkdown(weekId);
    } catch (e) {
      console.error("顺延失败", e);
    }
  }

  /** 复制到剪贴板（缺省用 markdown；可显式传入纯文本） */
  async function copyToClipboard(text?: string) {
    await writeClipboard(text ?? markdown.value);
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
    saveCarryOver,
    confirmAndCopy,
    confirmAndRender,
    copyToClipboard,
    saveToFile,
    checkReminder,
  };
});
