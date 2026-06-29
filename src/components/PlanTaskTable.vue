<script setup lang="ts">
import { computed, ref } from "vue";
import { NCheckbox, NInput, NTag } from "naive-ui";
import { usePlanStore } from "@/stores/plan";
import type { PlannedTask, AdhocTask } from "@/types";

const store = usePlanStore();

type Row = { source: "planned" | "adhoc"; task: PlannedTask | AdhocTask };
type Group = { project: string; rows: Row[] };

/** 按项目分组：planned + adhoc 合并，组内按 sort_order 升序。
 *  组内按 sort_order 排序是为了让「拖动重排序号」在下次打开时仍按新顺序展示——
 *  后端 get_current_week 分别返回 planned/adhoc（各自按 sort_order 排），前端合并后需重排才不致
 *  把计划内/计划外在同一项目里重新分开。编辑内容不改变 sort_order，故不会触发重排。 */
const groups = computed<Group[]>(() => {
  const cw = store.currentWeek;
  if (!cw) return [];
  const map = new Map<string, Row[]>();
  const push = (t: PlannedTask | AdhocTask, source: "planned" | "adhoc") => {
    const arr = map.get(t.project) ?? [];
    arr.push({ source, task: t });
    map.set(t.project, arr);
  };
  for (const t of cw.planned) push(t, "planned");
  for (const t of cw.adhoc) push(t, "adhoc");
  for (const arr of map.values()) {
    arr.sort((a, b) => a.task.sort_order - b.task.sort_order);
  }
  return Array.from(map.entries()).map(([project, rows]) => ({ project, rows }));
});

/** 各项目下空行输入文本 */
const newTaskText = ref<Record<string, string>>({});

function projLabel(p: string) {
  return p || "未分类";
}

/** 落库单个任务（内容 / 序号 / 完成态） */
async function persist(row: Row) {
  await store.updateTaskRow(
    row.source,
    row.task.id,
    row.task.title,
    row.task.sort_order,
    row.task.done
  );
}

function onDoneChange(row: Row, val: boolean) {
  row.task.done = val;
  persist(row);
}

/** 空行回车：新建计划外任务，归入当前项目 */
async function addNew(project: string) {
  const text = (newTaskText.value[project] || "").trim();
  if (!text) return;
  const ok = await store.addAdhoc(project, text);
  if (ok) newTaskText.value[project] = "";
}

// ============ 拖动排序（仅项目内） ============
const dragGroup = ref<number | null>(null);
const dragRow = ref<number | null>(null);
const overKey = ref<string | null>(null);

function rowKey(gIdx: number, rIdx: number) {
  return `${gIdx}:${rIdx}`;
}

function onDragStart(gIdx: number, rIdx: number, e: DragEvent) {
  dragGroup.value = gIdx;
  dragRow.value = rIdx;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "move";
    // Firefox 需设置 data 才能触发拖动
    e.dataTransfer.setData("text/plain", rowKey(gIdx, rIdx));
  }
}

function onDragOver(gIdx: number, rIdx: number, e: DragEvent) {
  // 仅同组允许放置；跨组忽略（不改变归属项目）
  if (dragGroup.value !== gIdx) return;
  e.preventDefault();
  if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
  overKey.value = rowKey(gIdx, rIdx);
}

function onDrop(gIdx: number, rIdx: number, e: DragEvent) {
  e.preventDefault();
  overKey.value = null;
  if (dragGroup.value !== gIdx || dragRow.value === null) {
    return;
  }
  const from = dragRow.value;
  const to = rIdx;
  if (from === to) return;

  const rows = groups.value[gIdx].rows;
  const moved = rows[from];
  const target = rows[to];
  if (!moved || !target || moved === target) return;

  // 取出 moved，按拖动方向决定插入到 target 之前 / 之后。
  // 向下拖（from < to）时，移除 moved 后 target 已左移一位；若仍插到 target 之前会落回原位
  // （经典 DnD 向下相邻 no-op），故改为插到 target 之后。向上拖时插到 target 之前即可。
  const rest = rows.filter((r) => r !== moved);
  const targetPos = rest.indexOf(target);
  const insertPos = from < to ? targetPos + 1 : targetPos;
  rest.splice(insertPos, 0, moved);

  // 组内重赋连续序号：以该组现存最小 sort_order 为起点，避免越界到其它组区间
  const base = rows.reduce(
    (min, r) => Math.min(min, r.task.sort_order),
    Number.POSITIVE_INFINITY
  );
  rest.forEach((row, i) => {
    row.task.sort_order = base + i;
    persist(row);
  });
}

function onDragEnd() {
  dragGroup.value = null;
  dragRow.value = null;
  overKey.value = null;
}
</script>

<template>
  <div class="plan-task-list">
    <div v-for="(group, gIdx) in groups" :key="group.project" class="proj-block">
      <div class="proj-title">{{ projLabel(group.project) }}</div>
      <table class="proj-table">
        <tbody>
          <tr
            v-for="(row, rIdx) in group.rows"
            :key="row.source + ':' + row.task.id"
            class="task-row"
            :class="{
              'row-done': row.task.done,
              'row-over': overKey === rowKey(gIdx, rIdx),
            }"
            @dragover="onDragOver(gIdx, rIdx, $event)"
            @drop="onDrop(gIdx, rIdx, $event)"
          >
            <td class="cell-handle">
              <!-- 拖拽把手：draggable 放在把手 <span> 上而非整行 <tr>，
                   规避 HTML5 DnD 作用在 <tr> 上 dragstart/drop 不触发的问题，
                   也避免与文本框/勾选框争夺拖拽源。dragover/drop 仍留在 <tr>（整行作放置目标）。 -->
              <span
                class="drag-grip"
                draggable="true"
                title="拖动排序"
                @dragstart="onDragStart(gIdx, rIdx, $event)"
                @dragend="onDragEnd"
                >⠿</span
              >
            </td>
            <td class="cell-seq">
              <div class="seq-inner">
                <NCheckbox
                  :checked="row.task.done"
                  size="small"
                  @update:checked="(v: boolean) => onDoneChange(row, v)"
                />
                <span class="seq-num">{{ row.task.sort_order }}</span>
              </div>
            </td>
            <td class="cell-detail">
              <div class="detail-inner">
                <NInput
                  :value="row.task.title"
                  type="textarea"
                  size="small"
                  :autosize="{ minRows: 1, maxRows: 4 }"
                  :class="{ 'input-done': row.task.done }"
                  placeholder="任务内容"
                  @update:value="(v: string) => (row.task.title = v)"
                  @blur="persist(row)"
                />
                <NTag
                  v-if="row.source === 'adhoc'"
                  size="tiny"
                  :bordered="false"
                  type="warning"
                  >计划外</NTag
                >
              </div>
            </td>
          </tr>
          <!-- 空行：新增归属该项目的计划外任务 -->
          <tr class="row-add">
            <td colspan="3">
              <input
                class="add-input"
                :value="newTaskText[group.project] ?? ''"
                placeholder="添加计划外任务…（回车确认）"
                @input="(e) => (newTaskText[group.project] = (e.target as HTMLInputElement).value)"
                @keyup.enter="addNew(group.project)"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.plan-task-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.proj-block {
  border: 1px solid #e5e7eb;
  border-radius: 8px;
  overflow: hidden;
}

.proj-title {
  padding: 6px 12px;
  font-size: 13px;
  font-weight: 600;
  color: #3b82f6;
  background: #f0f7ff;
  border-bottom: 1px solid #e5e7eb;
}

.proj-table {
  width: 100%;
  table-layout: fixed;
  border-collapse: collapse;
  font-size: 13px;
}

.task-row td {
  padding: 4px 8px;
  border-bottom: 1px solid #f3f4f6;
  vertical-align: top;
}

/* 拖拽把手列：窄列，把 ⠿ 居中靠左 */
.task-row .cell-handle {
  width: 26px;
  padding-left: 10px;
  padding-right: 2px;
}

.drag-grip {
  display: inline-block;
  font-size: 14px;
  line-height: 1;
  color: #cbd5e1;
  user-select: none;
  cursor: grab;
}

.drag-grip:hover {
  color: #94a3b8;
}

.drag-grip:active {
  cursor: grabbing;
}

.row-over {
  background: #eff6ff;
}

.row-done {
  opacity: 0.55;
}

.cell-seq {
  width: 84px;
}

.seq-inner {
  display: flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
}

.seq-num {
  font-size: 12px;
  color: #6b7280;
  min-width: 18px;
  text-align: center;
}

.cell-detail {
  vertical-align: top;
}

.detail-inner {
  display: flex;
  align-items: flex-start;
  gap: 6px;
}

.detail-inner :deep(.n-input) {
  flex: 1;
}

/* NInput textarea 宽度填满，自适应换行 */
.detail-inner :deep(.n-input .n-input__textarea-el) {
  resize: none;
  word-break: break-word;
  white-space: pre-wrap;
}

.input-done :deep(.n-input__textarea-el) {
  text-decoration: line-through;
  color: #9ca3af;
}

.row-add td {
  padding: 6px 8px;
}

.row-add .add-input {
  width: 100%;
  padding: 4px 8px;
  font-size: 13px;
  border: 1px dashed #d1d5db;
  border-radius: 4px;
  color: #9ca3af;
  box-sizing: border-box;
}

.row-add .add-input:focus {
  outline: none;
  border-color: #3b82f6;
  color: #374151;
}
</style>
