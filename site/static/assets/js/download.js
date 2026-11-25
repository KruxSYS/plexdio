// Download page: the latest release from the update manifests (/api/v1/version, the same answer the app gets),
// all releases from /api/v1/releases. Shows a clear empty state while nothing is published.
"use strict";

(() => {
  const { el, formatBytes, formatDate, getJson, copyButton, compareVersions } = Plexdio;

  const PACKAGES = {
    "x64-setup": {
      title: "Installer",
      points: ["Start menu shortcut and uninstaller", "Installs for your user, no admin rights", "Updates itself silently"],
    },
    "x64": {
      title: "Portable zip",
      points: ["Runs from any folder, nothing installed", "Settings and cache in Data\\ next to the exe", "Updates by replacing its exe"],
    },
  };

  function fileName(info) {
    return info.file_name || decodeURIComponent(String(info.download_url).split("/").pop());
  }

  function packageCard(info) {
    const pkg = PACKAGES[info.arch];
    const name = fileName(info);
    const verify = `(Get-FileHash .\\${name} -Algorithm SHA256).Hash.ToLower() -eq "${info.sha256}"`;
    return el("article", { class: "card release-card" },
      el("div", {},
        el("h3", {}, pkg.title, info.arch === "x64-setup" ? el("span", { class: "badge", text: "Recommended" }) : null),
        el("ul", { class: "checklist" }, pkg.points.map((p) => el("li", { text: p }))),
        el("dl", { class: "facts" },
          el("dt", { text: "File" }), el("dd", {}, el("code", { text: name })),
          el("dt", { text: "Size" }), el("dd", { text: formatBytes(info.size) + " · Windows 10/11 x64" }),
          el("dt", { text: "SHA-256" }), el("dd", {}, el("div", { class: "hash" }, el("code", { text: info.sha256 }), copyButton(info.sha256))),
        ),
        el("details", {},
          el("summary", { text: "Verify the download (PowerShell)" }),
          el("pre", {}, el("code", { text: verify })),
          el("p", { class: "small muted", text: "Prints True when the file matches. Plexdio checks its updates the same way before installing them." }),
        ),
      ),
      el("div", { class: "actions-row" },
        el("a", { class: "btn " + (info.arch === "x64-setup" ? "btn-primary" : "btn-secondary"), href: info.download_url, download: "" }, "Download " + pkg.title.toLowerCase()),
      ),
    );
  }

  function emptyState() {
    return el("div", { class: "empty" },
      el("h2", { text: "No release published yet" }),
      el("p", { text: "Plexdio is in development and no version has been published on this site yet. This page shows the installer and the portable zip as soon as the first release is out." }),
      el("p", {}, "Meanwhile, see the ", el("a", { href: "/changelog", text: "changelog" }), " or the ",
        el("a", { href: "https://kruxsys.com/projects/plexdio/", text: "project page" }), "."),
    );
  }

  function errorState() {
    return el("p", { class: "notice error", text: "The release information could not be loaded. Please try again in a moment." });
  }

  async function latest() {
    const host = document.getElementById("latest");
    try {
      // Stable first; a beta-only history shows the beta.
      let channel = "stable";
      let [setup, zip] = await Promise.all([
        getJson("/api/v1/version?channel=stable&arch=x64-setup"),
        getJson("/api/v1/version?channel=stable&arch=x64"),
      ]);
      if (!setup && !zip) {
        channel = "beta";
        [setup, zip] = await Promise.all([
          getJson("/api/v1/version?channel=beta&arch=x64-setup"),
          getJson("/api/v1/version?channel=beta&arch=x64"),
        ]);
      }
      const infos = [setup, zip].filter(Boolean);
      if (infos.length === 0) {
        host.replaceChildren(emptyState());
        return;
      }
      const head = infos.reduce((a, b) => (compareVersions(a.version, b.version) >= 0 ? a : b));
      const badge = el("span", { class: "badge " + (head.channel === "beta" ? "badge-beta" : "badge-stable"), text: head.channel || channel });
      host.replaceChildren(
        el("div", { class: "release-head" },
          el("h2", { class: "version" }, "Plexdio " + head.version + " ", badge),
          el("p", { class: "muted" },
            "Released ", el("time", { datetime: head.released_at, text: formatDate(head.released_at) }),
            " · ", el("a", { href: "/changelog#v" + head.version, text: "Release notes" }),
            " · ", el("a", { href: "#all", text: "Older releases" })),
        ),
        el("div", { class: "grid grid-2" }, infos.map(packageCard)),
      );
    } catch (e) {
      host.replaceChildren(errorState());
    }
  }

  async function all() {
    const host = document.getElementById("all-releases");
    try {
      const releases = (await getJson("/api/v1/releases")) || [];
      if (releases.length === 0) {
        host.replaceChildren(el("p", { class: "muted", text: "No releases yet." }));
        return;
      }
      const byVersion = new Map();
      for (const r of releases) {
        if (!byVersion.has(r.version)) byVersion.set(r.version, []);
        byVersion.get(r.version).push(r);
      }
      const versions = [...byVersion.keys()].sort((a, b) => compareVersions(b, a));
      const rows = versions.map((v) => {
        const files = byVersion.get(v).sort((a, b) => (a.arch < b.arch ? 1 : -1));
        const first = files[0];
        return el("tr", {},
          el("td", {}, el("a", { href: "/changelog#v" + v, text: v })),
          el("td", {}, el("span", { class: "badge " + (first.channel === "beta" ? "badge-beta" : "badge-stable"), text: first.channel })),
          el("td", {}, el("time", { datetime: first.released_at, text: formatDate(first.released_at) })),
          el("td", {}, files.flatMap((f, i) => [i ? " · " : "", el("a", { href: f.download_url, download: "", text: PACKAGES[f.arch] ? PACKAGES[f.arch].title : f.arch }), " ", el("span", { class: "small muted", text: "(" + formatBytes(f.size) + ")" })])),
          el("td", {}, el("a", { href: "/downloads/" + encodeURIComponent(v) + "/SHA256SUMS", text: "SHA256SUMS" })),
        );
      });
      host.replaceChildren(el("div", { class: "table-wrap" }, el("table", {},
        el("thead", {}, el("tr", {}, ["Version", "Channel", "Released", "Files", "Checksums"].map((h) => el("th", { scope: "col", text: h })))),
        el("tbody", {}, rows),
      )));
    } catch (e) {
      host.replaceChildren(errorState());
    }
  }

  document.addEventListener("DOMContentLoaded", () => { latest(); all(); });
})();
