# Plexdio brand (KruxSYS)

Plexdio is a KruxSYS product. This file records where the app's colours, type and marks come from,
so they can be checked against the source and updated together.

Sources were fetched on 2026-11-24 with curl (raw HTML, CSS and image files, nothing executed):

| Source | URL | File (sha256 prefix) |
|---|---|---|
| KruxSYS home | https://kruxsys.com/ | |
| Plexdio project page | https://kruxsys.com/projects/plexdio/ | |
| Krunk project page | https://kruxsys.com/projects/krunk/ | |
| Krunk site (style reference only) | https://krunk.kruxsys.com/ | |
| KruxSYS stylesheet (shared by all kruxsys.com pages) | https://kruxsys.com/assets/css/site.3025c6de38.css | |
| Krunk stylesheet | https://krunk.kruxsys.com/style.css?v=c2fbd582f9 | |
| KruxSYS logo | https://kruxsys.com/assets/brand/logo.ce36e02f37.svg | `ce36e02f37…`, copied byte-for-byte to [`assets/brand/kruxsys.svg`](../assets/brand/kruxsys.svg) |
| Plexdio logo on the site | https://kruxsys.com/assets/img/plexdio/logo-128.2c314065b5.webp | not used, see *Logo provenance* |

Below, "site.css" means the KruxSYS stylesheet and "krunk.css" the Krunk one. Line numbers refer to
those files as fetched.

## Colour tokens

### From the KruxSYS stylesheet

| Site token | Value | site.css | What the site uses it for | App token (`ui/theme.rs`) |
|---|---|---|---|---|
| `--bg` | `#07090c` | l. 5 (also `<meta name="theme-color">`) | page background | `SIDEBAR` (sidebar, title bar, now-playing bar) |
| `--panel` | `#0c1016` | l. 6 | panels, cards, secondary buttons | `BG` (content area), `BAR` (windows, menus, popups) |
| `--panel-2` | `#111620` | l. 7 | kbd, version chip | `CARD` |
| `--line` | `#1a212c` | l. 8 | panel and card borders, header rule | `LINE`, `CARD_HOVER`, `CONTROL` |
| `--line-2` | `#2a3442` | l. 9 | button borders, section rules | `LINE_2`, `CONTROL_HOVER` |
| `--text` | `#e8ecf2` | l. 10 | body text | `TEXT` |
| `--muted` | `#8e9bb0` | l. 11 | secondary text, nav | `TEXT_DIM` |
| (literal) | `#c9d1dd` | l. 948 (`.prose p`) | long-form body text | `TEXT_SOFT` |
| `--signal` | `#22d3ee` | l. 12 | accent: links, focus ring, brackets, "stable" LED, primary button | `ACCENT` |
| (literal) | `#67e8f9` | l. 872 (`.button.primary:hover`) | primary button hover | `ACCENT_HOVER` |
| (literal) | `#031014` | l. 135, l. 868 (`.skip`, `.button.primary`) | text on cyan | `ON_ACCENT` |
| `--signal-dim` | `rgba(34, 211, 238, 0.14)` | l. 14 | badge fill | `ACCENT_DIM` |
| `--amber` | `#ffb547` | l. 13 | "beta" / "in development" status LEDs and badges | not used in the app |
| `--amber-dim` | `rgba(255, 181, 71, 0.12)` | l. 15 | beta badge fill | not used |
| (literal) | `#e9dcc6` | l. 1000 (`.caveats li`) | caveat list text | not used |
| `--grid` | `rgba(255, 255, 255, 0.012)` | l. 16 | background grid | not used |

The app does not use `--amber`. Plexdio's old identity was a Plex amber, `rgb(229, 160, 13)`. A second
amber in the UI would blur the rebrand, and the site only uses its amber for release-status markers.

### Derived values

| App token | Value | How |
|---|---|---|
| `SELECTED` | `#0e242c` | `--signal` at 10 % over `--panel`: the site's `--signal-dim` idea, made opaque for track rows. Selected rows and the active nav item also get a 2 px cyan bar on the left, so the state never relies on the faint fill alone. |
| `ACCENT_DIM` | `#0f2b34` | `--signal-dim` (14 % cyan) composited over `--panel` |
| icon edge | `#3a4658` | border of the 16 and 24 px icon frames only: one step lighter than `--line-2`, so the tile keeps an edge on the dark Windows taskbar (about `#1c1c1c`) |

### Not in the KruxSYS stylesheet

kruxsys.com has no error or success colours. The app needs both: failed sign-in, the destructive
dialog button and the server-online dot. They come from the dark scheme of the sister product's site,
which is used as a style reference only:

| App token | Value | Source |
|---|---|---|
| `ERROR` | `#ff8a80` | krunk.css l. 43, dark `--danger` |
| `ON_ERROR` | `#2a0716` | krunk.css l. 38, dark `--on-primary` (dark text on a light red/pink fill) |
| `OK` | `#6fdc95` | krunk.css l. 44, dark `--ok` |

One colour is neither brand nor reference:

| App token | Value | Source |
|---|---|---|
| `CLOSE_HOVER` | `#c42b1c` | Windows 11 caption close-button hover. Users expect it, so it stays. |

### Logo SVG palette

The logo SVG (`assets/brand/kruxsys.svg`) uses `#0c1016` fill, `#2a3442` border, `#22d3ee` accent and
`#e8ecf2` light strokes. All four are stylesheet tokens above (`--panel`, `--line-2`, `--signal`, `--text`).

## Contrast

WCAG 2 contrast ratios, all at AA (4.5:1) or better for text:

| Foreground | Background | Ratio |
|---|---|---|
| `ON_ACCENT` `#031014` | `ACCENT` `#22d3ee` | 10.7 |
| `ON_ACCENT` | `ACCENT_HOVER` `#67e8f9` | 13.3 |
| `TEXT` `#e8ecf2` | `SIDEBAR` `#07090c` / `BG` `#0c1016` / `CARD` `#111620` | 16.8 / 16.1 / 15.3 |
| `TEXT` | `CONTROL` `#1a212c` | 13.7 |
| `TEXT_DIM` `#8e9bb0` | `SIDEBAR` / `BG` / `CARD` / `CONTROL` | 7.1 / 6.8 / 6.4 / 5.75 |
| `ACCENT` | `BG` / `CARD` / `CONTROL` | 10.6 / 10.0 / 9.0 |
| `ERROR` `#ff8a80` | `BG` | 8.4 |
| `ON_ERROR` `#2a0716` | `ERROR` | 8.1 |
| `OK` `#6fdc95` | `BG` | 11.2 |
| `TEXT` / `TEXT_DIM` / `ACCENT` | `SELECTED` `#0e242c` | 13.7 / 5.8 / 9.0 |
| `TEXT` / `ACCENT` | `CONTROL_HOVER` `#2a3442` | 10.6 / 7.0 |
| `TEXT` / `ACCENT` | `ACCENT_DIM` `#0f2b34` | 12.5 / 8.2 |

Two pairs are below AA, so the app never uses them:

- `TEXT_DIM` on `CONTROL_HOVER` is 4.47. Hovered rows and cards use `CARD_HOVER` (`#1a212c`, 5.75 with
  `TEXT_DIM`), and `CONTROL_HOVER` only fills hovered or pressed controls whose label is `TEXT`.
- `ON_ACCENT` on `ACCENT_DIM` is 1.3. Dim-cyan fills always carry `TEXT` or `ACCENT`.

`LINE_2` on `BG` is only 1.5:1. That is fine because it is a decorative border and never carries
information on its own.

## Type

The stylesheet has no `@font-face` and loads no web fonts. It uses system stacks only (site.css
l. 18–19):

- `--sans: system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans", sans-serif`
- `--mono: ui-monospace, "SFMono-Regular", "Cascadia Mono", "Segoe UI Mono", Menlo, Consolas, "Liberation Mono", monospace`

So there is no site font to bundle. Plexdio keeps the fonts it already embeds: egui's bundled
proportional and monospace fonts, plus the existing system-font fallback for CJK and symbols. It
mirrors the site's split by using the monospace face for small technical labels: the cyan **SYS** of
the KruxSYS wordmark and the build line in Settings > About.

Headings on the site are weight 650 with tight tracking (site.css l. 70–75). The app's headings stay
as they are.

## Shape

- **Corners are square.** site.css has no `border-radius` except `50%` on status LEDs and badge dots
  (l. 346, l. 390). Panels, cards and buttons are sharp. The app uses `RADIUS = 2` for controls and
  `0` for panels and cards. Round controls stay round only where roundness means something: the big
  play buttons, the toggle knob path and the slider handles.
- **Borders are thin.** They are 1 px `--line` on panels and cards, and 1 px `--line-2` on buttons.
- **Corner brackets are the motif.** On `.panel` and `.card` (site.css l. 282–315), a 12 px cyan
  bracket sits on the top-left and bottom-right corners. The app draws them on the login card and the
  About card.
- **Wordmark:** "Krux" is bold in the text colour, and "SYS" is cyan monospace (site.css `.wordmark`,
  `.accent`).

## Marks

| File | What |
|---|---|
| [`assets/brand/kruxsys.svg`](../assets/brand/kruxsys.svg) | KruxSYS logo, the official SVG unchanged |
| [`assets/brand/plexdio.svg`](../assets/brand/plexdio.svg) | Plexdio logo, the master for all sizes from 32 px up |
| [`assets/brand/plexdio-small.svg`](../assets/brand/plexdio-small.svg) | Plexdio logo, pixel-aligned variant for the 16 and 24 px icon frames |

`tools/make_icons.py` renders the Plexdio SVGs into `assets/icon.ico` (16/24/32/48/64/256),
`assets/icon.png`, `assets/icon64.rgba` and `assets/discord-icon.png`. The in-app marks (title bar,
login, About, credit) are drawn as vector shapes in `crates/plexdio/src/ui/brand.rs` to the same
geometry, so they stay sharp at every zoom level.

## Logo provenance

The official site does not provide a usable Plexdio logo:

- **The site's logo is the old amber icon.** The only Plexdio logo on kruxsys.com is
  `assets/img/plexdio/logo-128.2c314065b5.webp`, 128 px. There is no SVG and no larger size. It is the
  old Plex-amber icon: an amber `rgb(229, 160, 13)` disc with a dark `rgb(24, 24, 24)` play triangle.
  Decoded, it matches the 128 px render of the old `tools/make_icon.py` to within WebP compression
  (mean difference 0.38/255 per channel).
- **It was not used.** Adopting it would have kept the amber identity that this rebrand replaces.
- **A new mark was drawn instead.** `assets/brand/plexdio.svg` follows the KruxSYS logo's grammar:
  square `--panel` tile, `--line-2` border, cyan corner brackets, light glyph with miter joins. It
  keeps the play triangle of the old icon, so it reads as a restyle of Plexdio rather than a new
  product.
- **The site needs updating.** Its Plexdio logo (`logo-128.*.webp`, used on the home page and the
  project page) should be replaced with a render of `assets/brand/plexdio.svg`.

The organisation logo `https://kruxsys.com/logo.png` (512 px, used in the site's schema.org data) is a
rounded, softened variant of the KruxSYS mark. The header SVG is sharp-cornered with miter joins, and
that is the canonical form followed here.
