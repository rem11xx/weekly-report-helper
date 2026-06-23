<script setup lang="ts">
import { computed, onMounted } from "vue";
import { usePlanStore } from "@/stores/plan";
import {
  NModal,
  NInput,
  NButton,
  NSpace,
  NAlert,
  NTag,
  NEmpty,
} from "naive-ui";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const store = usePlanStore();

onMounted(() => {
  store.loadCurrentWeek();
});

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

async function save() {
  await store.save();
  if (!store.saveMsg || store.saveMsg.startsWith("保存失败")) return;
  emit("update:show", false);
}

function close() {
  emit("update:show", false);
}
</script>

<template>
  <NModal
    :show="props.show"
    preset="card"
    title="周计划输入"
    style="width: 520px; max-width: 90vw"
    :mask-closable="false"
    :closable="false"
  >
    <div class="plan-modal">
      <NInput
        v-model:value="store.rawText"
        type="textarea"
        placeholder="在此输入本周计划，每行一项..."
        :rows="8"
        :autosize="{ minRows: 6, maxRows: 14 }"
        @input="(v: string) => store.updateRaw(v)"
        style="font-family: monospace; font-size: 14px"
      />

      <div v-if="hasTasks || hasErrors" class="preview-section">
        <div class="preview-title">实时预览</div>

        <NAlert
          v-if="hasErrors"
          type="warning"
          style="margin-bottom: 12px"
        >
          <div v-for="(err, i) in store.parsed.errors" :key="i">
            ⚠️ {{ err }}
          </div>
        </NAlert>

        <ul class="preview-list">
          <li v-for="[project, tasks] in tasksByProject" :key="project" class="project-group">
            <div class="project-name">{{ project }}</div>
            <ul>
              <li v-for="t in tasks" :key="t.raw" class="task-item">
                <NTag size="tiny" :bordered="false" type="info">#{{ t.sort_order }}</NTag>
                <span class="task-title">{{ t.title }}</span>
              </li>
            </ul>
          </li>
        </ul>
      </div>

      <NEmpty
        v-else-if="store.rawText.trim()"
        description="未解析到任务，请检查格式"
        style="margin-top: 16px"
      />
    </div>

    <template #footer>
      <NSpace justify="end">
        <NButton @click="close">取消</NButton>
        <NButton type="primary" :loading="store.saving" :disabled="!store.rawText.trim()" @click="save">
          保存
        </NButton>
      </NSpace>
    </template>
  </NModal>
</template>

<style scoped>
.plan-modal {
  max-height: 60vh;
  overflow-y: auto;
}

.preview-section {
  margin-top: 16px;
}

.preview-title {
  font-size: 14px;
  font-weight: 600;
  color: #374151;
  margin-bottom: 10px;
}

.preview-list {
  list-style: none;
  padding: 0;
  margin: 0;
}

.project-group {
  margin-bottom: 10px;
}

.project-name {
  font-size: 13px;
  font-weight: 600;
  color: #3b82f6;
  margin-bottom: 6px;
}

.project-group ul {
  list-style: disc;
  padding-left: 18px;
  margin: 0;
}

.task-item {
  font-size: 13px;
  color: #4b5563;
  padding: 2px 0;
  display: flex;
  align-items: center;
  gap: 6px;
}

.task-title {
  flex: 1;
}
</style>
