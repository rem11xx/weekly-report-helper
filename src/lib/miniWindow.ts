import {
  getCurrentWindow,
  LogicalSize,
  LogicalPosition,
} from "@tauri-apps/api/window";

/**
 * 专注浮球模式的窗口操作封装（单窗口方案，不开新窗口）。
 *
 * 进入浮球：记下当前内尺寸 + 常态位置 → 放宽最小尺寸 → 去标题栏 → 缩窗至 80×80
 *           → 强制置顶 → 移到浮球上次位置。
 * 退出浮球：记下浮球位置 → 恢复标题栏 → 还原最小尺寸与原尺寸 → 移到常态上次位置
 *           → 按用户持久化偏好恢复置顶。
 *
 * 透明窗口（tauri.conf `transparent:true` + `macOSPrivateApi`）下，去标题栏后
 * 内容区透明，仅圆环描边可见，呈悬浮球；CSS drop-shadow 给描边加层次。
 *
 * 位置记忆：常态/浮球各自记住上次的外框位置（逻辑坐标），切换时回到该态上次位置，
 * 而非在原地变形。位置在同一 decorations 态下捕获与恢复——常态位置在带标题栏时
 * 捕获/恢复，浮球位置在无标题栏时捕获/恢复——避免标题栏高度造成偏移。
 *
 * 置顶不走后端 `set_always_on_top` 命令（那会写库、覆盖用户偏好），
 * 而是直接调前端 window API，仅在浮球期间临时生效，退出时按用户偏好恢复。
 */
// 半径 50（直径 100）的圆环 SVG ≈ 128px，窗口取 140×140 留余量给 drop-shadow 与拖动热区。
const MINI_SIZE = new LogicalSize(140, 140);
const MINI_MIN = new LogicalSize(140, 140);
// 退出时恢复的最小尺寸，须与 tauri.conf.json 的 minWidth(420)/minHeight(540) 一致。
const NORMAL_MIN = new LogicalSize(420, 540);

let savedSize: LogicalSize | null = null;
// 各态独立记忆的外框位置（逻辑坐标）；首次进入某态时为 null，留在原地。
let normalPos: LogicalPosition | null = null;
let miniPos: LogicalPosition | null = null;

/** 读取当前外框位置，转逻辑像素以便跨屏恢复 */
async function currentLogicalPos(): Promise<LogicalPosition> {
  const w = getCurrentWindow();
  const physical = await w.outerPosition();
  const factor = await w.scaleFactor();
  return new LogicalPosition(physical.x / factor, physical.y / factor);
}

/** 进入浮球态：放宽最小尺寸 + 去标题栏 + 缩窗 + 强制置顶 + 回到浮球上次位置 */
export async function enterMiniWindow() {
  const w = getCurrentWindow();
  if (!savedSize) {
    // 实时读取当前内尺寸（用户可能调整过窗口），转逻辑像素保存以便跨屏恢复
    const physical = await w.innerSize();
    const factor = await w.scaleFactor();
    savedSize = new LogicalSize(physical.width / factor, physical.height / factor);
  }
  // 常态位置：此时仍带标题栏，与退出时恢复态一致
  normalPos = await currentLogicalPos();
  // 先放宽最小尺寸，否则 setSize(80,80) 会被 tauri.conf 的 minHeight 540 钳制
  await w.setMinSize(MINI_MIN);
  await w.setDecorations(false);
  await w.setSize(MINI_SIZE);
  await w.setAlwaysOnTop(true);
  // 回到浮球上次位置（首次进入则留在原地不动）
  if (miniPos) {
    await w.setPosition(miniPos);
  }
}

/** 退出浮球态：恢复标题栏 + 原最小尺寸与原尺寸 + 回到常态上次位置 + 用户置顶偏好 */
export async function restoreWindow(alwaysOnTop: boolean) {
  const w = getCurrentWindow();
  // 浮球位置：此时已无标题栏，与进入时恢复态一致
  miniPos = await currentLogicalPos();
  await w.setDecorations(true);
  await w.setMinSize(NORMAL_MIN);
  if (savedSize) {
    await w.setSize(savedSize);
    savedSize = null;
  }
  await w.setAlwaysOnTop(alwaysOnTop);
  // 回到常态上次位置（首次退出若无常态记录则留在原地）
  if (normalPos) {
    await w.setPosition(normalPos);
  }
}
