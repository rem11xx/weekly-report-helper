<script setup lang="ts">
import {
  ref,
  reactive,
  computed,
  onMounted,
  onBeforeUnmount,
  watch,
} from "vue";
import { useMessage } from "naive-ui";
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
} from "naive-ui";

const store = useTimerStore();
const message = useMessage();

const emit = defineEmits<{
  (e: "confirm"): void;
}>();

/** 选中的任务 key：source:taskId */
const selectedKey = ref("");

/** 最近交互的输入：null=默认 / "radio"=计划内 / "group:<project>"=某项目空行 / "bottom"=底部表单。
 *  「确认」按钮据此判断提交哪一处——最近动了哪就提交哪。 */
const lastInteracted = ref<string | null>(null);

/** 每个项目分组下的「空行」输入：key=project，value=标题。
 *  在某组空行输入即视为在该项目下新建计划外任务。 */
const groupNewTitle = reactive<Record<string, string>>({});

/** 底部表单（用于列表中尚无的项目）：项目必填 */
const newProject = ref("");
const newTitle = ref("");

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

/** 弹窗每次打开时清空各输入与交互态（组件常驻挂载，输入会保留）；
 *  同时挂载 window 级回车监听（捕获阶段）——NModal 焦点陷阱会拦截冒泡到
 *  弹窗内部的键盘事件，只有 window 顶端的捕获监听能稳定收到。 */
watch(
  () => store.showTaskPicker,
  (show) => {
    if (show) {
      for (const k of Object.keys(groupNewTitle)) delete groupNewTitle[k];
      newProject.value = "";
      newTitle.value = "";
      lastInteracted.value = null;
      window.addEventListener("keydown", onWindowKeyDown, true);
    } else {
      window.removeEventListener("keydown", onWindowKeyDown, true);
    }
  }
);

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onWindowKeyDown, true);
});

/** 确认选择已有任务（计划内） */
function confirm() {
  const [source, idStr] = selectedKey.value.split(":");
  store.selectTask(source as "planned" | "adhoc", Number(idStr));
  emit("confirm");
}

/** window 级回车处理（捕获阶段，避开 NModal 焦点陷阱对弹窗内事件的拦截）：
 *  焦点在计划外输入区 → 交给输入框自身 @keyup.enter；否则 → 确认选中的计划内任务。 */
function onWindowKeyDown(e: KeyboardEvent) {
  if (!store.showTaskPicker) return;
  if (e.key !== "Enter") return;
  const el = document.activeElement as HTMLElement | null;
  if (!el) return;
  // 计划外输入区（项目空行/底部表单）：交给其自身 @keyup.enter，避免双触发
  if (el.closest("[data-adhoc-input]")) return;
  // 按钮自行处理回车（原生激活）
  if (el.closest("button")) return;
  // 其余（radio 等）→ 确认选中的计划内任务
  if (selectedKey.value) {
    e.preventDefault();
    confirm();
  }
}

/** 在某项目分组下新建计划外任务（项目即该组所属） */
function addGroupAdhoc(project: string) {
  const title = (groupNewTitle[project] ?? "").trim();
  if (!title) return;
  store.selectTask("adhoc", null, project, title);
  emit("confirm");
}

/** 底部表单新建计划外任务：所属项目必填 */
function addBottomAdhoc() {
  const project = newProject.value.trim();
  const title = newTitle.value.trim();
  if (!title) return;
  if (!project) {
    message.warning("请填写所属项目");
    return;
  }
  store.selectTask("adhoc", null, project, title);
  emit("confirm");
}

/** 「确认」按钮：计划内/计划外统一入口——
 *  最近交互处有计划外内容则提交计划外，否则确认选中的计划内任务。 */
function submit() {
  // 最近交互的是计划内 → 确认它
  if (lastInteracted.value === "radio") {
    if (selectedKey.value) confirm();
    return;
  }
  // 否则找一个有内容的计划外输入提交
  const target = pickAdhocTarget();
  if (target) {
    if (target.kind === "bottom" && !target.project) {
      message.warning("请填写所属项目");
      return;
    }
    store.selectTask("adhoc", null, target.project, target.title);
    emit("confirm");
    return;
  }
  // 没有计划外内容 → 确认选中的计划内任务
  if (selectedKey.value) confirm();
}

/** 挑出要提交的计划外输入：优先最近交互的那个（若有内容），否则任一有内容的 */
function pickAdhocTarget():
  | { kind: "group"; project: string; title: string }
  | { kind: "bottom"; project: string; title: string }
  | null {
  const last = lastInteracted.value;
  if (last === "bottom") {
    const title = newTitle.value.trim();
    if (title) return { kind: "bottom", project: newProject.value.trim(), title };
  } else if (last && last.startsWith("group:")) {
    const project = last.slice("group:".length);
    const title = (groupNewTitle[project] ?? "").trim();
    if (title) return { kind: "group", project, title };
  }
  // 回退：任一有内容的输入
  for (const [project, title] of Object.entries(groupNewTitle)) {
    if (title.trim()) return { kind: "group", project, title: title.trim() };
  }
  const bottomTitle = newTitle.value.trim();
  if (bottomTitle) return { kind: "bottom", project: newProject.value.trim(), title: bottomTitle };
  return null;
}

/** 「确认」是否可用：选中了计划内，或任一计划外输入有内容 */
const canSubmit = computed(() => {
  if (selectedKey.value) return true;
  if (newTitle.value.trim()) return true;
  for (const k of Object.keys(groupNewTitle)) {
    if ((groupNewTitle[k] ?? "").trim()) return true;
  }
  return false;
});

/** 单选选中变化（用户点击/方向键） */
function onRadioChange(val: string | number | null) {
  selectedKey.value = String(val ?? "");
  lastInteracted.value = "radio";
}

/** 底部项目输入变化 */
function onBottomProjectInput(val: string) {
  newProject.value = val;
  lastInteracted.value = "bottom";
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
      <!-- 已有任务列表（按项目分组，组前展示项目名；组下空行回车即加计划外任务到该项目） -->
      <NRadioGroup :value="selectedKey" class="task-list" @update:value="onRadioChange">
        <div
          v-for="[project, tasks] in groupedOptions"
          :key="project"
          class="task-group"
        >
          <div class="group-label">{{ project || "未分类" }}</div>
          <div
            v-for="opt in tasks"
            :key="`${opt.source}:${opt.task_id}`"
            class="task-option"
          >
            <NRadio :value="`${opt.source}:${opt.task_id}`">
              <span class="option-text">{{ opt.title }}</span>
              <NTag size="tiny" :bordered="false" :type="opt.source === 'planned' ? 'info' : 'warning'">
                {{ opt.source === "planned" ? "计划内" : "计划外" }}
              </NTag>
            </NRadio>
          </div>
          <!-- 空行：输入即在当前项目下添加计划外任务 -->
          <div class="group-add-row" data-adhoc-input>
            <NInput
              v-model:value="groupNewTitle[project]"
              placeholder="输入计划外任务，回车添加"
              size="small"
              @focus="lastInteracted = 'group:' + project"
              @keyup.enter="addGroupAdhoc(project)"
            />
          </div>
        </div>
      </NRadioGroup>

      <!-- 底部：添加到尚未列出的项目（所属项目必填，回车提交） -->
      <div class="new-task-form" data-adhoc-input>
        <div class="form-label">添加计划外任务</div>
        <NAutoComplete
          :value="newProject"
          :options="store.projects.map((p) => ({ label: p, value: p }))"
          placeholder="所属项目（必填）"
          style="margin-bottom: 8px"
          @update:value="onBottomProjectInput"
        />
        <NInput
          v-model:value="newTitle"
          placeholder="任务内容（回车添加）"
          @focus="lastInteracted = 'bottom'"
          @keyup.enter="addBottomAdhoc"
        />
      </div>
    </div>

    <template #footer>
      <NSpace justify="end">
        <NButton type="primary" :disabled="!canSubmit" @click="submit">
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
  color: rgba(0, 0, 0, 0.45);
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

.group-add-row {
  padding: 4px 8px;
  margin-top: 4px;
}

.new-task-form {
  padding: 12px;
  background: rgba(128, 128, 128, 0.05);
  border-radius: 8px;
  margin-top: 12px;
}

.form-label {
  font-size: 12px;
  font-weight: 600;
  color: rgba(0, 0, 0, 0.45);
  margin-bottom: 8px;
}
</style>
