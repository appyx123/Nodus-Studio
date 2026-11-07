local temp = 25.0
local hum = 50.0
local low_start = nil
local steps = {}
local step_idx = 1

function dht_step()
  if step_idx > #steps then
    nodus.set_pin_mode("DATA", "INPUT")
    nodus.set_visual_state("activity", 0.2)
    return
  end
  local s = steps[step_idx]
  step_idx = step_idx + 1
  nodus.set_pin_mode("DATA", "OUTPUT")
  nodus.set_pin("DATA", s.level)
  nodus.schedule_timer(s.dur_ns, "dht_step")
end

local function start_dht_response()
  steps = {}
  step_idx = 1
  nodus.set_visual_state("activity", 1.0)

  -- Response signal: 80us LOW, 80us HIGH
  table.insert(steps, { level = "LOW", dur_ns = 80000 })
  table.insert(steps, { level = "HIGH", dur_ns = 80000 })

  local h_raw = math.floor(hum * 10)
  if h_raw < 0 then h_raw = 0 end
  if h_raw > 1000 then h_raw = 1000 end

  local t_raw = 0
  if temp < 0 then
    t_raw = math.floor(math.abs(temp) * 10) + 32768
  else
    t_raw = math.floor(temp * 10)
  end

  local b1 = math.floor(h_raw / 256) % 256
  local b2 = h_raw % 256
  local b3 = math.floor(t_raw / 256) % 256
  local b4 = t_raw % 256
  local b5 = (b1 + b2 + b3 + b4) % 256

  local bytes = { b1, b2, b3, b4, b5 }
  for _, byte in ipairs(bytes) do
    for bit = 7, 0, -1 do
      local val = math.floor(byte / (2 ^ bit)) % 2
      -- Each bit begins with 50us LOW
      table.insert(steps, { level = "LOW", dur_ns = 50000 })
      -- Bit 0 is 28us HIGH, Bit 1 is 70us HIGH
      if val == 1 then
        table.insert(steps, { level = "HIGH", dur_ns = 70000 })
      else
        table.insert(steps, { level = "HIGH", dur_ns = 28000 })
      end
    end
  end

  -- Ending 50us LOW
  table.insert(steps, { level = "LOW", dur_ns = 50000 })

  -- Start step sequence after 30us release delay
  nodus.schedule_timer(30000, "dht_step")
end

function on_init()
  nodus.set_pin_mode("DATA", "INPUT")
  nodus.set_visual_state("activity", 0.2)
end

function on_control_change(control_id, value)
  if control_id == "temperature" then
    temp = value
  elseif control_id == "humidity" then
    hum = value
  end
end

function on_pin_edge(pin_id, state, timestamp_ns)
  if pin_id == "DATA" then
    if state == "LOW" then
      low_start = timestamp_ns
    elseif state == "HIGH" and low_start ~= nil then
      local dur = timestamp_ns - low_start
      low_start = nil
      -- MCU start signal is pulled low for >= 800us (typically 1ms - 20ms)
      if dur >= 800000 then
        start_dht_response()
      end
    end
  end
end
