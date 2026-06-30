import { defineStore } from "pinia";
import { ref } from "vue";
import { getAlwaysOnTop, setAlwaysOnTop } from "@/api";

export const useSettingsStore = defineStore("settings", () => {
  /** 窗口置顶 */
  const alwaysOnTop = ref(false);
  /** 切换中（开关 loading） */
  const loading = ref(false);

  /** 从后端同步当前设置 */
  async function load() {
    try {
      alwaysOnTop.value = await getAlwaysOnTop();
    } catch (e) {
      console.error("读取置顶设置失败", e);
    }
  }

  /** 切换置顶：乐观更新，失败回滚 */
  async function updateAlwaysOnTop(v: boolean) {
    const prev = alwaysOnTop.value;
    alwaysOnTop.value = v;
    loading.value = true;
    try {
      await setAlwaysOnTop(v);
    } catch (e) {
      alwaysOnTop.value = prev;
      throw e;
    } finally {
      loading.value = false;
    }
  }

  return { alwaysOnTop, loading, load, updateAlwaysOnTop };
});
