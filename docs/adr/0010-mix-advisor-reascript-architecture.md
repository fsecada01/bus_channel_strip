# ADR-0010: Mix Advisor — Rust HTTP Broker + ReaScript/ReaImGui Client, Studio-Profile-v1 Schema

**Status**: Implemented

**Deciders**: Project (Claude Code orchestration session)

---

## Context

A companion feature was designed to let a Reaper user request a mix character in natural language ("make this sound like Power Station in 1982") and get back concrete Bus Channel Strip parameter suggestions grounded in real studio/engineer research, with an in-DAW A/B comparison workflow. This required deciding: what runs where (Reaper-native script vs. external app), how studio/engineer character gets encoded as structured data the LLM and the script can both use, how the Claude API key gets held without landing in a script that's easy to leak, and how parameter suggestions get applied safely without corrupting DAW automation/session state.

The original design considered here called the Claude API directly from ReaScript with a locally stored plaintext API key file. **That was not what got built.** The implemented architecture inserts a small local Rust HTTP service between the ReaScript client and the Claude API specifically to keep the API key off disk in plaintext and out of Lua's reach.

## Decision

**Architecture: a local Rust/Axum HTTP broker service (`advisor/`, a Cargo workspace member — `bcs-advisor`) holds the Claude API key server-side and exposes `/profiles`, `/profiles/:id`, `/suggest`, and `/health` over `localhost:7373`; ReaScript (Lua, `scripts/bcs_advisor/`) talks only to that local service, never to `api.anthropic.com` directly.** The plugin instance itself remains untouched — the ReaScript drives it purely through Reaper's existing `TrackFX_GetParam`/`TrackFX_SetParam`/`TrackFX_GetParamIdent` host API. This keeps the plugin's own audio-thread code completely uninvolved in the advisor feature — no new parameters, no new IPC channel, no risk to the audio-thread safety rules.

Component decisions:
- **API key isolation**: `ANTHROPIC_API_KEY` is read once from the advisor process's environment (`advisor/.env`, gitignored, loaded via `set dotenv-load := true` in the justfile) and held in `AppState` (`advisor/src/main.rs`) for the life of the process. It never appears in a file the ReaScript reads, is never sent to the Reaper process, and is never logged. `advisor_client.lua` (`scripts/bcs_advisor/advisor_client.lua`) only ever sees `http://localhost:7373`.
- **ReaScript ↔ broker transport**: `curl` subprocess (`ExecProcess`, async/fire-and-forget) writing request/response bodies to temp files under the Reaper resource path, polled via `reaper.defer()` — chosen because ReaScript's built-in HTTP facilities are limited and this avoids blocking the UI thread while waiting on a network round-trip that includes an LLM call.
- **Broker ↔ Claude transport**: the Rust service calls `https://api.anthropic.com/v1/messages` directly with `reqwest`, model `claude-sonnet-4-6`, a fixed system prompt describing the six-module signal chain and a strict JSON-only output contract (`advisor/src/claude.rs`), and parses/validates the structured response before returning it to the ReaScript client — so a malformed or off-contract LLM response fails at the broker, not deep inside Lua string parsing.
- **Stable parameter addressing** via `TrackFX_GetParamIdent` to build an ID→index map at session start (NIH-plug's stable string IDs, not positional indices, since indices aren't guaranteed stable across plugin versions) — `scripts/bcs_advisor/param_map.lua`.
- **A/B comparison via animated interpolation**, not instant snapshot/restore: `defer()`-driven smoothstep easing between saved parameter snapshots, so toggling A/B doesn't produce audible zipper artifacts (`scripts/bcs_advisor/ab_compare.lua`).
- **Optional JSFX spectral analyzer** (`jsfx/bcs_spectrum_reader.jsfx`) as a separate, audio-thread-safe companion effect inserted after Bus Channel Strip on the bus track — 1024-point FFT, 4-band RMS energy (Sub/Low, Low-Mid, Hi-Mid, High) written to `gmem` shared memory on a `defer()`-friendly hop schedule, one-pole smoothed, no allocation, no locks, no I/O. Genuinely optional: `/suggest` accepts `spectral` as an optional field, and the advisor works without it.
- **Studio-profile-v1 JSON schema**, stored as **`docs/adr/resources/studio-profiles.json`** (loaded by the broker at startup — see "Data file location" below) — models three entity types (`studio`, `engineer`, `combination`), each carrying room/console/tape/outboard character plus a `module_parameter_hints` block of normalized (0.0–1.0) suggestions per Bus Channel Strip module, presented to the user as *targets with rationale* via the `rationale` map in `/suggest`'s response, never hard-applied silently.
- **Normalized hints, raw-value snapshots**: suggestions travel as 0.0–1.0 normalized values (mapped to actual ranges via `TrackFX_GetParamRange` at apply time in the ReaScript), but the A/B snapshot system stores raw `TrackFX_GetParam` values — so DAW automation and session recall continue to work correctly regardless of what the advisor suggested.

### Data file location

`studio-profiles.json` is live application data loaded by the running `bcs-advisor` service at startup (`advisor/src/main.rs::load_profiles`) — not documentation. It lives at `docs/adr/resources/studio-profiles.json` because that's where it landed during the 2026-09-05 `docs/` cleanup (the prior location, `docs/specs/`, was removed as part of that cleanup); the broker's default path resolution and `advisor/.env.example`'s example both point here. `BCS_PROFILES_PATH` can override this for local development.

## Consequences

**Easier:**
- The API key never has to be trusted to a script interpreter with no compiler, no sandbox, and no dependency isolation — the highest-risk credential-handling code lives in a typed, testable Rust binary instead.
- Zero coupling between this feature and the plugin's own DSP codebase or audio-thread rules — the broker, the ReaScript, and the JSFX analyzer can all be built, changed, or abandoned without touching `src/`.
- The studio-profile schema is reusable outside the advisor script (e.g., a future in-plugin preset browser) since it's plain versioned JSON, not tied to ReaScript internals.
- Response validation (the JSON-contract parsing in `advisor/src/claude.rs`) happens once, server-side, instead of being duplicated in Lua.

**Harder:**
- There is now a process the user must have running (`just advisor-dev` / `just advisor`) before the ReaScript panel is useful; both `advisor_client.lua` and `panel.lua` surface "is the advisor running?" errors when it isn't, but this is an extra moving part compared to a self-contained script.
- ReaScript/Lua still has no compiler or strong tooling — correctness for the client half of the integration (`scripts/bcs_advisor/*.lua`) relies more on manual testing in Reaper than on the audio-thread module's usual test discipline.
- `docs/adr/resources/` is an ADR-adjacent path being used to store a real runtime dependency of a live service; this is a known awkward fit accepted for now (see "Data file location" above) rather than introducing a dedicated `advisor/data/` directory for one file.

**Unchanged:**
- The plugin's own parameter automation, smoothing, and NIH-plug integration are untouched by this feature — the advisor only ever calls the same host API (`TrackFX_*`) any external automation would use.
- This ADR does not commit to a ship date or further feature scope for the advisor beyond what's implemented — future changes to it (new profile fields, additional broker routes) should get their own ADR if they involve a real architectural choice.
