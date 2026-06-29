import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { getTaskOptions, listProjects, createAdhocTask, recordSession } from "@/api";
import { useClockStore } from "@/stores/clock";
import type { TaskOption } from "@/types";

/** 番茄钟状态 */
export type TimerPhase = "idle" | "focus" | "break";
/** 预设档位 */
export interface PomodoroPreset {
  label: string;
  focusMin: number;
  breakMin: number;
}

const PRESETS: PomodoroPreset[] = [
  { label: "25/5", focusMin: 25, breakMin: 5 },
  { label: "50/10", focusMin: 50, breakMin: 10 },
];

export const useTimerStore = defineStore("timer", () => {
  const clock = useClockStore();

  // ---- 预设 ----
  const presets = PRESETS;
  const presetIndex = ref(0);
  const preset = computed(() => presets[presetIndex.value]);

  // ---- 状态 ----
  const phase = ref<TimerPhase>("idle");
  const remaining = ref(0); // 剩余秒数
  const total = ref(0); // 总秒数（当前轮）
  const startedAt = ref(""); // ISO 时间戳

  // 上一个专注时段选中的任务（用于默认高亮）
  const lastTaskSource = ref<"planned" | "adhoc" | null>(null);
  const lastTaskId = ref<number | null>(null);

  // 任务选项列表（弹窗用）
  const taskOptions = ref<TaskOption[]>([]);
  const projects = ref<string[]>([]);
  // 弹窗是否显示
  const showTaskPicker = ref(false);
  // 当前 session 的 started_at（结束时传给后端）
  const focusStartAt = ref("");

  // ---- 计算属性 ----
  const progress = computed(() => {
    if (total.value === 0) return 0;
    return 1 - remaining.value / total.value;
  });

  const minutesDisplay = computed(() => {
    const m = Math.floor(remaining.value / 60);
    const s = remaining.value % 60;
    return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
  });

  // ---- 倒计时 ----
  let timerHandle: ReturnType<typeof setInterval> | null = null;

  function startFocus() {
    phase.value = "focus";
    total.value = preset.value.focusMin * 60;
    remaining.value = total.value;
    startedAt.value = clock.nowDate().toISOString();
    focusStartAt.value = startedAt.value;
    startTick();
  }

  function startBreak() {
    phase.value = "break";
    total.value = preset.value.breakMin * 60;
    remaining.value = total.value;
    startedAt.value = clock.nowDate().toISOString();
    startTick();
  }

  function startTick() {
    stopTick();
    timerHandle = setInterval(() => {
      remaining.value--;
      if (remaining.value <= 0) {
        remaining.value = 0;
        stopTick();
        onTimerEnd();
      }
    }, 1000);
  }

  function stopTick() {
    if (timerHandle) {
      clearInterval(timerHandle);
      timerHandle = null;
    }
  }

  function pause() {
    stopTick();
  }

  function resume() {
    if (phase.value !== "idle") {
      startTick();
    }
  }

  function reset() {
    stopTick();
    phase.value = "idle";
    remaining.value = 0;
    total.value = 0;
  }

  /** 手动结束专注：走与自然结束相同的「弹窗选任务 → 记录 session → 进休息」流程 */
  function manualEnd() {
    stopTick();
    onTimerEnd();
  }

  /** 计时结束处理 */
  async function onTimerEnd() {
    if (phase.value === "focus") {
      // 专注结束 → 弹窗选任务 → 选完后记录 session 并进休息
      // 实际上我们在选完任务后再记录，这里先弹窗
      await loadTaskOptions();
      showTaskPicker.value = true;
    } else if (phase.value === "break") {
      // 休息结束 → 回到 idle（等待手动开始下一个）
      const breakEnd = clock.nowDate().toISOString();
      const breakMin = preset.value.breakMin;
      await doRecordSession(null, null, startedAt.value, breakEnd, breakMin, true);
      phase.value = "idle";
      remaining.value = 0;
      total.value = 0;
    }
  }

  /** 加载任务选项 */
  async function loadTaskOptions() {
    try {
      taskOptions.value = await getTaskOptions();
      projects.value = await listProjects();
    } catch (e) {
      console.error("加载任务选项失败", e);
    }
  }

  /** 用户选择任务后记录 session */
  async function selectTask(
    source: "planned" | "adhoc",
    taskId: number | null,
    // 新建 adhoc 时可选
    newProject?: string,
    newTitle?: string
  ) {
    showTaskPicker.value = false;

    let finalSource = source;
    let finalTaskId: number | null = taskId;

    // 新建计划外任务
    if (source === "adhoc" && finalTaskId === null && newTitle) {
      try {
        const task = await createAdhocTask(newProject || "", newTitle);
        finalTaskId = task.id;
      } catch (e) {
        console.error("新建计划外任务失败", e);
        return;
      }
    }

    const focusEnd = clock.nowDate().toISOString();
    const focusMin = Math.round(
      (new Date(focusEnd).getTime() - new Date(focusStartAt.value).getTime()) / 60000
    );

    await doRecordSession(finalSource, finalTaskId, focusStartAt.value, focusEnd, focusMin, false);

    // 记住上次选中
    lastTaskSource.value = finalSource;
    lastTaskId.value = finalTaskId;

    // 自动开始休息
    startBreak();
  }

  /** 记录 session 到后端 */
  async function doRecordSession(
    source: "planned" | "adhoc" | null,
    taskId: number | null,
    startedAt: string,
    endedAt: string,
    durationMin: number,
    isBreak: boolean
  ) {
    try {
      await recordSession({
        task_source: source || "planned",
        task_id: taskId,
        started_at: startedAt,
        ended_at: endedAt,
        duration_min: durationMin,
        is_break: isBreak,
      });
    } catch (e) {
      console.error("记录 session 失败", e);
    }
  }

  return {
    presets,
    presetIndex,
    preset,
    phase,
    remaining,
    total,
    startedAt,
    lastTaskSource,
    lastTaskId,
    taskOptions,
    projects,
    showTaskPicker,
    progress,
    minutesDisplay,
    startFocus,
    startBreak,
    pause,
    resume,
    reset,
    manualEnd,
    selectTask,
    loadTaskOptions,
  };
});
