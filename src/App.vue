<script setup lang="ts">
import { NConfigProvider, NMessageProvider, NDialogProvider, zhCN, dateZhCN } from "naive-ui";
import { useTimerStore } from "@/stores/timer";

const timer = useTimerStore();
</script>

<template>
  <NConfigProvider
    :locale="zhCN"
    :date-locale="dateZhCN"
    :theme-overrides="{
      common: {
        primaryColor: '#3b82f6',
        primaryColorHover: '#2563eb',
        primaryColorPressed: '#1d4ed8',
      },
    }"
  >
    <NMessageProvider>
      <NDialogProvider>
        <div class="app-shell" :class="{ mini: timer.miniMode }">
          <RouterView />
        </div>
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style>
/* 透明窗口（tauri.conf transparent:true）下，html/body 不设背景以保持透明；
   由 .app-shell 在常态自绘不透明底色，浮球态切透明露出桌面 */
html,
body {
  margin: 0;
  overflow: hidden;
}
</style>

<style scoped>
.app-shell {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f5f7fa;
}

.app-shell.mini {
  background: transparent;
}
</style>
