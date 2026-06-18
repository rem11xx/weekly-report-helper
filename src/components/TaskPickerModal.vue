<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useTimerStore } from "@/stores/timer";
import type { TaskOption } from "@/types";
import {
  NModal,
  NButton,
  NRadioGroup,
  NRadio,
  NSpace,
  NInput,
  NAutoComplete,
  NTag,
  NDivider,
} from "naive-ui";

const store = useTimerStore();

const emit = defineEmits<{
  (e: "confirm"): void;
}>();

/** 选中的任务 key：source:taskId */
const selectedKey = ref("");

/** 新建任务相关 */
const newProject = ref("");
const newTitle = ref("");

/** 是否选择了"新建任务" */
const isNewTaskMode = computed(() => selectedKey.value === "__new__");

/** 计算默认选中：沿用上一次任务 */
const defaultKey = computed(() => {
  if (store.lastTaskSource && store.lastTaskId != null) {
    return `${store.lastTaskSource}:${store.lastTaskId}`;
  }
  return "";
});

onMounted(() => {
  selectedKey.value = defaultKey.value;
});

/** 确认选择 */
function confirm() {
  if (isNewTaskMode.value) {
    store.selectTask("adhoc", null, newProject.value, newTitle.value);
  } else {
    const [source, idStr] = selectedKey.value.split(":");
    store.selectTask(source as "planned" | "adhoc", Number(idStr));
  }
  emit("confirm");
}

/** 按项目分组 */
const groupedOptions = computed(() => {
  const map = new Map<string, TaskOption[]>();
  for (const opt of store.taskOptions) {
    if (!map.has(opt.project)) map.set(opt.project, []);
    map.get(opt.project)!.push(opt);
  }
  return map;
});
</script>

<template>
  <NModal
    :show="store.showTaskPicker"
    preset="card"
    title="这个番茄钟你在做什么？"
    style="width: 520px; max-width: 90vw"
    :mask-closable="false"
    :closable="false"
  >
    <div class="task-picker">
      <!-- 已有任务列表 -->
      <NRadioGroup v-model:value="selectedKey" class="task-list">
        <div
          v-for="[project, tasks] in groupedOptions"
          :key="project"
          class="task-group"
        >
          <div class="group-label">{{ project || "未分类" }}</div>
          <div v-for="opt in tasks" :key="`${opt.source}:${opt.task_id}`" class="task-option">
            <NRadio :value="`${opt.source}:${opt.task_id}`">
              <span class="option-text">{{ opt.title }}</span>
              <NTag size="tiny" :bordered="false" :type="opt.source === 'planned' ? 'info' : 'warning'">
                {{ opt.source === "planned" ? "计划内" : "计划外" }}
              </NTag>
            </NRadio>
          </div>
        </div>

        <NDivider style="margin: 8px 0" />

        <!-- 新建任务选项 -->
        <NRadio value="__new__" class="new-task-radio">
          <span style="font-weight: 600">+ 新建任务</span>
        </NRadio>
      </NRadioGroup>

      <!-- 新建任务表单 -->
      <div v-if="isNewTaskMode" class="new-task-form">
        <NAutoComplete
          v-model:value="newProject"
          :options="store.projects.map(p => ({ label: p, value: p }))"
          placeholder="所属项目"
          style="margin-bottom: 8px"
        />
        <NInput
          v-model:value="newTitle"
          placeholder="任务标题"
          @keyup.enter="confirm"
        />
      </div>
    </div>

    <template #footer>
      <NSpace justify="end">
        <NButton type="primary" :disabled="!selectedKey" @click="confirm">
          确认
        </NButton>
      </NSpace>
    </template>
  </NModal>
</template>

<style scoped>
.task-picker {
  max-height: 400px;
  overflow-y: auto;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.task-group {
  margin-bottom: 4px;
}

.group-label {
  font-size: 12px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.5);
  padding: 4px 0 2px;
}

.task-option {
  padding: 4px 8px;
  border-radius: 6px;
}

.task-option:hover {
  background: rgba(128, 128, 128, 0.08);
}

.option-text {
  margin-right: 8px;
}

.new-task-form {
  padding: 12px;
  background: rgba(128, 128, 128, 0.05);
  border-radius: 8px;
  margin-top: 8px;
}

.new-task-radio {
  padding: 4px 8px;
}
</style>
