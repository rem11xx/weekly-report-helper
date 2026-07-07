<script setup lang="ts">
import { watch } from "vue";
import {
  NModal,
  NSwitch,
  NButton,
  NSpace,
  NInput,
  NTag,
  useMessage,
  useDialog,
} from "naive-ui";
import { useSettingsStore } from "@/stores/settings";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const store = useSettingsStore();
const message = useMessage();
const dialog = useDialog();

// 打开时从后端同步最新值
watch(
  () => props.show,
  async (show) => {
    if (show) await store.load();
  }
);

async function onToggle(v: boolean) {
  try {
    await store.updateAlwaysOnTop(v);
  } catch {
    message.error("设置失败，请重试");
  }
}

async function onToggleMini(v: boolean) {
  try {
    await store.updateFocusEntersMini(v);
  } catch {
    message.error("设置失败，请重试");
  }
}

// 选择文件夹 → 二次确认 → 后端迁移+重启。
// 重启会杀进程，invoke 的 promise 不一定 resolve，故先弹持久 loading toast。
async function onPickFolder() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({ directory: true, multiple: false });
  if (!picked) return;
  const dir = Array.isArray(picked) ? picked[0] : (picked as string);
  if (!dir) return;

  dialog.warning({
    title: "切换数据库位置",
    content: "将切换 weekly.db 存储文件夹并自动重启应用，继续？",
    positiveText: "确认并重启",
    negativeText: "取消",
    onPositiveClick: async () => {
      const hide = message.loading("正在迁移数据并重启…", { duration: 0 });
      try {
        await store.setDbPath(dir);
      } catch (e) {
        hide.destroy();
        message.error("切换失败：" + String(e));
      }
    },
  });
}

function onRestoreDefault() {
  dialog.warning({
    title: "恢复默认位置",
    content: "将把数据库迁回默认位置并重启应用，继续？",
    positiveText: "确认并重启",
    negativeText: "取消",
    onPositiveClick: async () => {
      const hide = message.loading("正在迁移并重启…", { duration: 0 });
      try {
        await store.restoreDb();
      } catch (e) {
        hide.destroy();
        message.error("恢复失败：" + String(e));
      }
    },
  });
}

function close() {
  emit("update:show", false);
}
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    title="设置"
    style="width: 420px; max-width: 92vw"
    :mask-closable="true"
    :closable="true"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <div class="settings-row">
      <div class="settings-label">
        <div class="title">窗口置顶</div>
        <div class="desc">开启后窗口始终显示在最上层</div>
      </div>
      <NSwitch :value="store.alwaysOnTop" :loading="store.loading" @update:value="onToggle" />
    </div>

    <div class="settings-row">
      <div class="settings-label">
        <div class="title">开始专注进入浮球</div>
        <div class="desc">开启后点击「开始」即收起为浮球，仅显示倒计时圆环；关闭则停留在主界面</div>
      </div>
      <NSwitch
        :value="store.focusEntersMini"
        :loading="store.loading"
        @update:value="onToggleMini"
      />
    </div>

    <div class="settings-row db-row">
      <div class="settings-label">
        <div class="title">
          数据库位置
          <NTag
            size="small"
            :type="store.dbInfo?.is_custom ? 'warning' : 'success'"
            :bordered="false"
          >
            {{ store.dbInfo?.is_custom ? "自定义" : "默认" }}
          </NTag>
        </div>
        <div class="desc">切换 weekly.db 存储文件夹，切换后应用自动重启</div>
      </div>
      <div class="db-controls">
        <NInput
          :value="store.dbInfo?.path ?? ''"
          readonly
          type="textarea"
          :autosize="{ minRows: 1, maxRows: 2 }"
          placeholder="未读取"
          size="small"
        />
        <NSpace :size="8">
          <NButton size="small" tertiary :loading="store.dbBusy" @click="onPickFolder">
            选择文件夹…
          </NButton>
          <NButton
            size="small"
            tertiary
            :disabled="!store.dbInfo?.is_custom"
            @click="onRestoreDefault"
          >
            恢复默认
          </NButton>
        </NSpace>
      </div>
    </div>

    <template #footer>
      <NSpace justify="end">
        <NButton @click="close">关闭</NButton>
      </NSpace>
    </template>
  </NModal>
</template>

<style scoped>
.settings-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 4px 2px;
}

/* 数据库位置行：纵向排布，路径 + 按钮组 */
.db-row {
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  margin-top: 12px;
}

.db-controls {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.settings-label .title {
  font-size: 15px;
  font-weight: 600;
  color: #111827;
  display: flex;
  align-items: center;
  gap: 6px;
}

.settings-label .desc {
  font-size: 12px;
  color: #6b7280;
  margin-top: 4px;
}
</style>
