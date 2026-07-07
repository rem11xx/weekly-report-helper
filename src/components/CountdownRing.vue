<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    /** 0 ~ 1 */
    progress: number;
    /** 显示的时间文本 MM:SS */
    display: string;
    /** 当前阶段 */
    phase: "idle" | "focus" | "break";
    radius?: number;
  }>(),
  { radius: 110 }
);

const circumference = computed(() => 2 * Math.PI * props.radius);
const strokeDashoffset = computed(
  () => circumference.value * (1 - props.progress)
);

/** 描边宽度随半径缩放：常态 110→10px；浮球态 20→4px（max 兜底避免过细） */
const strokeWidth = computed(() => Math.max(4, Math.round(props.radius * 0.09)));

/** 时间字号随半径缩放（radius 110→55px）；浮球态隐藏中心文字，不依赖此值 */
const timeFontSize = computed(() => `${Math.round(props.radius * 0.5)}px`);
/** 阶段标签同步缩放，保持视觉比例 */
const labelFontSize = computed(
  () => `${Math.max(16, Math.round(props.radius * 0.1))}px`
);

const phaseColor = computed(() => {
  switch (props.phase) {
    case "focus":
      return "#3b82f6";
    case "break":
      return "#60a5fa";
    default:
      return "#3b82f6";
  }
});

const phaseLabel = computed(() => {
  switch (props.phase) {
    case "focus":
      return "专注";
    case "break":
      return "休息";
    default:
      return "专注";
  }
});

/** 圆环可点击，悬停提示随当前阶段变化 */
const hint = computed(() => {
  switch (props.phase) {
    case "focus":
      return "点击结束专注";
    case "break":
      return "点击跳过休息";
    default:
      return "点击开始专注";
  }
});
</script>

<template>
  <div class="countdown-ring" :title="hint">
    <svg :width="radius * 2 + 28" :height="radius * 2 + 28" class="ring-svg">
      <!-- 背景圆环 -->
      <circle
        :cx="radius + 14"
        :cy="radius + 14"
        :r="radius"
        fill="none"
        stroke="#e5e7eb"
        :stroke-width="strokeWidth"
      />
      <!-- 进度圆环 -->
      <circle
        :cx="radius + 14"
        :cy="radius + 14"
        :r="radius"
        fill="none"
        :stroke="phaseColor"
        :stroke-width="strokeWidth"
        stroke-linecap="round"
        :stroke-dasharray="circumference"
        :stroke-dashoffset="strokeDashoffset"
        :transform="`rotate(-90, ${radius + 14}, ${radius + 14})`"
        style="transition: stroke-dashoffset 0.3s ease"
      />
    </svg>
    <div class="center-content">
      <div class="time-display" :style="{ color: phaseColor, fontSize: timeFontSize }">{{ display || "25:00" }}</div>
      <div class="phase-label" :style="{ fontSize: labelFontSize }">{{ phaseLabel }}</div>
    </div>
  </div>
</template>

<style scoped>
.countdown-ring {
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
}

.ring-svg {
  display: block;
}

.center-content {
  position: absolute;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.phase-label {
  font-size: 16px;
  font-weight: 500;
  color: #4b5563;
  margin-top: 4px;
}

.time-display {
  font-size: 56px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  letter-spacing: 2px;
}
</style>
