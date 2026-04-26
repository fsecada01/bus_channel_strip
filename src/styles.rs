// src/styles.rs
// CSS styling for reusable components

pub const COMPONENT_STYLES: &str = r#"

/* ── Base & root ───────────────────────────────────────────────────────────
   Depth-layered surface model:
     • Desktop (deepest): #070a0e              — sits behind chassis
     • Chassis outer:     #0d1014 → #141820   — frames the whole plugin
     • Strip/scroll rail: #10141a → #161a22   — inset mid-tone rail
     • Module card:       tinted per-theme     — raised above the rail
     • Control backing:   #0f1319 inset        — reads as recessed

   vizia CSS does NOT reliably support: box-shadow, margin-bottom,
   transform:translateY(). Apparent "lighting" therefore comes from
   layered gradients, border-color tints, and contrast between adjacent
   surfaces. Darker inner + lighter outer = looks recessed; lighter inner
   + darker surround = looks raised. */

:root {
    font-family: "Noto Sans";
    background-color: #070a0e;
    color: #ffffff;
}

/* Lunchbox chassis styling — outermost frame */
.lunchbox-chassis {
    background: linear-gradient(160deg, #0f131a, #181d27 60%, #101418);
    border: 2px solid #2a2f38;
    border-radius: 10px;
}

.chassis-header {
    background: linear-gradient(180deg, #2d323c 0%, #24282f 35%, #1b1f27 75%, #151922 100%);
    border-bottom: 2px solid #373c46;
    border-top: 1px solid #4a5162;
    padding: 10px 18px;
    border-radius: 8px 8px 0 0;
    gap: 14px;
    alignment: center;
}

.chassis-brand {
    font-size: 24px;
    font-weight: 700;
    color: #d4d8e0;
    letter-spacing: 2px;
}

.chassis-title {
    font-size: 18px;
    font-weight: 500;
    color: #ffffff;
    margin-left: 20px;
}

.master-controls {
    background: linear-gradient(180deg, rgba(20, 24, 32, 0.55), rgba(14, 17, 22, 0.6));
    padding: 6px 12px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.04);
    alignment: center;
}

.master-label {
    font-size: 12px;
    font-weight: 600;
    color: #d0d8e0;
    text-transform: uppercase;
    letter-spacing: 1px;
}

/* Strip scroll container — mid-tone rail between chassis and modules.
   Slight inset gradient + darker border reads as "recessed" beneath the
   raised modules above. */
.strip-scroll {
    background: linear-gradient(180deg, #0c0f14, #141821 60%, #191e27);
    border: 1px solid #252a32;
    border-radius: 8px;
}

/* Opaque fill is REQUIRED: ScrollView with transparent inner content causes
   Skia to composite translated text over stale pixels during horizontal
   scroll, producing a ghost/smear trail at the head and tail of the strip.
   Solid fill matches the .strip-scroll rail midpoint so the seam is invisible. */
.lunchbox-slots {
    padding: 16px;
    background-color: #141821;
    border-radius: 0;
}

.plugin-title {
    font-size: 24px;
    font-weight: 300;
    text-align: center;
    color: #ffffff;
    height: 40px;
    margin-bottom: 16px;
}

/* 500 Series Module Slots
   Each card sits above the strip rail. Per-theme .api5500-theme etc. rules
   override the background/border with tinted variants (see below). The base
   box-shadow-alike effect comes from:
     • thicker border (set inline in Rust) acts as an outer edge
     • inner gradient lighter-at-top for a subtle bevel
     • darker control-backing wells inside controls read as inset. */
.module-slot {
    border-radius: 8px;
    margin: 2px;
}

.module-header {
    text-align: center;
    padding-bottom: 4px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.12);
}

.module-name {
    font-size: 15px;
    font-weight: 700;
    color: #e0e0e0;
    letter-spacing: 1px;
    text-transform: uppercase;
}

.module-type {
    font-size: 13px;
    font-weight: 500;
    color: #b8b8b8;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    margin-top: 2px;
}

.section-label {
    font-size: 12px;
    font-weight: 600;
    color: #c8c8c8;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    text-align: center;
    margin-bottom: 4px;
}

/* Module section components - legacy support */
.module-section {
    background-color: #2a2a2a;
    border-radius: 8px;
    padding: 12px;
    margin: 4px;
    border: 1px solid #3a3a3a;
    transition: border-color 0.2s ease;
}

.module-section:hover {
    border-color: #4a4a4a;
}

.module-title {
    font-size: 16px;
    font-weight: 500;
    text-align: center;
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.section-title {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    margin-bottom: 6px;
}

/* ── Module themes ─────────────────────────────────────────────────────────
   Cards sit above the strip rail. Tinted-dark gradients preserve the module
   color coding spec (EQ cyan, Comp orange, Pultec gold, DynEQ green,
   Transformer charcoal, Punch red) while keeping good contrast with controls
   that use a darker backing. No box-shadow (vizia-unsupported). Rust inline
   .border_color() and .background_color() override CSS; these rules mostly
   serve as a fallback and for the .module-title label tint. */

.api5500-theme {
    border: 3px solid #40a0d0 !important;
    background: linear-gradient(165deg, #263945 0%, #1e2d38 45%, #182530) !important;
}
.api5500-theme .module-title {
    color: #7fc8e8;
}

.buttercomp2-theme {
    border: 3px solid #ff9640 !important;
    background: linear-gradient(165deg, #38281b 0%, #2b1f15 45%, #22170f) !important;
}
.buttercomp2-theme .module-title {
    color: #ffb070;
}

.pultec-theme {
    border: 3px solid #ffd700 !important;
    background: linear-gradient(165deg, #38311e 0%, #2b2617 45%, #221e12) !important;
}
.pultec-theme .module-title {
    color: #ffe055;
}

.dynamic-eq-theme {
    border: 3px solid #66cc66 !important;
    background: linear-gradient(165deg, #263825 0%, #1c2a1c 45%, #162216) !important;
}
.dynamic-eq-theme .module-title {
    color: #8fdf8f;
}

.transformer-theme {
    border: 3px solid #cc6633 !important;
    background: linear-gradient(165deg, #33211a 0%, #261810 45%, #1d120c) !important;
}
.transformer-theme .module-title {
    color: #e08858;
}

.punch-theme {
    border: 3px solid #ff3344 !important;
    background: linear-gradient(165deg, #381c1f 0%, #2a1618 45%, #20101a) !important;
}
.punch-theme .module-title {
    color: #ff6b78;
}

.haas-theme {
    border: 3px solid #8ca0d2 !important;
    background: linear-gradient(165deg, #1e2238 0%, #181b2d 45%, #121527) !important;
}
.haas-theme .module-title {
    color: #b4c0e4;
}

/* Signal flow indicator — subtle pill that blends into the chassis header
   gradient instead of reading as a separate framed block. */
.signal-flow-section {
    padding: 6px 14px;
    background: linear-gradient(180deg, rgba(14, 18, 23, 0.5), rgba(10, 13, 18, 0.55));
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.03);
}

.signal-flow-label {
    font-size: 11px;
    font-weight: 700;
    color: #8c98a8;
    text-transform: uppercase;
    letter-spacing: 1.2px;
}

.signal-flow-hint {
    font-size: 10px;
    font-weight: 400;
    color: #707886;
    font-style: italic;
}

.signal-flow-params {
    font-size: 9px;
    font-weight: 400;
    color: #5d6672;
    font-family: monospace;
}

/* ── Chain preset selector ─────────────────────────────────────────────────
   Compact horizontal button row inside the chassis header. Each button is
   a tag (3-char code) over a name. Visually borrows the zoom-btn aesthetic
   so the header reads as a unified control band, but with a slightly
   brighter accent on hover to invite experimentation. */

.chain-preset-btn {
    background: linear-gradient(180deg, #222730, #1b1f27);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 4px;
    cursor: pointer;
    alignment: center;
    padding: 2px 4px;
    gap: 1px;
}

.chain-preset-btn:hover {
    background: linear-gradient(180deg, #2c3340, #242a36);
    border-color: rgba(180, 200, 255, 0.3);
}

.chain-preset-tag {
    font-size: 11px;
    font-weight: 800;
    color: #c0c8d4;
    letter-spacing: 1px;
    text-align: center;
    width: 1s;
    height: 14px;
}

.chain-preset-name {
    font-size: 9px;
    font-weight: 600;
    color: #8a929e;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    text-align: center;
    width: 1s;
    height: 12px;
}

.chain-preset-btn:hover .chain-preset-tag {
    color: #ffffff;
}
.chain-preset-btn:hover .chain-preset-name {
    color: #c8d0dc;
}

/* ── Focus mode ────────────────────────────────────────────────────────────
   The clickable module-name target reads as a normal header by default but
   gains a subtle hover glow so users discover the focus affordance. The
   focused slot's name target gets a brighter top accent so the rack
   communicates "this one is the active focus". */

.module-name-target {
    padding: 2px 4px;
    border-radius: 3px;
    border: 1px solid transparent;
}

.module-name-target:hover {
    background: rgba(255, 255, 255, 0.04);
    border-color: rgba(255, 255, 255, 0.08);
}

.module-name-target-focused {
    background: rgba(255, 220, 100, 0.06);
    border-color: rgba(255, 220, 100, 0.25);
}

/* EXIT FOCUS pill in the chassis header. Sits beside the brand block while
   focus mode is active; matches the chain-preset button family but in an
   alert tone so it reads as "leave this mode". */
.exit-focus-btn {
    background: linear-gradient(180deg, #4a2018, #381610);
    border: 1px solid rgba(255, 140, 100, 0.4);
    border-radius: 4px;
    padding: 4px 10px;
    alignment: center;
}
.exit-focus-btn:hover {
    background: linear-gradient(180deg, #5c2820, #4a1d16);
    border-color: rgba(255, 180, 140, 0.6);
}
.exit-focus-label {
    font-size: 11px;
    font-weight: 700;
    color: #ffb098;
    letter-spacing: 0.8px;
    text-transform: uppercase;
    height: 14px;
    width: Auto;
}
.exit-focus-btn:hover .exit-focus-label {
    color: #ffe0d4;
}

/* ── Library sidebar ───────────────────────────────────────────────────────
   Narrow vertical strip at the left edge of the rack area. Compact rows
   show a status dot + 3-char tag for each module. In-rack rows use the
   module's accent color; available rows are muted. */

.library-sidebar {
    background: linear-gradient(180deg, rgba(20, 23, 30, 0.6), rgba(14, 16, 22, 0.7));
    border: 1px solid rgba(255, 255, 255, 0.04);
    border-radius: 6px;
    padding: 6px 4px;
}

.library-sidebar-header {
    font-size: 9px;
    font-weight: 800;
    color: #8c98a8;
    letter-spacing: 1.4px;
    text-transform: uppercase;
    text-align: center;
    height: 14px;
    width: 1s;
}

.library-row {
    padding: 2px 4px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
}

.library-row:hover {
    background: rgba(255, 255, 255, 0.05);
    border-color: rgba(255, 255, 255, 0.1);
}

.library-row-in-rack {
    background: rgba(255, 255, 255, 0.03);
}

.library-row-dot {
    font-size: 11px;
    height: 14px;
    width: 12px;
    text-align: center;
}

.library-row-tag {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.6px;
    height: 14px;
    width: 1s;
}

/* ── Chain mini-map ────────────────────────────────────────────────────────
   Slim band between the chassis header and the rack. One pill per slot,
   arrows between them. The focused pill gains a bright outline so users
   can see where they are even with the rack scrolled or in focus mode. */

.chain-minimap {
    background: linear-gradient(180deg, rgba(14, 16, 22, 0.6), rgba(10, 12, 18, 0.7));
    border: 1px solid rgba(255, 255, 255, 0.04);
    border-radius: 5px;
    padding: 4px 8px;
}

.minimap-arrow {
    font-size: 12px;
    color: #5d6672;
    width: 12px;
    height: 14px;
    text-align: center;
}

.minimap-pill {
    background: rgba(40, 44, 52, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 3px;
    alignment: center;
}

.minimap-pill:hover {
    background: rgba(60, 66, 78, 0.9);
    border-color: rgba(255, 255, 255, 0.2);
}

.minimap-pill-empty {
    background: transparent;
    border: 1px dashed rgba(110, 116, 128, 0.5);
}

.minimap-pill-focused {
    background: rgba(255, 220, 100, 0.12) !important;
    border-color: rgba(255, 220, 100, 0.7) !important;
}

.minimap-pill-tag {
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.8px;
    text-align: center;
    width: 1s;
    height: 14px;
}

/* ── Zoom controls ─────────────────────────────────────────────────────────
   Discrete zoom buttons in the chassis header. Active level has a tinted
   background + brighter label so the current scale is unambiguous. */

.zoom-controls {
    padding: 4px 8px;
    background: linear-gradient(180deg, rgba(14, 18, 23, 0.5), rgba(10, 13, 18, 0.55));
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.03);
}

.zoom-label {
    font-size: 10px;
    font-weight: 700;
    color: #8c98a8;
    text-transform: uppercase;
    letter-spacing: 1.2px;
    text-align: center;
}

.zoom-btn {
    background: linear-gradient(180deg, #222730, #1b1f27);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 4px;
    cursor: pointer;
    display: flex;
    alignment: center;
}

.zoom-btn:hover {
    background: linear-gradient(180deg, #2a303c, #232833);
    border-color: rgba(255, 255, 255, 0.12);
}

.zoom-btn-active {
    background: linear-gradient(180deg, #3a6888, #2a4e68) !important;
    border-color: rgba(138, 197, 232, 0.55) !important;
}

.zoom-btn-label {
    font-size: 11px;
    font-weight: 700;
    color: #a8b4c2;
    text-align: center;
    width: 1s;
}

.zoom-btn-active .zoom-btn-label {
    color: #ffffff;
}

.master-section {
    background: linear-gradient(145deg, #333333, #3a3a3a);
    border-radius: 8px;
    padding: 16px;
    margin: 8px;
    border: 2px solid #555555;
}

/* Parameter control components */
.param-group {
    margin: 4px 2px;
    padding: 8px;
    background-color: rgba(255, 255, 255, 0.02);
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    min-width: 120px;
    flex: 1;
}

.param-control {
    /* padding removed — morphorm counts padding in Auto height resolution,
       causing DynEQ band columns to overflow by ~64px (8 sliders × 8px).
       Instead, a subtle rgba tint separates each control from the tinted
       module card behind it, reading as a lightly recessed well. */
    background-color: rgba(0, 0, 0, 0.18);
    border-radius: 3px;
    transition: background-color 0.15s ease;
}

.param-control:hover {
    background-color: rgba(0, 0, 0, 0.28);
}

/* Parameter labels for 500 series modules */
.param-label {
    font-size: 12px;
    color: #e0e0e0;
    text-align: center;
    margin-bottom: 4px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

/* Dark param labels for dark chassis areas */
.param-label.dark {
    color: #cccccc;
}

/* Compact param label for DynEQ band columns — slightly smaller to recover
   vertical space. Used with dynamic (Stretch) spacing in dyneq_band_col!. */
.dyneq-param-label {
    font-size: 11px;
    color: #d0d0d0;
    text-align: center;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

/* Specialized parameter controls */
.frequency-control .param-label {
    color: #40a0d0;
}

.gain-control .param-label {
    color: #ff9640;
}

.ratio-control .param-label {
    color: #66cc66;
}

/* ── Bypass button ─────────────────────────────────────────────────────────
   Kept simple and clear: dark = on/normal, green = enabled, red = bypassed.
   No box-shadow or transform (vizia-unsupported); we rely on color + border. */
.bypass-button {
    background: linear-gradient(145deg, #2a3038, #1f242c);
    border: 1px solid #3a4050;
    border-radius: 4px;
    color: #e0e6ee;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    text-align: center;
    cursor: pointer;
    min-width: 60px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    transition: background-color 0.15s ease, border-color 0.15s ease;
}

.bypass-button:hover {
    background: linear-gradient(145deg, #343b45, #272d38);
    border-color: #4a5160;
}

.bypass-button.on {
    background: linear-gradient(145deg, #226b22, #1a5c1a);
    border-color: #3a8a3a;
    color: #ffffff;
}

.bypass-button.bypass {
    background: linear-gradient(145deg, #6b2222, #5c1a1a);
    border-color: #8a3a3a;
    color: #ffffff;
}

/* ── Active-LED button (module bypass) ─────────────────────────────────────
   Hardware power-LED convention: lit green when the module is processing,
   dark when bypassed. The BoolParam semantics are inverted (bypass=true means
   OFF), so the CSS applies the lit style to the UNCHECKED state and the dark
   style to :checked. Used for all six module bypass toggles; SOLO buttons
   keep the standard .bypass-button treatment. */
.active-led-button {
    background: linear-gradient(145deg, #2c8a2c, #1e7024);
    border: 1px solid #4dbd4d;
    border-radius: 4px;
    color: #f5fff5;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 700;
    text-align: center;
    cursor: pointer;
    min-width: 60px;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    transition: background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}

.active-led-button:hover {
    background: linear-gradient(145deg, #35a035, #24842a);
    border-color: #66d866;
}

.active-led-button:checked {
    background: linear-gradient(145deg, #1c1f24, #14171c);
    border-color: #2d3138;
    color: #6a7280;
}

.active-led-button:checked:hover {
    background: linear-gradient(145deg, #22262c, #191d23);
    border-color: #3a4050;
    color: #8a93a0;
}

/* Always-visible status LED: small glowing dot in the module header.
   Same color semantics as .active-led-button (green = active, dark = bypassed)
   but shaped as a round indicator and sized for header placement. Empty-label
   so the :checked state only reads as color change, not text. Still clickable
   (vizia CSS lacks pointer-events: none) — that's fine, double-toggle is
   harmless since it targets the same bypass param. */
.module-led-indicator {
    background: radial-gradient(circle at 35% 35%, #6ee46e, #2c8a2c 60%, #1e7024);
    border: 1px solid #4dbd4d;
    border-radius: 7px;
    width: 14px;
    height: 14px;
    min-width: 14px;
    min-height: 14px;
    padding: 0;
    color: transparent;
    font-size: 1px;
}
.module-led-indicator:hover {
    border-color: #7ce87c;
}
.module-led-indicator:checked {
    background: radial-gradient(circle at 35% 35%, #2a2e36, #16181d 60%, #0d0f13);
    border-color: #2d3138;
}
.module-led-indicator:checked:hover {
    border-color: #3a4050;
}

/* Hide button — small "×" glyph in the module header that collapses the slot
   to a narrow tab. Subtle by default, brightens on hover so it's discoverable
   without stealing attention from the main controls. */
.hide-btn {
    width: 18px;
    height: 18px;
    min-width: 18px;
    min-height: 18px;
    padding: 0;
    font-size: 14px;
    font-weight: 700;
    color: #6a7280;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
}
.hide-btn:hover {
    color: #e0a0a0;
    border-color: #5a3038;
    background: rgba(200, 60, 60, 0.08);
}

/* Eject button — header-mounted control that removes the slot's module and
   returns it to the picker state. Visually distinct from the hide button:
   uses the eject glyph (⏏) and a faint amber tint on hover so it reads as a
   destructive action without screaming for attention. */
.eject-btn {
    width: 18px;
    height: 18px;
    min-width: 18px;
    min-height: 18px;
    padding: 0;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 3px;
    alignment: center;
}
.eject-btn:hover {
    border-color: #6a4a18;
    background: rgba(220, 160, 50, 0.08);
}
.eject-btn-glyph {
    font-size: 13px;
    font-weight: 700;
    color: #7a8090;
    text-align: center;
    height: 14px;
    width: 1s;
}
.eject-btn:hover .eject-btn-glyph {
    color: #ffd070;
}

/* Empty slot theme — neutral steel border, dashed feel via a subtle muted
   gradient. Distinct from real-module themes so users can scan the rack and
   immediately see which slots are unoccupied. */
.empty-theme {
    border: 3px dashed #6e7480 !important;
    background: linear-gradient(165deg, #1e2128 0%, #181a20 45%, #14161b) !important;
}
.empty-theme .module-name {
    color: #9aa0ae;
    font-style: italic;
}
.empty-theme .module-type {
    color: #6e7480;
}

/* Library picker — the body of an empty slot. Vertical stack of one row per
   available module type. Each row reads as a clickable card with the
   module's accent color in the title; the subtitle stays muted. */
.library-picker {
    padding-top: 4px;
}
.picker-header {
    font-size: 11px;
    font-weight: 700;
    color: #c8ccd4;
    letter-spacing: 1.2px;
    text-transform: uppercase;
    text-align: center;
    margin-bottom: 4px;
    height: 16px;
}
.picker-row {
    padding: 6px 8px;
    background: rgba(40, 44, 52, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.04);
    border-radius: 4px;
}
.picker-row:hover {
    background: rgba(60, 66, 78, 0.85);
    border-color: rgba(255, 255, 255, 0.18);
}
.picker-row-name {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.5px;
    text-transform: uppercase;
    height: 14px;
}
.picker-row-subtitle {
    font-size: 10px;
    font-weight: 500;
    color: #8a909a;
    letter-spacing: 0.4px;
    text-transform: uppercase;
    height: 12px;
}

/* Collapsed tab layout — 56px wide strip containing the 3-char module tag
   and a full-height expand button. The tab keeps the theme border color so
   the chain remains visually readable even with several modules collapsed. */
.slot-collapsed {
    gap: 8px;
    padding: 8px 4px;
}
.collapsed-name {
    font-size: 13px;
    font-weight: 800;
    letter-spacing: 1.5px;
    color: inherit;
    text-align: center;
    width: 1s;
    height: 18px;
}
.expand-btn {
    width: 1s;
    height: 1s;
    min-height: 32px;
    padding: 0;
    font-size: 16px;
    font-weight: 700;
    color: #a0a8b8;
    background: rgba(30, 34, 42, 0.6);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 4px;
}
.expand-btn:hover {
    color: #ffffff;
    background: rgba(50, 58, 72, 0.8);
    border-color: rgba(255, 255, 255, 0.18);
}

/* Band ON button — inverted convention vs bypass buttons.
   :checked = param is true = band ENABLED = should look DARK (normal state).
   Unchecked = param is false = band DISABLED = should look LIT (alert state). */
.on-button {
    background: linear-gradient(145deg, #4a4a4a, #3a3a3a);
    border: 1px solid #666666;
    border-radius: 4px;
    color: #aaaaaa;
    padding: 6px 12px;
    font-size: 12px;
    font-weight: 600;
    text-align: center;
    cursor: pointer;
    min-width: 60px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.2);
}

.on-button:hover {
    background: linear-gradient(145deg, #555555, #444444);
}

/* Checked = enabled = DARK (normal processing state, like bypass=false) */
.on-button:checked {
    background: linear-gradient(145deg, #3a3a3a, #2a2a2a);
    border-color: #555555;
    color: #888888;
    box-shadow: inset 0 1px 3px rgba(0, 0, 0, 0.4);
}

/* Enhanced slider styling */
slider {
    height: 20px;
    background: linear-gradient(145deg, #404040, #353535);
    border-radius: 10px;
    margin: 2px 0px;
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.3);
    transition: all 0.15s ease;
}

slider:hover {
    background: linear-gradient(145deg, #454545, #3a3a3a);
}

slider .track {
    background: linear-gradient(145deg, #606060, #555555);
    border-radius: 10px;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.2);
}

slider .active {
    background: linear-gradient(145deg, #4080ff, #3070ef);
    border-radius: 10px;
    box-shadow: 0 0 8px rgba(64, 128, 255, 0.3);
}

slider .thumb {
    background: linear-gradient(145deg, #ffffff, #e0e0e0);
    border: 1px solid #cccccc;
    border-radius: 50%;
    width: 16px;
    height: 16px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
    transition: all 0.15s ease;
}

slider .thumb:hover {
    background: linear-gradient(145deg, #ffffff, #f0f0f0);
    box-shadow: 0 3px 6px rgba(0, 0, 0, 0.4);
    transform: scale(1.1);
}

/* Specialized slider themes */
.frequency-slider {
    border: 1px solid rgba(64, 160, 208, 0.3);
}

.frequency-slider .active {
    background: linear-gradient(145deg, #40a0d0, #3090c0);
}

.gain-slider {
    border: 1px solid rgba(255, 150, 64, 0.3);
}

.gain-slider .active {
    background: linear-gradient(145deg, #ff9640, #ef8630);
}

.ratio-slider {
    border: 1px solid rgba(102, 204, 102, 0.3);
}

.ratio-slider .active {
    background: linear-gradient(145deg, #66cc66, #56bc56);
}

/* Scrolling container */
.main-scroll-container {
    overflow-y: auto;
    overflow-x: hidden;
    height: 100%;
    width: 100%;
    padding-right: 8px;
}

/* Scrollbar styling */
scrollbar {
    width: 12px;
    background-color: rgba(255, 255, 255, 0.05);
    border-radius: 6px;
}

scrollbar .track {
    background-color: rgba(255, 255, 255, 0.1);
    border-radius: 6px;
}

scrollbar .thumb {
    background-color: rgba(255, 255, 255, 0.3);
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.1);
}

scrollbar .thumb:hover {
    background-color: rgba(255, 255, 255, 0.4);
}

/* Module section adjustments for flexible layout */
.module-section {
    background-color: #2a2a2a;
    border-radius: 6px;
    padding: 12px;
    margin: 6px;
    border: 1px solid #3a3a3a;
    transition: border-color 0.2s ease;
    min-height: 160px;
    height: auto;
}

/* Animation and interaction enhancements */
@keyframes glow-pulse {
    0%, 100% {
        box-shadow: 0 0 8px rgba(64, 160, 208, 0.2);
    }
    50% {
        box-shadow: 0 0 16px rgba(64, 160, 208, 0.4);
    }
}

.module-section.active {
    animation: glow-pulse 2s ease-in-out infinite;
}

/* ── Drag-to-reorder handle ──────────────────────────────────────────────── */

.drag-handle {
    background: rgba(255, 255, 255, 0.04);
    border-radius: 3px;
    padding: 2px 6px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    transition: background 0.12s ease, border-color 0.12s ease;
    display: flex;
    align-items: center;
    gap: 6px;
}

.drag-handle:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.2);
}

/* Applied when this slot is the selected source */
.drag-handle-active {
    background: rgba(64, 160, 255, 0.35) !important;
    border-color: rgba(64, 160, 255, 0.95) !important;
}

/* "● SELECTED" badge shown inside the drag handle when active */
.drag-selected-indicator {
    font-size: 9px;
    font-weight: 700;
    color: #ffdc32;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    margin-left: auto;
}

.drag-handle-icon {
    font-size: 15px;
    font-weight: 900;
    color: #cccccc;
    line-height: 1;
}

.drag-handle-label {
    font-size: 10px;
    font-weight: 700;
    color: #bbbbbb;
    text-transform: uppercase;
    letter-spacing: 1px;
}

/* Module slot highlighted as the selected reorder source.
   Note: box-shadow has limited support in vizia; use border-color +
   background instead. Border and name color are also set reactively
   in Rust (see create_dynamic_module_slot) for reliable rendering. */
.drag-source {
    background-color: rgba(64, 160, 255, 0.10) !important;
}

/* ── DynEQ flip-view styles ──────────────────────────────────────────────── */

/* Compact card shown inside the strip slot */
.dyneq-card-hint {
    font-size: 13px;
    font-weight: 600;
    color: #66cc66;
    text-transform: uppercase;
    letter-spacing: 0.8px;
}

.dyneq-card-desc {
    font-size: 11px;
    color: #999999;
    font-style: italic;
    line-height: 1.4;
}

.dyneq-open-btn {
    background: linear-gradient(145deg, #1e3d1e, #2a5c2a);
    border: 2px solid #66cc66;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
}

.dyneq-open-btn:hover {
    background: linear-gradient(145deg, #2a5c2a, #3a7a3a);
    border-color: #88ee88;
}

.dyneq-open-label {
    font-size: 13px;
    font-weight: 700;
    color: #66cc66;
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 1px;
}

/* Back view container — deeper green/slate base so the spectrum canvas
   and band columns pop against it. Darker gradient at edges for a vignette
   feel without box-shadow. */
.dyneq-back-view {
    background: linear-gradient(165deg, #1e2e1e 0%, #152015 45%, #0f180f);
    border: 2px solid #66cc66;
    border-radius: 8px;
}

/* Back button */
.dyneq-back-btn {
    background: linear-gradient(145deg, #2a2a2a, #333333);
    border: 1px solid #66cc66;
    border-radius: 5px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
}

.dyneq-back-btn:hover {
    background: linear-gradient(145deg, #333333, #444444);
    border-color: #88ee88;
}

.dyneq-back-btn-label {
    font-size: 12px;
    font-weight: 700;
    color: #66cc66;
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 0.8px;
}

.dyneq-back-title {
    font-size: 20px;
    font-weight: 700;
    color: #66cc66;
    text-transform: uppercase;
    letter-spacing: 2px;
    text-shadow: 0 0 10px rgba(102, 204, 102, 0.4);
}

/* Spectral analyzer placeholder */
.dyneq-spectrum {
    background: linear-gradient(145deg, #0d1a0d, #111f11);
    border: 1px solid rgba(102, 204, 102, 0.4);
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.dyneq-spectrum-title {
    font-size: 14px;
    font-weight: 700;
    color: #66cc66;
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 1.5px;
    opacity: 0.7;
}

.dyneq-spectrum-hint {
    font-size: 11px;
    color: #556655;
    text-align: center;
    font-style: italic;
}

/* Per-band column in the back view */
.dyneq-band-col {
    background: rgba(102, 204, 102, 0.04);
    border: 1px solid rgba(102, 204, 102, 0.15);
    border-radius: 6px;
    padding: 6px;
}

.dyneq-band-title {
    font-size: 11px;
    font-weight: 700;
    color: #66cc66;
    text-align: center;
    text-transform: uppercase;
    letter-spacing: 0.8px;
    margin-bottom: 4px;
}

/* DynEQ per-band expand/collapse chevron button */
.dyneq-chevron {
    background-color: transparent;
    border-width: 0px;
    color: #8899aa;
}

.dyneq-chevron:hover {
    color: #ffffff;
}

/* Responsive adjustments */
@media (max-width: 1200px) {
    .param-control {
        width: 80px;
    }

    slider {
        width: 70px;
    }
}

@media (max-width: 800px) {
    .module-section {
        padding: 8px;
        margin: 2px;
    }

    .param-control {
        width: 70px;
    }
}

/* Zoom: content-only scaling. vizia-plug does not support runtime host-window
   resize, so zoom buttons toggle a .zoom-N class on the chassis root and CSS
   scales fonts per level. Slot width and chassis padding are scaled from Rust
   via a reactive lens; widget heights are intentionally left constant so the
   strip row stays vertically stable under zoom. */
.zoom-75 .module-name        { font-size: 11px; }
.zoom-75 .module-type        { font-size: 10px; }
.zoom-75 .section-label      { font-size: 9px;  }
.zoom-75 .param-label        { font-size: 9px;  }
.zoom-75 .dyneq-param-label  { font-size: 9px;  }
.zoom-75 .dyneq-band-title   { font-size: 10px; }
.zoom-75 .chassis-brand      { font-size: 13px; }
.zoom-75 .chassis-title      { font-size: 10px; }
.zoom-75 .signal-flow-hint   { font-size: 9px;  }

.zoom-125 .module-name       { font-size: 18px; }
.zoom-125 .module-type       { font-size: 15px; }
.zoom-125 .section-label     { font-size: 14px; }
.zoom-125 .param-label       { font-size: 14px; }
.zoom-125 .dyneq-param-label { font-size: 14px; }
.zoom-125 .dyneq-band-title  { font-size: 15px; }
.zoom-125 .chassis-brand     { font-size: 22px; }
.zoom-125 .chassis-title     { font-size: 16px; }
.zoom-125 .signal-flow-hint  { font-size: 13px; }

.zoom-150 .module-name       { font-size: 22px; }
.zoom-150 .module-type       { font-size: 18px; }
.zoom-150 .section-label     { font-size: 16px; }
.zoom-150 .param-label       { font-size: 16px; }
.zoom-150 .dyneq-param-label { font-size: 16px; }
.zoom-150 .dyneq-band-title  { font-size: 18px; }
.zoom-150 .chassis-brand     { font-size: 26px; }
.zoom-150 .chassis-title     { font-size: 19px; }
.zoom-150 .signal-flow-hint  { font-size: 15px; }

.zoom-200 .module-name       { font-size: 28px; }
.zoom-200 .module-type       { font-size: 23px; }
.zoom-200 .section-label     { font-size: 20px; }
.zoom-200 .param-label       { font-size: 20px; }
.zoom-200 .dyneq-param-label { font-size: 20px; }
.zoom-200 .dyneq-band-title  { font-size: 23px; }
.zoom-200 .chassis-brand     { font-size: 34px; }
.zoom-200 .chassis-title     { font-size: 25px; }
.zoom-200 .signal-flow-hint  { font-size: 19px; }

"#;
