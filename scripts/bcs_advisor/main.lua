-- main.lua — BCS Mix Advisor entry point
-- Add this script as a Reaper action via Actions > Load ReaScript.
-- Requires: js_ReaScriptAPI + ReaImGui (install via ReaPack).
--
-- Usage:
--   1. Select a track with Bus Channel Strip loaded.
--   2. Run the action. The floating panel opens.
--   3. Pick a studio profile (optional), type a brief, click Analyze.

-- ── Package path setup ───────────────────────────────────────────────────────
-- Resolve the directory this script lives in so require() works correctly.
local script_path = debug.getinfo(1, 'S').source:match('@?(.+[\\/])')
  or reaper.GetResourcePath() .. '/Scripts/bcs_advisor/'

-- Normalize to forward slashes for require
local require_base = script_path:gsub('\\', '/'):gsub('/$', '')

-- Add to package.path (Lua's module search path)
-- We support two layouts: inside Scripts/bcs_advisor/ or standalone
local function add_path(p)
  local entry = p .. '/?.lua'
  if not package.path:find(entry, 1, true) then
    package.path = entry .. ';' .. package.path
  end
end

-- The script directory itself (for bcs_advisor/param_map etc.)
local parent = require_base:match('(.+)/[^/]+$') or require_base
add_path(parent)
add_path(require_base)

-- ── Extension checks ─────────────────────────────────────────────────────────
if not reaper.ImGui_CreateContext then
  reaper.ShowMessageBox(
    'ReaImGui is not installed.\n\n' ..
    'Install it via ReaPack: Extensions > ReaPack > Browse Packages\n' ..
    'Search for "ReaImGui" and install.',
    'BCS Advisor — Missing Dependency', 0)
  return
end

-- ── Load modules ─────────────────────────────────────────────────────────────
local ok_params, params = pcall(require, 'bcs_advisor.param_map')
local ok_panel, panel   = pcall(require, 'bcs_advisor.panel')

if not ok_params or not ok_panel then
  reaper.ShowMessageBox(
    'Failed to load BCS Advisor modules.\n\n' ..
    'params: ' .. tostring(params) .. '\n' ..
    'panel: '  .. tostring(panel),
    'BCS Advisor — Load Error', 0)
  return
end

-- ── Find BCS on selected track ────────────────────────────────────────────────
local sel_track = reaper.GetSelectedTrack(0, 0)
if not sel_track then
  -- Fall back to master track
  sel_track = reaper.GetMasterTrack(0)
end

local fx_idx, fx_name = params.find_fx(sel_track)

if not fx_idx then
  -- Show panel anyway but with a warning — user can still run without it
  -- (useful for testing the Claude integration stand-alone)
  reaper.ShowConsoleMsg(
    '[BCS Advisor] Bus Channel Strip not found on selected/master track.\n' ..
    'Open the panel, select your bus track, and re-run, or continue for offline preview.\n')
end

local param_map = {}
if fx_idx then
  param_map = params.build_map(sel_track, fx_idx)
  reaper.ShowConsoleMsg(string.format(
    '[BCS Advisor] Found %s at FX[%d] — %d parameters mapped.\n',
    fx_name or 'BCS', fx_idx, (function()
      local n = 0
      for _ in pairs(param_map) do n = n + 1 end
      return n
    end)()))
end

-- ── Open panel ────────────────────────────────────────────────────────────────
panel.set_context(sel_track, fx_idx, param_map)
panel.open()
