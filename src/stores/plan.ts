import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { parsePlan, saveWeekPlan, getCurrentWeek, updateTask, createAdhocTask, clearWeekData } from "@/api";
import type { ParsedPlan, CurrentWeek, AdhocTask } from "@/types";

export const usePlanStore = defineStore("plan", () => {
  /** 原始输入文本 */
  const rawText = ref("");
  /** 解析预览结果（实时更新） */
  const parsed = ref<ParsedPlan>({ week_start: "", week_end: "", tasks: [], errors: [] });
  /** 当前周数据 */
  const currentWeek = ref<CurrentWeek | null>(null);
  /** 是否正在保存 */
  const saving = ref(false);
  /** 保存后的提示消息 */
  const saveMsg = ref("");

  /** 本周是否已录入计划且成功解析（有 planned 任务即视为已解析） */
  const hasPlan = computed(() => (currentWeek.value?.planned.length ?? 0) > 0);

  /** 加载当前周数据 */
  async function loadCurrentWeek() {
    try {
      currentWeek.value = await getCurrentWeek();
      if (currentWeek.value?.week?.plan_raw) {
        rawText.value = currentWeek.value.week.plan_raw;
        doParse();
      }
    } catch (e) {
      console.error("加载当前周失败", e);
    }
  }

  /** 实时解析输入文本（预览用，不落库） */
  async function doParse() {
    if (!rawText.value.trim()) {
      parsed.value = { week_start: "", week_end: "", tasks: [], errors: [] };
      return;
    }
    try {
      parsed.value = await parsePlan(rawText.value);
    } catch (e) {
      console.error("解析失败", e);
    }
  }

  /** 保存周计划（解析 + 落库） */
  async function save() {
    if (!rawText.value.trim()) return;
    saving.value = true;
    saveMsg.value = "";
    try {
      const week = await saveWeekPlan(rawText.value);
      saveMsg.value = `已保存（${week.week_start} ~ ${week.week_end}）`;
      await loadCurrentWeek();
    } catch (e) {
      saveMsg.value = "保存失败";
      console.error("保存失败", e);
    } finally {
      saving.value = false;
    }
  }

  /** 更新输入文本（外部调用） */
  function updateRaw(text: string) {
    rawText.value = text;
    doParse();
  }

  /** 落库单个任务的编辑（内容 / 序号 / 完成态）。
   *  本地状态由组件就地 mutate（不在此重排），这里只负责持久化。 */
  async function updateTaskRow(
    source: "planned" | "adhoc",
    id: number,
    title: string,
    sort_order: number,
    done: boolean
  ) {
    try {
      await updateTask({ source, id, title, sort_order, done });
    } catch (e) {
      console.error("更新任务失败", e);
    }
  }

  /** 新建计划外任务（用于表格空行）。返回新行，调用方追加到 currentWeek.adhoc。 */
  async function addAdhoc(project: string, title: string): Promise<AdhocTask | null> {
    try {
      const task = await createAdhocTask(project, title);
      currentWeek.value?.adhoc.push(task);
      return task;
    } catch (e) {
      console.error("新建计划外任务失败", e);
      return null;
    }
  }

  /** 【dev】清空本周全部数据（计划内/计划外任务 + 番茄钟记录 + plan_raw），回到无计划输入态 */
  async function clearWeekDataAll() {
    try {
      await clearWeekData();
      await loadCurrentWeek();
    } catch (e) {
      console.error("清空本周数据失败", e);
    }
  }

  /** 把当前内存中所有任务的编辑（内容/序号/完成态）一次性落库。
   *  用于弹窗关闭前兜底——即便某行 NInput 还没 blur，内存值（Pinia）已是最新，这里统一写回。 */
  async function flushAll() {
    const cw = currentWeek.value;
    if (!cw) return;
    const jobs: Promise<void>[] = [];
    for (const t of cw.planned) {
      jobs.push(updateTaskRow("planned", t.id, t.title, t.sort_order, t.done));
    }
    for (const t of cw.adhoc) {
      jobs.push(updateTaskRow("adhoc", t.id, t.title, t.sort_order, t.done));
    }
    await Promise.all(jobs);
  }

  return {
    rawText,
    parsed,
    currentWeek,
    saving,
    saveMsg,
    hasPlan,
    loadCurrentWeek,
    doParse,
    save,
    updateRaw,
    updateTaskRow,
    addAdhoc,
    clearWeekDataAll,
    flushAll,
  };
});
