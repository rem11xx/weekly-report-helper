import { defineStore } from "pinia";
import { ref } from "vue";
import { setMockNow, clearMockNow } from "@/api";

/**
 * 时钟 store —— 统一"当前时间"来源。
 *
 * 生产构建下 mockNow 始终为 null，nowDate() 返回真实时间；
 * dev 构建下可由 TimerView 的"模拟时间"面板注入一个 ISO 字符串，
 * 同步覆盖前端（番茄钟时间戳）与后端（current_week_range / needs_plan_reminder），
 * 用于在真实界面上测试周二提醒窗口、周五报告、周一归属上周等时间点。
 */
export const useClockStore = defineStore("clock", () => {
  /** 注入的模拟时间（ISO 形如 2026-06-17T13:00）；null 表示用真实系统时间 */
  const mockNow = ref<string | null>(null);

  /** 取当前 Date：有 mock 用 mock，否则用真实时间 */
  function nowDate(): Date {
    return mockNow.value ? new Date(mockNow.value) : new Date();
  }

  /** 注入模拟时间（前后端同步） */
  async function set(iso: string) {
    mockNow.value = iso;
    try {
      await setMockNow(iso);
    } catch (e) {
      console.error("后端注入模拟时间失败", e);
    }
  }

  /** 清除模拟时间，恢复真实系统时间（前后端同步） */
  async function clear() {
    mockNow.value = null;
    try {
      await clearMockNow();
    } catch (e) {
      console.error("后端清除模拟时间失败", e);
    }
  }

  return { mockNow, nowDate, set, clear };
});
