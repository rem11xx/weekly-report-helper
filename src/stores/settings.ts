import { defineStore } from "pinia";
import { ref } from "vue";
import {
  getAlwaysOnTop,
  setAlwaysOnTop,
  getDbStoragePath,
  setDbStoragePath,
  restoreDefaultDbPath,
} from "@/api";
import type { DbStorageInfo } from "@/types";

export const useSettingsStore = defineStore("settings", () => {
  /** 窗口置顶 */
  const alwaysOnTop = ref(false);
  /** 切换中（开关 loading） */
  const loading = ref(false);

  /** 数据库存储位置信息 */
  const dbInfo = ref<DbStorageInfo | null>(null);
  /** 数据库位置切换中（按钮 loading） */
  const dbBusy = ref(false);

  /** 从后端同步当前设置 */
  async function load() {
    try {
      alwaysOnTop.value = await getAlwaysOnTop();
    } catch (e) {
      console.error("读取置顶设置失败", e);
    }
    try {
      dbInfo.value = await getDbStoragePath();
    } catch (e) {
      console.error("读取数据库路径失败", e);
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

  /** 切换数据库存储文件夹。成功后后端会触发重启，进程退出；
   *  失败时复位 loading 并抛出，由组件层提示。确认弹窗 / loading toast 留在组件层。 */
  async function setDbPath(dir: string) {
    dbBusy.value = true;
    try {
      await setDbStoragePath(dir);
    } catch (e) {
      dbBusy.value = false;
      throw e;
    }
    // 成功路径：进程即将退出，无需复位 dbBusy
  }

  /** 恢复默认存储位置。同样后端会重启。 */
  async function restoreDb() {
    dbBusy.value = true;
    try {
      await restoreDefaultDbPath();
    } catch (e) {
      dbBusy.value = false;
      throw e;
    }
  }

  return {
    alwaysOnTop,
    loading,
    dbInfo,
    dbBusy,
    load,
    updateAlwaysOnTop,
    setDbPath,
    restoreDb,
  };
});
