// The error a page shows when its own script could not be loaded (a chunk
// that moved after a deploy, or no network): a visible block with a reload
// button, in the page's own place. Plain DOM and no imports, because the
// script that would normally draw it is the one that failed (T-554).

export function showLoadFailure(target: HTMLElement, title: string, body: string, retry: string): void {
  const block = document.createElement("div");
  block.className = "state state-error";
  const heading = document.createElement("p");
  heading.className = "state-title";
  heading.textContent = title;
  block.append(heading);
  if (body !== "") {
    const text = document.createElement("p");
    text.textContent = body;
    block.append(text);
  }
  const button = document.createElement("button");
  button.type = "button";
  button.className = "tb tb-small";
  button.textContent = retry;
  button.addEventListener("click", () => location.reload());
  block.append(button);
  target.replaceChildren(block);
}
