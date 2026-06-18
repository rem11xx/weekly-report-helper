<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import { NConfigProvider, NMessageProvider, NDialogProvider, zhCN, dateZhCN } from "naive-ui";

const route = useRoute();
const activeKey = computed(() => route.name as string);
</script>

<template>
  <NConfigProvider :locale="zhCN" :date-locale="dateZhCN">
    <NMessageProvider>
      <NDialogProvider>
        <div class="app-shell">
          <nav class="app-nav">
            <RouterLink to="/plan" class="nav-item" :class="{ active: activeKey === 'plan' }">
              本周计划
            </RouterLink>
            <RouterLink to="/timer" class="nav-item" :class="{ active: activeKey === 'timer' }">
              番茄钟
            </RouterLink>
            <RouterLink to="/report" class="nav-item" :class="{ active: activeKey === 'report' }">
              周报
            </RouterLink>
          </nav>
          <main class="app-main">
            <RouterView />
          </main>
        </div>
      </NDialogProvider>
    </NMessageProvider>
  </NConfigProvider>
</template>

<style scoped>
.app-shell {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}

.app-nav {
  display: flex;
  gap: 4px;
  padding: 8px 16px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.2);
  background: rgba(128, 128, 128, 0.05);
}

.nav-item {
  padding: 6px 16px;
  border-radius: 6px;
  text-decoration: none;
  color: inherit;
  opacity: 0.7;
  font-size: 14px;
  transition: all 0.15s;
}

.nav-item:hover {
  opacity: 1;
  background: rgba(128, 128, 128, 0.12);
}

.nav-item.active {
  opacity: 1;
  background: rgba(24, 160, 88, 0.15);
  color: #18a058;
  font-weight: 600;
}

.app-main {
  flex: 1;
  padding: 20px;
  overflow: auto;
}
</style>
