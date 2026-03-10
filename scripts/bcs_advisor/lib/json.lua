-- dkjson — minimal JSON encode/decode for Reaper Lua
-- Adapted from dkjson by David Heiko Kolf (MIT license)
-- https://dkolf.de/src/dkjson-lua.fsl/

local json = {}

-- ── Encode ────────────────────────────────────────────────────────────────────

local escape_map = {
  ['"']  = '\\"', ['\\'] = '\\\\', ['\b'] = '\\b',
  ['\f'] = '\\f', ['\n'] = '\\n',  ['\r'] = '\\r', ['\t'] = '\\t',
}

local function escape_string(s)
  return s:gsub('[%c"\\]', function(c)
    return escape_map[c] or string.format('\\u%04x', c:byte())
  end)
end

local function encode_value(val, indent, level)
  local t = type(val)
  if t == 'nil'     then return 'null'
  elseif t == 'boolean' then return tostring(val)
  elseif t == 'number' then
    if val ~= val then return 'null' end  -- NaN
    if val == math.huge or val == -math.huge then return 'null' end
    if math.floor(val) == val and math.abs(val) < 1e15 then
      return string.format('%d', val)
    end
    return string.format('%.6g', val)
  elseif t == 'string' then
    return '"' .. escape_string(val) .. '"'
  elseif t == 'table' then
    -- Detect array vs object
    local is_array = true
    local max_n = 0
    for k, _ in pairs(val) do
      if type(k) ~= 'number' or k < 1 or math.floor(k) ~= k then
        is_array = false; break
      end
      if k > max_n then max_n = k end
    end
    if is_array and max_n ~= #val then is_array = false end

    local parts = {}
    if is_array then
      for i = 1, #val do
        parts[#parts+1] = encode_value(val[i], indent, level+1)
      end
      return '[' .. table.concat(parts, ',') .. ']'
    else
      for k, v in pairs(val) do
        if type(k) == 'string' then
          parts[#parts+1] = '"' .. escape_string(k) .. '":' ..
            encode_value(v, indent, level+1)
        end
      end
      return '{' .. table.concat(parts, ',') .. '}'
    end
  end
  return 'null'
end

function json.encode(val)
  return encode_value(val, nil, 0)
end

-- ── Decode ────────────────────────────────────────────────────────────────────

local function skip_ws(s, i)
  return s:match('^%s*()', i)
end

local decode_value  -- forward declaration

local function decode_string(s, i)
  local res = {}
  i = i + 1  -- skip opening "
  while i <= #s do
    local c = s:sub(i,i)
    if c == '"' then return table.concat(res), i+1 end
    if c == '\\' then
      local e = s:sub(i+1,i+1)
      local simple = {['"']='"',['\\']='\\',['/']=
        '/b',b='\b',f='\f',n='\n',r='\r',t='\t'}
      -- simpler escape handling
      local esc = ({['"']='"',['\\']='\\',['/']=
        '/',b='\b',f='\f',n='\n',r='\r',t='\t'})[e]
      if esc then
        res[#res+1] = esc; i = i + 2
      elseif e == 'u' then
        local hex = s:sub(i+2, i+5)
        local cp = tonumber(hex, 16) or 0
        if cp < 0x80 then
          res[#res+1] = string.char(cp)
        elseif cp < 0x800 then
          res[#res+1] = string.char(0xC0 + math.floor(cp/64),
            0x80 + cp%64)
        else
          res[#res+1] = string.char(0xE0 + math.floor(cp/4096),
            0x80 + math.floor(cp/64)%64, 0x80 + cp%64)
        end
        i = i + 6
      else
        res[#res+1] = e; i = i + 2
      end
    else
      res[#res+1] = c; i = i + 1
    end
  end
  return nil, 'unterminated string'
end

local function decode_array(s, i)
  local arr = {}
  i = skip_ws(s, i+1)
  if s:sub(i,i) == ']' then return arr, i+1 end
  while true do
    local val, ni = decode_value(s, i)
    arr[#arr+1] = val
    i = skip_ws(s, ni)
    local c = s:sub(i,i)
    if c == ']' then return arr, i+1 end
    if c ~= ',' then return nil, 'expected , or ]' end
    i = skip_ws(s, i+1)
  end
end

local function decode_object(s, i)
  local obj = {}
  i = skip_ws(s, i+1)
  if s:sub(i,i) == '}' then return obj, i+1 end
  while true do
    if s:sub(i,i) ~= '"' then return nil, 'expected string key' end
    local key, ni = decode_string(s, i)
    i = skip_ws(s, ni)
    if s:sub(i,i) ~= ':' then return nil, 'expected :' end
    i = skip_ws(s, i+1)
    local val, vi = decode_value(s, i)
    obj[key] = val
    i = skip_ws(s, vi)
    local c = s:sub(i,i)
    if c == '}' then return obj, i+1 end
    if c ~= ',' then return nil, 'expected , or }' end
    i = skip_ws(s, i+1)
  end
end

decode_value = function(s, i)
  i = skip_ws(s, i)
  local c = s:sub(i,i)
  if c == '"' then return decode_string(s, i)
  elseif c == '[' then return decode_array(s, i)
  elseif c == '{' then return decode_object(s, i)
  elseif c == 't' then return true,  i+4
  elseif c == 'f' then return false, i+5
  elseif c == 'n' then return nil,   i+4
  else
    local num = s:match('^-?%d+%.?%d*[eE]?[+-]?%d*', i)
    if num then return tonumber(num), i + #num end
    return nil, 'unexpected character: ' .. c
  end
end

function json.decode(s)
  local val, pos = decode_value(s, 1)
  return val
end

return json
