-- advisor_client.lua
-- Async HTTP client that calls the bcs-advisor service at localhost:7373.
-- Uses curl as a subprocess + temp files + defer() polling so Reaper
-- doesn't block while waiting for the Claude API response.

local json = require('bcs_advisor.lib.json')

local M = {}

M.BASE_URL = 'http://localhost:7373'

-- Platform helper: returns a temp file path
local function tmpfile(ext)
  local base = reaper.GetResourcePath() .. '/bcs_advisor_tmp_'
  return base .. tostring(os.time()) .. '_' .. tostring(math.random(1e6)) .. (ext or '')
end

-- Build the curl command for a POST request
-- Writes body to a temp file, reads response from another temp file.
local function build_curl_cmd(url, body_file, out_file)
  -- Windows-safe quoting; curl is expected on PATH (ships with Windows 10+)
  return string.format(
    'curl -s -X POST "%s" -H "Content-Type: application/json" --data @"%s" -o "%s" --max-time 30',
    url, body_file, out_file)
end

local function build_get_cmd(url, out_file)
  return string.format(
    'curl -s -X GET "%s" -o "%s" --max-time 10',
    url, out_file)
end

-- Fire a command asynchronously (fire-and-forget launch, poll result file)
local function launch_async(cmd, out_file, callback)
  -- Write a done-sentinel alongside output so we can detect completion
  local sentinel = out_file .. '.done'

  -- On Windows: wrap in cmd /c so we can chain with echo to write sentinel
  local full_cmd
  if reaper.GetOS():find('Win') then
    full_cmd = string.format(
      'cmd /c %s & echo done > "%s"', cmd, sentinel)
  else
    full_cmd = string.format(
      '/bin/sh -c \'%s; touch "%s"\'', cmd, sentinel)
  end

  -- ExecProcess with timeout -1 = fire and forget on Windows
  -- On non-Win, same semantics via sh -c
  reaper.ExecProcess(full_cmd, -2)  -- -2 = async, discard output

  -- Poll every ~100ms via defer
  local function poll()
    local f = io.open(sentinel, 'r')
    if not f then
      reaper.defer(poll)
      return
    end
    f:close()
    os.remove(sentinel)

    local rf = io.open(out_file, 'r')
    local body = ''
    if rf then
      body = rf:read('*a')
      rf:close()
    end
    os.remove(out_file)

    callback(body)
  end

  reaper.defer(poll)
end

-- Fetch the profile list from the advisor service
-- callback(profiles_table, error_string)
function M.get_profiles(callback)
  local out = tmpfile('.json')
  local cmd = build_get_cmd(M.BASE_URL .. '/profiles', out)

  launch_async(cmd, out, function(body)
    if body == '' then
      callback(nil, 'No response from advisor service. Is `just advisor-dev` running?')
      return
    end
    local ok, result = pcall(json.decode, body)
    if not ok or not result then
      callback(nil, 'Failed to parse profiles: ' .. tostring(body):sub(1, 200))
      return
    end
    callback(result, nil)
  end)
end

-- POST /suggest
-- req: { brief, profile_id (optional), current_params (optional table), spectral (optional [4]) }
-- callback(response_table, error_string)
function M.suggest(req, callback)
  local body_file = tmpfile('_body.json')
  local out_file  = tmpfile('_resp.json')

  -- Write JSON body to temp file
  local body = json.encode(req)
  local f = io.open(body_file, 'w')
  if not f then
    callback(nil, 'Failed to write temp file: ' .. body_file)
    return
  end
  f:write(body)
  f:close()

  local cmd = build_curl_cmd(M.BASE_URL .. '/suggest', body_file, out_file)

  launch_async(cmd, out_file, function(resp_body)
    os.remove(body_file)

    if resp_body == '' then
      callback(nil, 'No response from advisor. Is `just advisor-dev` running on port 7373?')
      return
    end

    local ok, result = pcall(json.decode, resp_body)
    if not ok or not result then
      callback(nil, 'Failed to parse advisor response: ' .. tostring(resp_body):sub(1, 400))
      return
    end

    -- Advisor returns: { summary, parameters, rationale, warnings, profile_used }
    callback(result, nil)
  end)
end

-- Health check — callback(true/false)
function M.health_check(callback)
  local out = tmpfile('_health.txt')
  local cmd = build_get_cmd(M.BASE_URL .. '/health', out)
  launch_async(cmd, out, function(body)
    callback(body:match('ok') ~= nil)
  end)
end

return M
