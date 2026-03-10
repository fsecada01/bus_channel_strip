# Bus Channel Strip — ReaScript Mix Advisor Integration

## Concept

A Reaper-native mix advisor that:

1. Reads the current Bus Channel Strip parameter state on any bus track
2. Optionally captures a spectral snapshot of the bus signal
3. Sends context to Claude API with a user-defined creative brief
4. Receives parameter suggestions tied to studio profiles or free-form direction
5. Displays a ReaImGui floating panel with explanations + apply/compare controls
6. Supports A/B comparison: save current state, animate to suggestion, toggle back

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Reaper Session                           │
│                                                             │
│  ┌──────────────┐    TrackFX_Get/SetParam    ┌───────────┐ │
│  │ Bus Channel  │◄──────────────────────────►│ ReaScript │ │
│  │ Strip (VST3) │                            │ Advisor   │ │
│  └──────────────┘                            │           │ │
│                                              │  ReaImGui │ │
│  ┌──────────────┐    gmem shared memory      │  Panel    │ │
│  │ JSFX         │───────────────────────────►│           │ │
│  │ Spectral     │  (optional real-time       │           │ │
│  │ Analyzer     │   frequency data)          └─────┬─────┘ │
│  └──────────────┘                                  │       │
└───────────────────────────────────────────────────┼───────┘
                                                    │ HTTPS
                                                    ▼
                                           Claude API
                                        (claude-sonnet-4-6)
```

---

## Component 1: Parameter Snapshot

All Bus Channel Strip parameters are addressable via their NIH-plug stable IDs.
The ReaScript iterates through all plugin parameters using `TrackFX_GetParamIdent`
to build an ID→index map at session start, then reads/writes by index.

```lua
-- scripts/bcs_advisor/param_map.lua

local BCS = {}

-- Build a stable ID → parameter index map for Bus Channel Strip on a track/fx
function BCS.build_param_map(track, fx)
  local map = {}
  local n = reaper.TrackFX_GetNumParams(track, fx)
  for i = 0, n - 1 do
    local ok, ident = reaper.TrackFX_GetParamIdent(track, fx, i)
    if ok then
      map[ident] = i
    end
  end
  return map
end

-- Snapshot: returns table of { ident = value } for all params
function BCS.snapshot(track, fx, param_map)
  local snap = {}
  for ident, idx in pairs(param_map) do
    local val = reaper.TrackFX_GetParam(track, fx, idx)
    snap[ident] = val
  end
  return snap
end

-- Restore: applies a snapshot to the plugin
function BCS.restore(track, fx, param_map, snap)
  for ident, val in pairs(snap) do
    local idx = param_map[ident]
    if idx then
      reaper.TrackFX_SetParam(track, fx, idx, val)
    end
  end
end

return BCS
```

---

## Component 2: A/B Comparison System

```lua
-- scripts/bcs_advisor/ab_compare.lua

local AB = {}
local snapshot_a = nil
local snapshot_b = nil
local current_slot = "a"

function AB.save_a(track, fx, param_map)
  snapshot_a = BCS.snapshot(track, fx, param_map)
end

function AB.save_b(track, fx, param_map)
  snapshot_b = BCS.snapshot(track, fx, param_map)
end

-- Animate transition between states using defer() smoothstep interpolation
function AB.animate_to(track, fx, param_map, from_snap, to_snap, duration_s)
  local start_time = reaper.time_precise()

  local function step()
    local t = (reaper.time_precise() - start_time) / duration_s
    if t >= 1.0 then
      BCS.restore(track, fx, param_map, to_snap)
      return  -- done
    end

    -- Smoothstep easing
    local ease = t * t * (3 - 2 * t)

    for ident, to_val in pairs(to_snap) do
      local from_val = from_snap[ident] or to_val
      local idx = param_map[ident]
      if idx then
        reaper.TrackFX_SetParam(track, fx, idx,
          from_val + (to_val - from_val) * ease)
      end
    end

    reaper.defer(step)
  end

  reaper.defer(step)
end

-- Toggle between A and B
function AB.toggle(track, fx, param_map)
  if current_slot == "a" then
    local current = BCS.snapshot(track, fx, param_map)
    AB.animate_to(track, fx, param_map, current, snapshot_b, 0.8)
    current_slot = "b"
  else
    local current = BCS.snapshot(track, fx, param_map)
    AB.animate_to(track, fx, param_map, current, snapshot_a, 0.8)
    current_slot = "a"
  end
end

return AB
```

---

## Component 3: JSFX Spectral Analyzer (Optional)

When real-time frequency analysis is needed (e.g., detecting masking issues or
low-end buildup before sending to Claude), a companion JSFX effect reads the
bus signal and writes band energy to `gmem` shared memory.

```
// jsfx/bcs_spectrum_reader.jsfx
desc: BCS Spectrum Reader — writes band energy to gmem

slider1:gmem_offset=0<0,1024,1> GMem Offset

@init
fft_size = 1024;
buf = 0;
pos = 0;

@sample
buf[pos] = (spl0 + spl1) * 0.5;
pos = (pos + 1) % fft_size;

@block
// Run FFT and write 4-band RMS to gmem[offset..offset+3]
// Band 0: <500 Hz, Band 1: 500–2k, Band 2: 2k–6k, Band 3: 6k+
// (implementation uses Reaper's built-in fft() functions)
```

The ReaScript polls `gmem[offset..offset+3]` every 100ms and includes the band
energy in the Claude API request as additional context.

---

## Component 4: Claude API Integration

```lua
-- scripts/bcs_advisor/claude_client.lua

local json = require("json")  -- bundled with the script package

local CLAUDE = {}
CLAUDE.api_key = nil  -- loaded from %APPDATA%/reaper/bcs_advisor_key.txt

local function load_api_key()
  local path = reaper.GetResourcePath() .. "/bcs_advisor_key.txt"
  local f = io.open(path, "r")
  if f then
    CLAUDE.api_key = f:read("*l"):gsub("%s+", "")
    f:close()
    return true
  end
  return false
end

-- Build the system prompt with studio profile context
local function build_system_prompt(profile)
  local prompt = [[
You are a professional mix engineer and studio historian advising a producer
using the Bus Channel Strip VST3 plugin in Reaper. Your job is to suggest
parameter adjustments that reflect a specific studio or engineer's approach.

The plugin has six modules in signal order:
1. API5500 EQ (5-band semi-parametric)
2. ButterComp2 (VCA/Optical/FET/Tube compressor)
3. Pultec EQ (passive EQ with simultaneous boost+cut)
4. Dynamic EQ (4-band frequency-dependent compression)
5. Transformer (harmonic saturation, 4 vintage models)
6. Punch (clipper + transient shaper)

You must respond with a JSON object containing:
- "summary": 2-3 sentences describing your approach and the sonic character
- "warnings": array of strings (potential issues or conflicts to watch for)
- "parameters": object mapping NIH-plug stable parameter IDs to normalized
  values (0.0–1.0), only for parameters you recommend changing
- "rationale": object mapping each parameter ID to a brief explanation

Only suggest parameters where the change meaningfully reflects the requested
character. Do not suggest cosmetic or neutral changes.
]]

  if profile then
    prompt = prompt .. "\n\nStudio Profile Context:\n" .. json.encode(profile)
  end

  return prompt
end

-- Main request function
function CLAUDE.request(brief, current_params, profile, spectral_data, callback)
  if not CLAUDE.api_key then
    if not load_api_key() then
      callback(nil, "API key not found. Save your key to: " ..
        reaper.GetResourcePath() .. "/bcs_advisor_key.txt")
      return
    end
  end

  local user_message = "Creative brief: " .. brief .. "\n\n"
  user_message = user_message .. "Current parameter values (normalized 0–1):\n"
  user_message = user_message .. json.encode(current_params) .. "\n\n"

  if spectral_data then
    user_message = user_message .. "Real-time spectral analysis (band energy 0–1):\n"
    user_message = user_message .. string.format(
      "Sub/Low: %.2f | Low-Mid: %.2f | Hi-Mid: %.2f | High: %.2f\n",
      spectral_data[1], spectral_data[2], spectral_data[3], spectral_data[4])
  end

  local body = json.encode({
    model = "claude-sonnet-4-6",
    max_tokens = 1024,
    system = build_system_prompt(profile),
    messages = {{ role = "user", content = user_message }}
  })

  -- Use Reaper's async HTTP via curl (subprocess, non-blocking)
  local tmp_in  = os.tmpname() .. ".json"
  local tmp_out = os.tmpname() .. ".json"

  local f = io.open(tmp_in, "w")
  f:write(body)
  f:close()

  local cmd = string.format(
    'curl -s -X POST https://api.anthropic.com/v1/messages '..
    '-H "x-api-key: %s" '..
    '-H "anthropic-version: 2023-06-01" '..
    '-H "content-type: application/json" '..
    '--data @"%s" -o "%s"',
    CLAUDE.api_key, tmp_in, tmp_out)

  -- Run in background via defer polling
  local handle = io.popen(cmd .. " & echo PID=$!", "r")
  -- ... polling logic omitted for brevity

  callback({ summary = "...", parameters = {}, rationale = {} }, nil)
end

return CLAUDE
```

---

## Component 5: ReaImGui Panel

The floating panel provides:

- **Brief input**: free-text field ("Make this sound like Power Station 1982")
- **Profile picker**: dropdown of studio_profiles.json entries
- **Analyze button**: triggers spectral analysis + Claude API call
- **Result display**: summary text + per-parameter suggestions with rationale
- **Apply button**: animates current → suggested state (saves current as A)
- **A/B toggle**: animated comparison between original and suggestion
- **Save preset**: writes applied parameters as a Reaper FX preset

```lua
-- scripts/bcs_advisor/panel.lua
-- Requires js_ReaScriptAPI + ReaImGui extensions

local ctx = reaper.ImGui_CreateContext("BCS Mix Advisor")
local profiles = load_profiles()  -- from studio-profiles.json
local selected_profile = 0
local brief_text = ""
local result = nil
local is_loading = false

local function draw_panel()
  reaper.ImGui_SetNextWindowSize(ctx, 480, 600, reaper.ImGui_Cond_FirstUseEver())
  local visible, open = reaper.ImGui_Begin(ctx, "Bus Channel Strip Advisor", true)

  if visible then
    -- Profile selector
    reaper.ImGui_Text(ctx, "Studio / Engineer Profile:")
    if reaper.ImGui_BeginCombo(ctx, "##profile",
        profiles[selected_profile + 1].display_name) then
      for i, p in ipairs(profiles) do
        if reaper.ImGui_Selectable(ctx, p.display_name, i - 1 == selected_profile) then
          selected_profile = i - 1
        end
      end
      reaper.ImGui_EndCombo(ctx)
    end

    reaper.ImGui_Spacing(ctx)
    reaper.ImGui_Text(ctx, "Creative Brief:")
    _, brief_text = reaper.ImGui_InputTextMultiline(ctx, "##brief",
      brief_text, 400, 80)

    if not is_loading then
      if reaper.ImGui_Button(ctx, "Analyze & Suggest", 200, 0) then
        is_loading = true
        -- fire Claude API request
        CLAUDE.request(brief_text, current_params(),
          profiles[selected_profile + 1], spectral_data(),
          function(r, err)
            is_loading = false
            result = r
          end)
      end
    else
      reaper.ImGui_Text(ctx, "Analyzing...")
    end

    if result then
      reaper.ImGui_Separator(ctx)
      reaper.ImGui_TextWrapped(ctx, result.summary)
      reaper.ImGui_Spacing(ctx)

      for param_id, value in pairs(result.parameters) do
        local rationale = result.rationale[param_id] or ""
        reaper.ImGui_Text(ctx, string.format("%-30s → %.2f", param_id, value))
        if reaper.ImGui_IsItemHovered(ctx) then
          reaper.ImGui_SetTooltip(ctx, rationale)
        end
      end

      reaper.ImGui_Spacing(ctx)
      if reaper.ImGui_Button(ctx, "Apply (save A/B)", 140, 0) then
        AB.save_a(track, fx, param_map)
        -- convert normalized hints to actual values and apply
        apply_suggestions(result.parameters)
        AB.save_b(track, fx, param_map)
      end
      reaper.ImGui_SameLine(ctx)
      if reaper.ImGui_Button(ctx, "A/B Toggle", 100, 0) then
        AB.toggle(track, fx, param_map)
      end
    end
  end

  reaper.ImGui_End(ctx)
  if open then reaper.defer(draw_panel) end
end

reaper.defer(draw_panel)
```

---

## Normalization Map

NIH-plug reports parameter values in their display range (Hz, dB, ratios, etc.).
The Claude API suggestion uses normalized 0.0–1.0. Conversion:

```lua
local function normalized_to_param(track, fx, idx, normalized)
  local min, max = reaper.TrackFX_GetParamRange(track, fx, idx)
  return min + normalized * (max - min)
end

local function param_to_normalized(track, fx, idx, value)
  local min, max = reaper.TrackFX_GetParamRange(track, fx, idx)
  if max == min then return 0 end
  return (value - min) / (max - min)
end
```

---

## Installation

1. Copy `scripts/bcs_advisor/` to your Reaper scripts folder
2. Save your Anthropic API key to `%APPDATA%\REAPER\bcs_advisor_key.txt`
3. Install `js_ReaScriptAPI` and `ReaImGui` extensions via ReaPack
4. Add `bcs_advisor/main.lua` as a Reaper action
5. Optionally add `jsfx/bcs_spectrum_reader.jsfx` after Bus Channel Strip on your bus track

---

## Security Notes

- The API key is stored in a plain text file in the Reaper resource folder.
  This is acceptable for local use; do not commit the key file to version control.
- All API calls go directly to Anthropic's API — no intermediate server.
- Plugin parameter data sent to Claude contains no audio content, only numeric
  parameter values and optional band energy readings.
- The JSFX analyzer runs on the audio thread and uses gmem — no allocation,
  no locks, audio-thread safe.

---

## Creative Brief Examples

| Brief | Suggested Profile |
|-------|------------------|
| "80s NYC R&B hit" | `power-station-nyc` + `jimmy-jam-terry-lewis` blend |
| "West Coast classic rock warmth" | `sunset-sound` |
| "Controlled death metal brutality" | `erik-rutan-engineer` |
| "Unexpected combinations" | `rutan-producing-rb` or `cannibal-corpse-electric-lady` |
| "Nashville transparent mix" | `blackbird-studio` |
| "Deep Miami soul" | `criteria-recording` |
| "NYC analog warmth" | `electric-lady-studios` |
| Free-form (no profile) | Claude uses the brief alone |
