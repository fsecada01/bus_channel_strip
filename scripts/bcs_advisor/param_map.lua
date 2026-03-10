-- param_map.lua
-- Discovers Bus Channel Strip parameters on a track/fx slot,
-- and provides snapshot / restore helpers.

local M = {}

local BCS_PLUGIN_NAMES = {
  "Bus Channel Strip",
  "bus_channel_strip",
  "Bus-Channel-Strip",
}

-- Find the first BCS fx index on a track, or nil
function M.find_fx(track)
  local n = reaper.TrackFX_GetCount(track)
  for i = 0, n - 1 do
    local ok, name = reaper.TrackFX_GetFXName(track, i, '')
    if ok then
      for _, pat in ipairs(BCS_PLUGIN_NAMES) do
        if name:find(pat, 1, true) then
          return i, name
        end
      end
    end
  end
  return nil
end

-- Build a stable ident → index map for all BCS parameters
function M.build_map(track, fx)
  local map = {}
  local n = reaper.TrackFX_GetNumParams(track, fx)
  for i = 0, n - 1 do
    local ok, ident = reaper.TrackFX_GetParamIdent(track, fx, i)
    if ok and ident ~= '' then
      map[ident] = i
    end
  end
  return map
end

-- Snapshot: returns { [ident] = normalized_value } for all params
-- Values are stored as normalized (0..1) via GetParamNormalized
function M.snapshot(track, fx, param_map)
  local snap = {}
  for ident, idx in pairs(param_map) do
    snap[ident] = reaper.TrackFX_GetParamNormalized(track, fx, idx)
  end
  return snap
end

-- Restore: applies a normalized snapshot to the plugin
function M.restore(track, fx, param_map, snap)
  reaper.Undo_BeginBlock()
  for ident, norm_val in pairs(snap) do
    local idx = param_map[ident]
    if idx then
      reaper.TrackFX_SetParamNormalized(track, fx, idx, norm_val)
    end
  end
  reaper.Undo_EndBlock('BCS Advisor: restore snapshot', -1)
end

-- Apply a partial set of normalized suggestions (from Claude)
-- Only touches the keys present in suggestions
function M.apply_suggestions(track, fx, param_map, suggestions)
  reaper.Undo_BeginBlock()
  for ident, norm_val in pairs(suggestions) do
    local idx = param_map[ident]
    if idx then
      reaper.TrackFX_SetParamNormalized(track, fx, idx,
        math.max(0.0, math.min(1.0, norm_val)))
    end
  end
  reaper.Undo_EndBlock('BCS Advisor: apply suggestions', -1)
end

-- Build a compact params table for sending to the advisor API
-- Returns { [ident] = normalized_value } for all params
function M.current_params_for_api(track, fx, param_map)
  return M.snapshot(track, fx, param_map)
end

return M
