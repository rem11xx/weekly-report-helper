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
  { radius: 120 }
);

const circumference = 2 * Math.PI * props.radius;
const strokeDashoffset = computed(
  () => circumference * (1 - props.progress)
);

const phaseColor = computed(() => {
  switch (props.phase) {
    case "focus":
      return "#18a058";
    case "break":
      return "#2080f0";
    default:
      return "#666";
  }
});

const phaseLabel = computed(() => {
  switch (props.phase) {
    case "focus":
      return "专注中";
    case "break":
      return "休息中";
    default:
      return "准备开始";
  }
});
</script>

<template>
  <div class="countdown-ring">
    <svg :width="radius * 2 + 20" :height="radius * 2 + 20" class="ring-svg">
      <!-- 背景圆环 -->
      <circle
        :cx="radius + 10"
        :cy="radius + 10"
        :r="radius"
        fill="none"
        stroke="rgba(128,128,128,0.15)"
        :stroke-width="8"
      />
      <!-- 进度圆环 -->
      <circle
        :cx="radius + 10"
        :cy="radius + 10"
        :r="radius"
        fill="none"
        :stroke="phaseColor"
        :stroke-width="8"
        stroke-linecap="round"
        :stroke-dasharray="circumference"
        :stroke-dashoffset="strokeDashoffset"
        transform="rotate(-90, cx, cy)"
        style="transition: stroke-dashoffset 0.3s ease"
      />
    </svg>
    <div class="center-content">
      <div class="phase-label" :style="{ color: phaseColor }">
        {{ phaseLabel }}
      </div>
      <div class="time-display">{{ display || "00:00" }}</div>
    </div>
  </div>
</template>

<style scoped>
.countdown-ring {
  position: relative;
  display: flex;
  justify-content: center;
  align-items: center;
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
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 4px;
}

.time-display {
  font-size: 42px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  letter-spacing: 2px;
}
</style>
