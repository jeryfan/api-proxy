export const isMac = () => navigator.userAgent.includes("Mac");
export const isWindows = () => navigator.userAgent.includes("Windows");
export const isLinux = () => navigator.userAgent.includes("Linux") && !isMac();

export const DRAG_REGION_ENABLED = !isLinux();
