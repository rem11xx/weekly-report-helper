import {
  getCurrentWindow,
  LogicalSize,
} from "@tauri-apps/api/window";

/**
 * 专注浮球模式的窗口操作封装（单窗口方案，不开新窗口）。
 *
 * 进入浮球：记下当前内尺寸（逻辑像素）→ 去标题栏 → 缩窗 → 强制置顶。
 * 退出浮球：恢复标题栏 → 还原原尺寸 → 按用户持久化偏好恢复置顶。
 *
 * 透明窗口（tauri.conf `transparent:true` + `macOSPrivateApi`）下，
 * 去标题栏后内容区透明，圆环呈悬浮球；CSS box-shadow 给球加阴影。
 *
 * 置顶不走后端 `set_always_on_top` 命令（那会写库、覆盖用户偏好），
 * 而是直接调前端 window API，仅在浮球期间临时生效，退出时按用户偏好恢复。
 */
// 半径 200 的圆环 SVG ≈ 428px + 圆盘 432px；窗口留余量给 box-shadow。
// 高度取 540 以匹配 tauri.conf 的 minHeight，避免被静默钳制。
const MINI_SIZE = new LogicalSize(520, 540);

let savedSize: LogicalSize | null = null;

/** 进入浮球态：去标题栏 + 缩窗 + 强制置顶 */
export async function enterMiniWindow() {
  const w = getCurrentWindow();
  if (!savedSize) {
    // 实时读取当前内尺寸（用户可能调整过窗口），转逻辑像素保存以便跨屏恢复
    const physical = await w.innerSize();
    const factor = await w.scaleFactor();
    savedSize = new LogicalSize(physical.width / factor, physical.height / factor);
  }
  await w.setDecorations(false);
  await w.setSize(MINI_SIZE);
  await w.setAlwaysOnTop(true);
}

/** 退出浮球态：恢复标题栏 + 原尺寸 + 用户置顶偏好 */
export async function restoreWindow(alwaysOnTop: boolean) {
  const w = getCurrentWindow();
  await w.setDecorations(true);
  if (savedSize) {
    await w.setSize(savedSize);
    savedSize = null;
  }
  await w.setAlwaysOnTop(alwaysOnTop);
}
