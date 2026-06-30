<script setup lang="ts">
import { watch } from "vue";
import { NModal, NSwitch, NButton, NSpace, useMessage } from "naive-ui";
import { useSettingsStore } from "@/stores/settings";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const store = useSettingsStore();
const message = useMessage();

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

.settings-label .title {
  font-size: 15px;
  font-weight: 600;
  color: #111827;
}

.settings-label .desc {
  font-size: 12px;
  color: #6b7280;
  margin-top: 4px;
}
</style>
