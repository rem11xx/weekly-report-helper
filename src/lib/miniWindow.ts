import {
  getCurrentWindow,
  LogicalSize,
} from "@tauri-apps/api/window";

/**
 * 专注浮球模式的窗口操作封装（单窗口方案，不开新窗口）。
 *
 * 进入浮球：记下当前内尺寸 → 放宽最小尺寸 → 去标题栏 → 缩窗至 80×80 → 强制置顶。
 * 退出浮球：恢复标题栏 → 还原最小尺寸与原尺寸 → 按用户持久化偏好恢复置顶。
 *
 * 透明窗口（tauri.conf `transparent:true` + `macOSPrivateApi`）下，去标题栏后
 * 内容区透明，仅圆环描边可见，呈悬浮球；CSS drop-shadow 给描边加层次。
 *
 * 置顶不走后端 `set_always_on_top` 命令（那会写库、覆盖用户偏好），
 * 而是直接调前端 window API，仅在浮球期间临时生效，退出时按用户偏好恢复。
 */
// 半径 20 的圆环 SVG ≈ 68px，窗口取 80×80 留余量给 drop-shadow 与拖动热区。
const MINI_SIZE = new LogicalSize(80, 80);
const MINI_MIN = new LogicalSize(80, 80);
// 退出时恢复的最小尺寸，须与 tauri.conf.json 的 minWidth(420)/minHeight(540) 一致。
const NORMAL_MIN = new LogicalSize(420, 540);

let savedSize: LogicalSize | null = null;

/** 进入浮球态：放宽最小尺寸 + 去标题栏 + 缩窗 + 强制置顶 */
export async function enterMiniWindow() {
  const w = getCurrentWindow();
  if (!savedSize) {
    // 实时读取当前内尺寸（用户可能调整过窗口），转逻辑像素保存以便跨屏恢复
    const physical = await w.innerSize();
    const factor = await w.scaleFactor();
    savedSize = new LogicalSize(physical.width / factor, physical.height / factor);
  }
  // 先放宽最小尺寸，否则 setSize(80,80) 会被 tauri.conf 的 minHeight 540 钳制
  await w.setMinSize(MINI_MIN);
  await w.setDecorations(false);
  await w.setSize(MINI_SIZE);
  await w.setAlwaysOnTop(true);
}

/** 退出浮球态：恢复标题栏 + 原最小尺寸与原尺寸 + 用户置顶偏好 */
export async function restoreWindow(alwaysOnTop: boolean) {
  const w = getCurrentWindow();
  await w.setDecorations(true);
  await w.setMinSize(NORMAL_MIN);
  if (savedSize) {
    await w.setSize(savedSize);
    savedSize = null;
  }
  await w.setAlwaysOnTop(alwaysOnTop);
}
