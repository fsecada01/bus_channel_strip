-- ab_compare.lua
-- A/B snapshot system with smoothstep-animated transitions.
-- A = original state, B = suggested state.

local M = {}

local snap_a    = nil
local snap_b    = nil
local current   = 'a'   -- which slot is currently active

-- Save current live state as slot A
function M.save_a(track, fx, param_map, snapshot_fn)
  snap_a   = snapshot_fn(track, fx, param_map)
  current  = 'a'
end

-- Save a pre-built snapshot as slot B (e.g. from advisor suggestions)
function M.save_b_from(snap)
  snap_b = snap
end

function M.has_a() return snap_a ~= nil end
function M.has_b() return snap_b ~= nil end
function M.current_slot() return current end

-- Smoothstep easing: maps t∈[0,1] → smooth 0→1
local function smoothstep(t)
  t = math.max(0, math.min(1, t))
  return t * t * (3 - 2 * t)
end

-- Animate from from_snap → to_snap over duration_s seconds using defer()
local function animate_to(track, fx, param_map, from_snap, to_snap,
                           duration_s, on_done)
  local start = reaper.time_precise()

  local function step()
    local elapsed = reaper.time_precise() - start
    local t = elapsed / duration_s

    if t >= 1.0 then
      -- Final frame: exact target values
      for ident, to_val in pairs(to_snap) do
        local idx = param_map[ident]
        if idx then
          reaper.TrackFX_SetParamNormalized(track, fx, idx, to_val)
        end
      end
      if on_done then on_done() end
      return
    end

    local ease = smoothstep(t)
    for ident, to_val in pairs(to_snap) do
      local from_val = from_snap[ident] or to_val
      local idx = param_map[ident]
      if idx then
        reaper.TrackFX_SetParamNormalized(track, fx, idx,
          from_val + (to_val - from_val) * ease)
      end
    end

    reaper.defer(step)
  end

  reaper.defer(step)
end

-- Toggle between A and B with animation
function M.toggle(track, fx, param_map, on_done)
  if not snap_a or not snap_b then return end

  if current == 'a' then
    -- Live state is A — animate to B
    local live = {}
    for ident, idx in pairs(param_map) do
      live[ident] = reaper.TrackFX_GetParamNormalized(track, fx, idx)
    end
    animate_to(track, fx, param_map, live, snap_b, 0.5, function()
      current = 'b'
      if on_done then on_done() end
    end)
  else
    -- Live state is B — animate back to A
    local live = {}
    for ident, idx in pairs(param_map) do
      live[ident] = reaper.TrackFX_GetParamNormalized(track, fx, idx)
    end
    animate_to(track, fx, param_map, live, snap_a, 0.5, function()
      current = 'a'
      if on_done then on_done() end
    end)
  end
end

-- Jump immediately to A (no animation)
function M.recall_a(track, fx, param_map)
  if not snap_a then return end
  for ident, val in pairs(snap_a) do
    local idx = param_map[ident]
    if idx then
      reaper.TrackFX_SetParamNormalized(track, fx, idx, val)
    end
  end
  current = 'a'
end

-- Jump immediately to B (no animation)
function M.recall_b(track, fx, param_map)
  if not snap_b then return end
  for ident, val in pairs(snap_b) do
    local idx = param_map[ident]
    if idx then
      reaper.TrackFX_SetParamNormalized(track, fx, idx, val)
    end
  end
  current = 'b'
end

return M
