import "@testing-library/jest-dom";

// jsdom 未实现 matchMedia，而 themeStore 在初始化时会调用它（读取系统暗色偏好并监听变化）。
if (typeof window.matchMedia !== "function") {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}
