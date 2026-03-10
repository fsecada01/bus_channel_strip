-- panel.lua
-- ReaImGui floating panel for the BCS Mix Advisor.
-- Requires: js_ReaScriptAPI + ReaImGui extensions (install via ReaPack).

local json    = require('bcs_advisor.lib.json')
local params  = require('bcs_advisor.param_map')
local ab      = require('bcs_advisor.ab_compare')
local client  = require('bcs_advisor.advisor_client')

local M = {}

-- ── State ──────────────────────────────────────────────────────────────────────

local ctx             = nil
local font_large      = nil

-- Plugin context (set by main.lua)
local track           = nil
local fx_idx          = nil
local param_map       = {}

-- Profiles
local profiles        = {}         -- list of { id, display_name, ... }
local profile_names   = {}         -- display strings for combo
local selected_idx    = 0          -- 0-based into profiles[]

-- UI state
local brief_buf       = ''
local is_loading      = false
local load_start      = 0
local status_msg      = ''
local status_is_error = false

-- Result
local result          = nil        -- last SuggestResponse from advisor
local ab_animating    = false

-- Advisor online flag
local advisor_online  = false

-- ── Helpers ───────────────────────────────────────────────────────────────────

local function set_status(msg, is_err)
  status_msg      = msg or ''
  status_is_error = is_err or false
end

local function refresh_profiles()
  client.get_profiles(function(data, err)
    if err then
      set_status('Profiles: ' .. err, true)
      advisor_online = false
      return
    end
    advisor_online = true
    profiles = data or {}
    profile_names = {}
    for _, p in ipairs(profiles) do
      profile_names[#profile_names+1] = p.display_name
        .. (p.entity_type == 'combination' and '  ✦' or '')
    end
    if #profiles > 0 and selected_idx >= #profiles then
      selected_idx = 0
    end
    set_status('Advisor online — ' .. #profiles .. ' profiles loaded')
  end)
end

local function do_suggest()
  if not track or not fx_idx then
    set_status('No Bus Channel Strip found on selected track.', true)
    return
  end
  if brief_buf:gsub('%s+', '') == '' and selected_idx == 0 then
    set_status('Enter a brief or select a studio profile.', true)
    return
  end

  is_loading  = true
  load_start  = reaper.time_precise()
  result      = nil
  set_status('Sending to Claude…')

  local req = { brief = brief_buf }

  -- Attach selected profile id (profiles list is 1-based, selected_idx is 0-based)
  if selected_idx > 0 and profiles[selected_idx] then
    req.profile_id = profiles[selected_idx].id
  end

  -- Attach current params
  if next(param_map) then
    req.current_params = params.current_params_for_api(track, fx_idx, param_map)
  end

  -- Attach spectral data from gmem if JSFX reader is present
  local gmem_base = 0   -- must match slider1 in bcs_spectrum_reader.jsfx
  local b0 = reaper.gmem_read(gmem_base)
  if b0 and b0 ~= 0 then
    req.spectral = {
      reaper.gmem_read(gmem_base),
      reaper.gmem_read(gmem_base + 1),
      reaper.gmem_read(gmem_base + 2),
      reaper.gmem_read(gmem_base + 3),
    }
  end

  client.suggest(req, function(resp, err)
    is_loading = false
    if err then
      set_status('Error: ' .. err, true)
      return
    end
    result = resp
    set_status('Done — ' .. (resp.profile_used and ('Profile: ' .. resp.profile_used) or 'free-form'))
  end)
end

local function do_apply()
  if not result or not result.parameters then return end
  if not track or not fx_idx then return end

  -- Save current live state as A
  ab.save_a(track, fx_idx, param_map, params.snapshot)

  -- Apply suggestions
  params.apply_suggestions(track, fx_idx, param_map, result.parameters)

  -- Build normalized B snapshot from applied state
  local snap_b = params.snapshot(track, fx_idx, param_map)
  ab.save_b_from(snap_b)

  set_status('Applied ✓  —  A/B toggle ready')
end

local function spinner_char()
  local chars = { '⠋','⠙','⠸','⠴','⠦','⠇' }
  local i = math.floor((reaper.time_precise() - load_start) * 8) % #chars + 1
  return chars[i]
end

-- ── Draw ──────────────────────────────────────────────────────────────────────

local function draw()
  reaper.ImGui_SetNextWindowSize(ctx, 500, 640,
    reaper.ImGui_Cond_FirstUseEver())

  local visible, open = reaper.ImGui_Begin(ctx, 'BCS Mix Advisor', true)

  if visible then
    -- ── Header ────────────────────────────────────────────────────────────────
    local r, g, b = 0.4, 0.75, 1.0
    reaper.ImGui_TextColored(ctx, reaper.ImGui_ColorConvertDouble4ToU32(r,g,b,1),
      'Bus Channel Strip Advisor')
    reaper.ImGui_SameLine(ctx)

    -- Online indicator
    if advisor_online then
      reaper.ImGui_TextColored(ctx,
        reaper.ImGui_ColorConvertDouble4ToU32(0.2,0.9,0.3,1), '● online')
    else
      reaper.ImGui_TextColored(ctx,
        reaper.ImGui_ColorConvertDouble4ToU32(0.9,0.3,0.2,1), '● offline')
    end

    -- Track / FX info
    if track then
      local ok, tname = reaper.GetTrackName(track, '')
      local fxname = ''
      if fx_idx then
        reaper.TrackFX_GetFXName(track, fx_idx, '')
        ok, fxname = reaper.TrackFX_GetFXName(track, fx_idx, '')
      end
      reaper.ImGui_TextDisabled(ctx,
        string.format('Track: %s  |  FX[%d]: %s  |  %d params',
          tname or '?', fx_idx or -1, fxname or '?', #param_map))
    else
      reaper.ImGui_TextColored(ctx,
        reaper.ImGui_ColorConvertDouble4ToU32(1,0.6,0.2,1),
        'No BCS found on selected track.')
    end

    reaper.ImGui_Separator(ctx)

    -- ── Profile picker ────────────────────────────────────────────────────────
    reaper.ImGui_Text(ctx, 'Studio / Engineer Profile:')
    reaper.ImGui_SetNextItemWidth(ctx, -1)

    local preview = selected_idx == 0 and '— None (free-form) —'
      or (profile_names[selected_idx] or '?')

    if reaper.ImGui_BeginCombo(ctx, '##profile', preview) then
      if reaper.ImGui_Selectable(ctx, '— None (free-form) —', selected_idx == 0) then
        selected_idx = 0
      end
      for i, name in ipairs(profile_names) do
        local is_sel = (i == selected_idx)
        if reaper.ImGui_Selectable(ctx, name, is_sel) then
          selected_idx = i
        end
        -- Show description as tooltip
        if reaper.ImGui_IsItemHovered(ctx) and profiles[i] then
          reaper.ImGui_SetTooltip(ctx, profiles[i].description or '')
        end
      end
      reaper.ImGui_EndCombo(ctx)
    end

    reaper.ImGui_Spacing(ctx)

    -- ── Brief input ───────────────────────────────────────────────────────────
    reaper.ImGui_Text(ctx, 'Creative Brief:')
    reaper.ImGui_SetNextItemWidth(ctx, -1)
    local _, new_brief = reaper.ImGui_InputTextMultiline(ctx, '##brief',
      brief_buf, 460, 80)
    brief_buf = new_brief or brief_buf

    reaper.ImGui_TextDisabled(ctx,
      'e.g. "Heavy 80s NYC R&B, analog warmth, punchy low-mids"')

    reaper.ImGui_Spacing(ctx)

    -- ── Action buttons ────────────────────────────────────────────────────────
    if is_loading then
      reaper.ImGui_TextColored(ctx,
        reaper.ImGui_ColorConvertDouble4ToU32(0.8,0.8,0.2,1),
        spinner_char() .. '  Analyzing…')
    else
      if reaper.ImGui_Button(ctx, 'Analyze & Suggest', 200, 0) then
        do_suggest()
      end
      reaper.ImGui_SameLine(ctx)
      if reaper.ImGui_Button(ctx, 'Refresh Profiles', 140, 0) then
        refresh_profiles()
      end
    end

    -- ── Status bar ────────────────────────────────────────────────────────────
    if status_msg ~= '' then
      reaper.ImGui_Spacing(ctx)
      if status_is_error then
        reaper.ImGui_TextColored(ctx,
          reaper.ImGui_ColorConvertDouble4ToU32(1,0.4,0.4,1), status_msg)
      else
        reaper.ImGui_TextDisabled(ctx, status_msg)
      end
    end

    -- ── Result ────────────────────────────────────────────────────────────────
    if result then
      reaper.ImGui_Separator(ctx)

      -- Summary
      if result.summary then
        reaper.ImGui_TextWrapped(ctx, result.summary)
        reaper.ImGui_Spacing(ctx)
      end

      -- Warnings
      if result.warnings and #result.warnings > 0 then
        reaper.ImGui_TextColored(ctx,
          reaper.ImGui_ColorConvertDouble4ToU32(1,0.75,0.2,1), 'Warnings:')
        for _, w in ipairs(result.warnings) do
          reaper.ImGui_BulletText(ctx, w)
        end
        reaper.ImGui_Spacing(ctx)
      end

      -- Parameter suggestions table
      if result.parameters and next(result.parameters) then
        reaper.ImGui_Text(ctx, 'Suggested parameters:')

        if reaper.ImGui_BeginTable(ctx, '##params', 2,
            reaper.ImGui_TableFlags_RowBg() |
            reaper.ImGui_TableFlags_BordersOuter()) then

          reaper.ImGui_TableSetupColumn(ctx, 'Parameter', 0, 280)
          reaper.ImGui_TableSetupColumn(ctx, 'Value', 0, 60)
          reaper.ImGui_TableHeadersRow(ctx)

          for ident, norm_val in pairs(result.parameters) do
            reaper.ImGui_TableNextRow(ctx)
            reaper.ImGui_TableSetColumnIndex(ctx, 0)
            reaper.ImGui_Text(ctx, ident)
            -- Tooltip = rationale
            if reaper.ImGui_IsItemHovered(ctx) and result.rationale then
              local note = result.rationale[ident]
              if note then reaper.ImGui_SetTooltip(ctx, note) end
            end
            reaper.ImGui_TableSetColumnIndex(ctx, 1)
            reaper.ImGui_Text(ctx, string.format('%.2f', norm_val))
          end

          reaper.ImGui_EndTable(ctx)
        end

        reaper.ImGui_Spacing(ctx)

        -- Apply / A-B buttons
        if reaper.ImGui_Button(ctx, 'Apply  (save A→B)', 150, 0) then
          do_apply()
        end

        reaper.ImGui_SameLine(ctx)

        if ab.has_a() and ab.has_b() then
          local slot_label = ab.current_slot() == 'a' and 'A→B' or 'B→A'
          if not ab_animating then
            if reaper.ImGui_Button(ctx, 'A/B: ' .. slot_label, 100, 0) then
              ab_animating = true
              ab.toggle(track, fx_idx, param_map, function()
                ab_animating = false
              end)
            end
          else
            reaper.ImGui_TextDisabled(ctx, 'animating…')
          end
          reaper.ImGui_SameLine(ctx)
          if reaper.ImGui_Button(ctx, 'Recall A', 80, 0) then
            ab.recall_a(track, fx_idx, param_map)
          end
        end
      end
    end
  end

  reaper.ImGui_End(ctx)
  return open
end

-- ── Public API ────────────────────────────────────────────────────────────────

function M.set_context(t, fx, pm)
  track     = t
  fx_idx    = fx
  param_map = pm or {}
end

function M.open()
  if ctx then return end  -- already open
  ctx = reaper.ImGui_CreateContext('BCS Mix Advisor')

  refresh_profiles()

  local function loop()
    local open = draw()
    if open then
      reaper.defer(loop)
    else
      reaper.ImGui_DestroyContext(ctx)
      ctx = nil
    end
  end

  reaper.defer(loop)
end

function M.is_open()
  return ctx ~= nil
end

return M
