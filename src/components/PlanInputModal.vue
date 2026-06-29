<script setup lang="ts">
import { computed, onMounted, watch } from "vue";
import { usePlanStore } from "@/stores/plan";
import { NModal, NInput, NButton, NSpace, NAlert, NEmpty } from "naive-ui";
import PlanTaskTable from "@/components/PlanTaskTable.vue";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const store = usePlanStore();

onMounted(() => {
  store.loadCurrentWeek();
});

// 打开时重新加载（实现「下次打开按序号重排」）；关闭时把所有未落库的编辑写回数据库
watch(
  () => props.show,
  async (show, prev) => {
    if (show) {
      await store.loadCurrentWeek();
    } else if (prev) {
      await store.flushAll();
    }
  }
);

// 无计划态：把解析预览按项目分组（同项目首列合并）
const previewGroups = computed(() => {
  const map = new Map<string, typeof store.parsed.tasks>();
  for (const t of store.parsed.tasks) {
    const arr = map.get(t.project) ?? [];
    arr.push(t);
    map.set(t.project, arr);
  }
  return Array.from(map.entries()).map(([project, tasks]) => ({ project, tasks }));
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
    :title="store.hasPlan ? '本周任务清单' : '周计划输入'"
    style="width: 640px; max-width: 92vw"
    :mask-closable="true"
    :closable="true"
    @update:show="(v: boolean) => emit('update:show', v)"
  >
    <div class="plan-modal">
      <!-- 有计划态：只展示可编辑表格 -->
      <PlanTaskTable v-if="store.hasPlan" />

      <!-- 无计划态：文本输入 + 只读表格预览 -->
      <template v-else>
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

          <NAlert v-if="hasErrors" type="warning" style="margin-bottom: 12px">
            <div v-for="(err, i) in store.parsed.errors" :key="i">
              ⚠️ {{ err }}
            </div>
          </NAlert>

          <table v-if="hasTasks" class="preview-table">
            <thead>
              <tr>
                <th class="col-proj">项目</th>
                <th class="col-seq">序号</th>
                <th class="col-detail">任务详情</th>
              </tr>
            </thead>
            <tbody>
              <template v-for="g in previewGroups" :key="g.project">
                <tr v-for="(t, idx) in g.tasks" :key="t.raw">
                  <td v-if="idx === 0" :rowspan="g.tasks.length" class="cell-proj">
                    {{ g.project || "未分类" }}
                  </td>
                  <td class="cell-seq">{{ t.sort_order }}</td>
                  <td class="cell-detail">{{ t.title }}</td>
                </tr>
              </template>
            </tbody>
          </table>
        </div>

        <NEmpty
          v-else-if="store.rawText.trim()"
          description="未解析到任务，请检查格式"
          style="margin-top: 16px"
        />
      </template>
    </div>

    <template #footer>
      <NSpace justify="end">
        <template v-if="store.hasPlan">
          <NButton type="primary" @click="close">关闭</NButton>
        </template>
        <template v-else>
          <NButton @click="close">取消</NButton>
          <NButton
            type="primary"
            :loading="store.saving"
            :disabled="!store.rawText.trim()"
            @click="save"
          >
            保存
          </NButton>
        </template>
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

.preview-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.preview-table th {
  text-align: left;
  font-weight: 600;
  color: #6b7280;
  padding: 6px 8px;
  border-bottom: 1px solid #e5e7eb;
  background: #f9fafb;
}

.preview-table td {
  padding: 5px 8px;
  border-bottom: 1px solid #f3f4f6;
  vertical-align: middle;
}

.col-proj,
.cell-proj {
  width: 110px;
  font-weight: 600;
  color: #3b82f6;
}

.col-seq,
.cell-seq {
  width: 60px;
  color: #6b7280;
}

.cell-detail {
  color: #4b5563;
}
</style>
