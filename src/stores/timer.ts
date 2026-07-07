import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { getTaskOptions, listProjects, createAdhocTask, recordSession, getAlwaysOnTop, getFocusEntersMini } from "@/api";
import { useClockStore } from "@/stores/clock";
import { enterMiniWindow, restoreWindow } from "@/lib/miniWindow";
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

  // 浮球模式（专注中收起为仅圆环的小窗）
  const miniMode = ref(false);
  // 进入浮球前的窗口置顶偏好，退出时恢复（不写库，仅浮球期间临时置顶）
  let savedOnTop = false;

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
    // 按设置决定是否开始即进入浮球（默认开）
    void maybeEnterMini();
  }

  /** 读取「开始专注即进入浮球」设置，开启则收起为浮球 */
  async function maybeEnterMini() {
    try {
      if (await getFocusEntersMini()) await enterMini();
    } catch (e) {
      console.error("读取专注浮球设置失败", e);
    }
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
    if (miniMode.value) void exitMini();
    phase.value = "idle";
    remaining.value = 0;
    total.value = 0;
  }

  /** 手动结束专注：走与自然结束相同的「弹窗选任务 → 记录 session → 进休息」流程 */
  function manualEnd() {
    stopTick();
    onTimerEnd();
  }

  /** 进入浮球模式：仅专注态可用；记下置顶偏好 → 去标题栏 + 缩窗 + 强制置顶 */
  async function enterMini() {
    if (phase.value !== "focus") return;
    try {
      savedOnTop = await getAlwaysOnTop();
      miniMode.value = true;
      await enterMiniWindow();
    } catch (e) {
      console.error("进入浮球模式失败", e);
      miniMode.value = false;
    }
  }

  /** 退出浮球模式：恢复标题栏 + 原尺寸 + 用户置顶偏好 */
  async function exitMini() {
    if (!miniMode.value) return;
    try {
      await restoreWindow(savedOnTop);
    } catch (e) {
      console.error("退出浮球模式失败", e);
    }
    miniMode.value = false;
  }

  /** 【dev】快进到结束前 5 秒：把剩余时间设为 5 秒并保证倒计时在跑，
   *  5 秒后自然触发「弹窗选任务 → 记录 session → 进休息」流程。
   *  对专注阶段额外把 focusStartAt 前移到 (total-5) 秒前，使 5 秒后结束时
   *  记录的 session 时长 ≈ 预设时长（25/50min），从而一并验证分钟数/状态判定等时长逻辑。
   *  休息阶段无需调整：break 时长直接取预设 breakMin，与墙钟无关。 */
  function fastForwardToEnd() {
    if (phase.value === "idle") return;
    if (remaining.value <= 5) return; // 已不足 5 秒则不动，避免倒退
    if (phase.value === "focus") {
      // 假装专注是在 (total-5) 秒前开始的：结束时刻 - 此刻 = total 秒 = 预设时长
      const start = clock.nowDate();
      start.setSeconds(start.getSeconds() - (total.value - 5));
      focusStartAt.value = start.toISOString();
    }
    remaining.value = 5;
    startTick(); // 即便之前是暂停状态，也保证 5 秒后能触发自然结束
  }

  /** 计时结束处理 */
  async function onTimerEnd() {
    if (phase.value === "focus") {
      // 专注结束 → 先退出浮球恢复主界面 → 弹窗选任务 → 选完后记录 session 并进休息
      await exitMini();
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
    miniMode,
    progress,
    minutesDisplay,
    startFocus,
    startBreak,
    pause,
    resume,
    reset,
    manualEnd,
    enterMini,
    exitMini,
    fastForwardToEnd,
    selectTask,
    loadTaskOptions,
  };
});
