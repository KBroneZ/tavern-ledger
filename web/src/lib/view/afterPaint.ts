// Resolves once the page's first paint is out. A page script that needs the
// network or a big module waits for it, so the words are drawn first and the
// requests do not compete with the styles and fonts (T-554).

// The page gives up waiting for the paint report after this long (a browser
// that never reports it, or a page that paints nothing).
const GIVE_UP_MS = 2000;

function paintTimingSupported(): boolean {
  return typeof PerformanceObserver !== "undefined" && PerformanceObserver.supportedEntryTypes?.includes("paint") === true;
}

export function afterPaint(): Promise<void> {
  // A tab opened in the background paints nothing yet (and pauses animation
  // frames): it starts at once.
  if (document.visibilityState === "hidden") return Promise.resolve();
  // Without paint timing: the task after the next animation frame.
  if (!paintTimingSupported()) {
    return new Promise((done) => requestAnimationFrame(() => setTimeout(done, 0)));
  }
  return new Promise((done) => {
    let observer: PerformanceObserver | null = null;
    const finish = (): void => {
      clearTimeout(giveUp);
      observer?.disconnect();
      // One task later, so the request starts after the paint was reported.
      setTimeout(done, 0);
    };
    const giveUp = setTimeout(finish, GIVE_UP_MS);
    observer = new PerformanceObserver((list) => {
      if (list.getEntriesByName("first-contentful-paint").length > 0) finish();
    });
    observer.observe({ type: "paint", buffered: true });
  });
}
