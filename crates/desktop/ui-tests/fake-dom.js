// Just enough of a page for focus-keeper.js and tabs.js: elements with an id,
// attributes, a parent, `hidden`/`disabled`, focus and event listeners. Not a
// browser; no dependency.
"use strict";

class FakeNode {
  constructor(doc, tag, attrs = {}) {
    this.doc = doc;
    this.tag = tag;
    this.attrs = { ...attrs };
    this.id = attrs.id || "";
    this.hidden = false;
    this.disabled = false;
    this.parent = null;
    this.children = [];
    this.isConnected = true;
    this.listeners = {};
  }

  setAttribute(name, value) {
    this.attrs[name] = String(value);
    if (name === "id") this.id = String(value);
  }

  removeAttribute(name) {
    delete this.attrs[name];
  }

  addEventListener(type, fn) {
    (this.listeners[type] = this.listeners[type] || []).push(fn);
  }

  // Runs the listeners of this node and its parents, like a bubbling event.
  dispatch(type, extra = {}) {
    const event = { type, target: this, defaultPrevented: false, preventDefault() { this.defaultPrevented = true; }, ...extra };
    for (let n = this; n; n = n.parent) (n.listeners[type] || []).forEach((fn) => fn(event));
    (this.doc.listeners[type] || []).forEach((fn) => fn(event));
    return event;
  }

  append(child) {
    child.parent = this;
    this.children.push(child);
    return child;
  }

  // A redraw: the old children are gone for good, the new ones are connected.
  replaceChildren(...next) {
    const drop = (node) => {
      node.isConnected = false;
      node.children.forEach(drop);
    };
    this.children.forEach(drop);
    this.children = [];
    next.forEach((n) => this.append(n));
    if (this.doc.activeElement && !this.doc.activeElement.isConnected) this.doc.activeElement = this.doc.body;
  }

  getAttribute(name) {
    return name in this.attrs ? this.attrs[name] : null;
  }

  closest(selector) {
    for (let n = this; n; n = n.parent) {
      if (selector === "[hidden]" && n.hidden) return n;
      if (selector === "[data-focus-scope]" && "data-focus-scope" in n.attrs) return n;
    }
    return null;
  }

  // Descendants that can take focus (buttons, inputs, anything with a tabindex).
  querySelectorAll(selector) {
    const all = [];
    const walk = (n) => n.children.forEach((c) => (all.push(c), walk(c)));
    walk(this);
    if (selector === "*") return all;
    if (selector === "[data-focus]") return all.filter((n) => "data-focus" in n.attrs);
    const attr = /^\[([a-z-]+)="([^"]*)"\]$/.exec(selector);
    if (attr) return all.filter((n) => n.attrs[attr[1]] === attr[2]);
    return all.filter((n) => ["button", "input", "select", "textarea"].includes(n.tag) || "tabindex" in n.attrs);
  }

  focus() {
    if (!this.isConnected || this.disabled) return;
    this.doc.activeElement = this;
    this.doc.focusCalls += 1;
  }
}

class FakeDocument {
  constructor() {
    this.focusCalls = 0;
    this.body = new FakeNode(this, "body");
    this.documentElement = new FakeNode(this, "html");
    this.activeElement = this.body;
    this.listeners = {};
  }

  addEventListener(type, fn) {
    (this.listeners[type] = this.listeners[type] || []).push(fn);
  }

  make(tag, attrs, parent = this.body) {
    return parent.append(new FakeNode(this, tag, attrs));
  }

  getElementById(id) {
    return this.body.querySelectorAll("*").find((n) => n.id === id) || null;
  }

  querySelectorAll(selector) {
    return this.body.querySelectorAll(selector);
  }
}

module.exports = { FakeDocument, FakeNode };
