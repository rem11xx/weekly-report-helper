import { defineStore } from "pinia";
import { ref } from "vue";
import { parsePlan, saveWeekPlan, getCurrentWeek } from "@/api";
import type { ParsedPlan, CurrentWeek } from "@/types";

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

  return {
    rawText,
    parsed,
    currentWeek,
    saving,
    saveMsg,
    loadCurrentWeek,
    doParse,
    save,
    updateRaw,
  };
});
