// Resolves once the page's first paint is out. A page script that needs the
// network or a big module waits for it, so the words are drawn first and the
// requests do not compete with the styles and fonts (T-554).

// The page gives up waiting for the paint report after this long (a browser
// that never reports it, or a page that paints nothing).
const GIVE_UP_MS = 2000;

export function afterPaint(): Promise<void> {
  // A tab opened in the background paints nothing yet (and pauses animation
  // frames): it starts at once.
  if (document.visibilityState === "hidden") return Promise.resolve();
  return new Promise((done) => {
    const go = (): void => {
      clearTimeout(giveUp);
      // One task later, so the request starts after the paint was reported.
      setTimeout(done, 0);
    };
    const giveUp = setTimeout(done, GIVE_UP_MS);
    try {
      new PerformanceObserver((list, observer) => {
        if (list.getEntriesByName("first-contentful-paint").length === 0) return;
        observer.disconnect();
        go();
      }).observe({ type: "paint", buffered: true });
    } catch {
      requestAnimationFrame(() => go());
    }
  });
}
