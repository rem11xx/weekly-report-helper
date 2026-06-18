<script setup lang="ts">
import { onMounted, computed } from "vue";
import { usePlanStore } from "@/stores/plan";
import {
  NInput,
  NButton,
  NAlert,
  NTag,
  NSpace,
  NEmpty,
} from "naive-ui";

const store = usePlanStore();

onMounted(() => {
  store.loadCurrentWeek();
});

/** 解析出的任务按项目分组 */
const tasksByProject = computed(() => {
  const map = new Map<string, typeof store.parsed.tasks>();
  for (const t of store.parsed.tasks) {
    if (!map.has(t.project)) map.set(t.project, []);
    map.get(t.project)!.push(t);
  }
  return map;
});

const hasTasks = computed(() => store.parsed.tasks.length > 0);
const hasErrors = computed(() => store.parsed.errors.length > 0);
</script>

<template>
  <div class="plan-input">
    <h2 class="page-title">本周计划</h2>

    <!-- 提示区域 -->
    <NAlert
      v-if="store.saveMsg"
      type="success"
      closable
      style="margin-bottom: 16px"
    >
      {{ store.saveMsg }}
    </NAlert>

    <!-- 已有本周计划的加载提示 -->
    <NAlert
      v-if="store.currentWeek?.week && !store.rawText"
      type="info"
      style="margin-bottom: 16px"
    >
      本周（{{ store.currentWeek.week.week_start }} ~
      {{ store.currentWeek.week.week_end }}）已有计划，可直接编辑。
    </NAlert>

    <div class="input-row">
      <NInput
        v-model:value="store.rawText"
        type="textarea"
        placeholder="粘贴或输入本周计划文本..."
        :rows="12"
        :autosize="{ minRows: 8, maxRows: 24 }"
        @input="(v: string) => store.updateRaw(v)"
        style="font-family: monospace; font-size: 13px"
      />
    </div>

    <!-- 解析预览 -->
    <div v-if="hasTasks || hasErrors" class="preview-section">
      <h3 class="preview-title">
        解析预览（{{ store.parsed.tasks.length }} 个任务）
      </h3>

      <!-- 解析错误 -->
      <NAlert
        v-if="hasErrors"
        type="warning"
        style="margin-bottom: 12px"
      >
        <div v-for="(err, i) in store.parsed.errors" :key="i">
          ⚠️ {{ err }}
        </div>
      </NAlert>

      <!-- 周范围 -->
      <div class="week-range">
        周范围：{{ store.parsed.week_start }} ~ {{ store.parsed.week_end }}
      </div>

      <!-- 按项目分组展示 -->
      <div
        v-for="[project, tasks] in tasksByProject"
        :key="project"
        class="project-block"
      >
        <div class="project-name">{{ project }}</div>
        <div v-for="t in tasks" :key="t.raw" class="task-item">
          <NSpace align="center" :size="8">
            <NTag size="small" :bordered="false" type="info">
              #{{ t.sort_order }}
            </NTag>
            <span class="task-title">{{ t.title }}</span>
            <NTag v-if="t.estimate_d > 0" size="small" type="warning">
              {{ t.estimate_d }}d
            </NTag>
          </NSpace>
        </div>
      </div>
    </div>

    <NEmpty
      v-else-if="store.rawText.trim()"
      description="未解析到任务，请检查格式"
      style="margin-top: 20px"
    />

    <!-- 保存按钮 -->
    <div class="action-row">
      <NButton
        type="primary"
        :loading="store.saving"
        :disabled="!store.rawText.trim()"
        @click="store.save()"
      >
        保存本周计划
      </NButton>
    </div>
  </div>
</template>

<style scoped>
.plan-input {
  max-width: 800px;
  margin: 0 auto;
}

.page-title {
  font-size: 20px;
  font-weight: 600;
  margin-bottom: 16px;
}

.input-row {
  margin-bottom: 16px;
}

.preview-section {
  margin-top: 20px;
}

.preview-title {
  font-size: 15px;
  font-weight: 600;
  margin-bottom: 12px;
  color: rgba(255, 255, 255, 0.7);
}

.week-range {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
  margin-bottom: 16px;
}

.project-block {
  margin-bottom: 16px;
  padding: 12px;
  border-radius: 8px;
  background: rgba(128, 128, 128, 0.08);
}

.project-name {
  font-size: 14px;
  font-weight: 600;
  color: #18a058;
  margin-bottom: 8px;
  padding-bottom: 4px;
  border-bottom: 1px solid rgba(128, 128, 128, 0.15);
}

.task-item {
  padding: 4px 0;
}

.task-title {
  font-size: 13px;
}

.action-row {
  margin-top: 24px;
  display: flex;
  gap: 12px;
}
</style>
