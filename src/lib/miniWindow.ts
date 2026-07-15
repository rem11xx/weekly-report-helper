import {
  getCurrentWindow,
  LogicalSize,
  LogicalPosition,
} from "@tauri-apps/api/window";
import { getWindowPositions, setWindowPositions, getAlwaysOnTop } from "@/api";

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
 * 位置记忆（跨重启持久化到 app_settings.window_positions）：
 * 常态/浮球各自记住上次的外框位置（逻辑坐标），切换时回到该态上次位置而非在原地变形。
 * 位置在同一 decorations 态下捕获与恢复——常态位置在带标题栏时捕获/恢复，
 * 浮球位置在无标题栏时捕获/恢复——避免标题栏高度造成偏移。
 * 常态位置由后端 setup 钩子启动时恢复（首帧前，无闪烁，带显示器内校验）；
 * 浮球位置由前端 initWindowPositions 启动加载、进入浮球时使用、退出浮球时写回。
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

/** 把当前两态位置整体写回后端（捕获位置后调用，失败仅记日志不阻断） */
async function persistPositions() {
  try {
    await setWindowPositions({
      normal: normalPos ? { x: normalPos.x, y: normalPos.y } : null,
      mini: miniPos ? { x: miniPos.x, y: miniPos.y } : null,
    });
  } catch (e) {
    console.error("持久化窗口位置失败", e);
  }
}

/** 启动时加载上次窗口位置：常态位置已由后端 setup 钩子恢复，此处取浮球位置备用 */
export async function initWindowPositions() {
  try {
    const p = await getWindowPositions();
    if (p.normal) normalPos = new LogicalPosition(p.normal.x, p.normal.y);
    if (p.mini) miniPos = new LogicalPosition(p.mini.x, p.mini.y);
  } catch (e) {
    console.error("读取窗口位置失败", e);
  }
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
  void persistPositions(); // 持久化常态位置，供下次启动恢复
  // 先放宽最小尺寸，否则 setSize(80,80) 会被 tauri.conf 的 minHeight 540 钳制
  await w.setMinSize(MINI_MIN);
  await w.setDecorations(false);
  await w.setSize(MINI_SIZE);
  await w.setAlwaysOnTop(true);
  // 回到浮球上次位置（首次进入则留在原地不动）。
  // 位置恢复为非关键：失败仅记日志不外抛，避免浮球态已缩窗却因移位失败被上层
  // catch 回 miniMode=false，造成"小窗 + 全界面"的破损态。
  if (miniPos) {
    try {
      await w.setPosition(miniPos);
    } catch (e) {
      console.error("恢复浮球位置失败，留在原地", e);
    }
  }
}

/** 退出浮球态：恢复标题栏 + 原最小尺寸与原尺寸 + 回到常态上次位置 + 用户置顶偏好 */
export async function restoreWindow(alwaysOnTop: boolean) {
  const w = getCurrentWindow();
  // 浮球位置：此时已无标题栏，与进入时恢复态一致
  miniPos = await currentLogicalPos();
  void persistPositions(); // 持久化浮球位置，供下次进入浮球恢复
  await w.setDecorations(true);
  await w.setMinSize(NORMAL_MIN);
  if (savedSize) {
    await w.setSize(savedSize);
    savedSize = null;
  }
  await w.setAlwaysOnTop(alwaysOnTop);
  // 回到常态上次位置（首次退出若无常态记录则留在原地）。位置恢复非关键，失败仅记日志。
  if (normalPos) {
    try {
      await w.setPosition(normalPos);
    } catch (e) {
      console.error("恢复常态窗口位置失败，留在原地", e);
    }
  }
}

/** 专注结束时把主窗口「置顶一次」以提醒填写任务弹窗。
 *  临时 setAlwaysOnTop(true) 把窗口抬到 Z 序最顶（绕过 Windows 前台锁，比
 *  setFocus 可靠--后者从后台抢焦点常被降级为任务栏闪烁），随后恢复用户持久化
 *  的置顶偏好：偏好为 true 时是 no-op，偏好为 false 时窗口留在普通窗口最前
 *  而非持久置顶，即「置顶一次」效果，不污染用户的置顶设置。
 *  偏好取后端 getAlwaysOnTop() 而非 timer store 的 savedOnTop--后者仅在
 *  enterMini() 赋值，本次专注若未进浮球则可能是上一轮的过期值。
 *  失败仅记日志，不阻断弹窗流程。 */
export async function bringToFrontOnce() {
  try {
    const w = getCurrentWindow();
    const userPref = await getAlwaysOnTop();
    await w.setAlwaysOnTop(true);
    await w.setAlwaysOnTop(userPref);
  } catch (e) {
    console.error("置顶提醒失败", e);
  }
}
