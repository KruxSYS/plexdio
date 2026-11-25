// Shared helpers for all pages (as on Screvia's site): current-page highlight, formatting, JSON access.
"use strict";

const Plexdio = (() => {
  function markCurrentPage() {
    const path = location.pathname.replace(/\.html$/, "").replace(/\/$/, "") || "/";
    document.querySelectorAll(".nav a").forEach((a) => {
      const href = a.getAttribute("href").replace(/\/$/, "") || "/";
      if (href === path) a.setAttribute("aria-current", "page");
    });
  }

  function formatBytes(n) {
    if (!Number.isFinite(n)) return "";
    const units = ["B", "KB", "MB", "GB"];
    let i = 0;
    while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
    return (i === 0 ? n : n.toFixed(1)) + " " + units[i];
  }

  function formatDate(iso) {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso || "";
    return d.toLocaleDateString("en-GB", { year: "numeric", month: "long", day: "numeric", timeZone: "UTC" });
  }

  // null on 404 (nothing published), throws on other failures.
  async function getJson(url) {
    const response = await fetch(url, { headers: { Accept: "application/json" }, cache: "no-cache" });
    if (response.status === 404) return null;
    if (!response.ok) throw new Error("HTTP " + response.status);
    return response.json();
  }

  function el(tag, attrs, ...children) {
    const node = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs || {})) {
      if (v === null || v === undefined || v === false) continue;
      if (k === "class") node.className = v;
      else if (k === "text") node.textContent = v;
      else node.setAttribute(k, v === true ? "" : v);
    }
    for (const c of children.flat()) {
      if (c === null || c === undefined) continue;
      node.append(c instanceof Node ? c : document.createTextNode(String(c)));
    }
    return node;
  }

  async function copyText(text, button) {
    try {
      await navigator.clipboard.writeText(text);
      if (button) {
        const old = button.textContent;
        button.textContent = "Copied";
        setTimeout(() => { button.textContent = old; }, 1500);
      }
    } catch (e) {
      window.prompt("Copy:", text);
    }
  }

  function copyButton(text, label) {
    const b = el("button", { type: "button", class: "btn btn-secondary btn-small", text: label || "Copy" });
    b.addEventListener("click", () => copyText(text, b));
    return b;
  }

  // Semantic version comparison (pre-releases sort below their release).
  function compareVersions(a, b) {
    const parse = (v) => {
      const [core, pre] = String(v).split("+")[0].split(/-(.*)/s);
      return { nums: core.split(".").map((n) => parseInt(n, 10) || 0), pre: pre ? pre.split(".") : null };
    };
    const x = parse(a), y = parse(b);
    for (let i = 0; i < 3; i++) if (x.nums[i] !== y.nums[i]) return x.nums[i] - y.nums[i];
    if (!x.pre || !y.pre) return (x.pre ? -1 : 0) - (y.pre ? -1 : 0);
    for (let i = 0; i < Math.max(x.pre.length, y.pre.length); i++) {
      const p = x.pre[i], q = y.pre[i];
      if (p === undefined) return -1;
      if (q === undefined) return 1;
      const pn = /^\d+$/.test(p), qn = /^\d+$/.test(q);
      if (pn && qn && +p !== +q) return +p - +q;
      if (pn !== qn) return pn ? -1 : 1;
      if (p !== q) return p < q ? -1 : 1;
    }
    return 0;
  }

  // Landing page: "Latest: 1.2.3" when a stable release exists.
  async function showLatestVersion() {
    const target = document.getElementById("latest-version");
    if (!target) return;
    try {
      const info = await getJson("/api/v1/version?channel=stable&arch=x64-setup");
      if (info) target.textContent = "Latest: " + info.version + " · Windows 10/11 x64";
    } catch (e) { /* keep the static text */ }
  }

  document.addEventListener("DOMContentLoaded", () => {
    markCurrentPage();
    document.querySelectorAll("[data-year]").forEach((n) => { n.textContent = String(new Date().getFullYear()); });
    showLatestVersion();
  });

  return { formatBytes, formatDate, getJson, el, copyText, copyButton, compareVersions };
})();
