// Changelog: every release's notes from /api/v1/releases (as on Screvia's site). notes_html is rendered by
// ./build.sh release from CHANGELOG.md with everything HTML-escaped and only http(s) or site links kept.
"use strict";

(() => {
  const { el, formatDate, getJson, compareVersions } = Plexdio;

  async function load() {
    const host = document.getElementById("changelog");
    try {
      const releases = (await getJson("/api/v1/releases")) || [];
      host.replaceChildren();
      if (releases.length === 0) {
        host.append(el("div", { class: "empty" },
          el("h2", { text: "No releases yet" }),
          el("p", { text: "Release notes appear here as soon as the first version of Plexdio is published on this site." })));
        return;
      }
      const seen = new Map();
      for (const r of releases) if (!seen.has(r.version)) seen.set(r.version, r); // one entry per version
      const list = [...seen.values()].sort((a, b) => compareVersions(b.version, a.version));
      for (const r of list) {
        const notes = el("div", { class: "notes" });
        notes.innerHTML = r.notes_html || "<p>No notes for this release.</p>";
        host.append(el("article", { class: "release-entry", id: "v" + r.version },
          el("header", {},
            el("h2", {}, el("a", { href: "#v" + r.version, text: "Plexdio " + r.version })),
            el("span", { class: "badge " + (r.channel === "beta" ? "badge-beta" : "badge-stable"), text: r.channel }),
            el("time", { datetime: r.released_at, text: formatDate(r.released_at) }),
          ),
          notes,
          el("p", { class: "small" }, el("a", { href: "/download", text: "Download" }), " · ",
            el("a", { href: "/downloads/" + encodeURIComponent(r.version) + "/SHA256SUMS", text: "SHA256SUMS" })),
        ));
      }
      // Jump to #v1.2.3 once the entries exist.
      if (location.hash) {
        const target = document.getElementById(decodeURIComponent(location.hash.slice(1)));
        if (target) target.scrollIntoView();
      }
    } catch (e) {
      host.replaceChildren(el("p", { class: "notice error", text: "The release notes could not be loaded. Please try again in a moment." }));
    }
  }

  document.addEventListener("DOMContentLoaded", load);
})();
