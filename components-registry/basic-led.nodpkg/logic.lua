local OFF = 0.15
local ON = 1.0
local WINDOW_NS = 5000000
local last_ns = 0
local high_ns = 0
local window_ns = 0
local active = false
local settle_handle = nil

local function update(now_ns)
  local dt = now_ns - last_ns
  if dt > 0 then
    if dt >= WINDOW_NS then
      window_ns = dt
      high_ns = active and dt or 0
    else
      window_ns = window_ns + dt
      if active then
        high_ns = high_ns + dt
      end
      if window_ns > WINDOW_NS then
        local scale = WINDOW_NS / window_ns
        high_ns = high_ns * scale
        window_ns = WINDOW_NS
      end
    end
    last_ns = now_ns
  end
  active = nodus.read_pin("A") == "HIGH" and nodus.read_pin("K") == "LOW"
  local duty = active and 1.0 or 0.0
  if window_ns > 0 and high_ns > 0 and high_ns < window_ns then
    duty = high_ns / window_ns
    if settle_handle then
      nodus.cancel_timer(settle_handle)
    end
    settle_handle = nodus.schedule_timer(WINDOW_NS, "on_settle")
  end
  nodus.set_visual_state("brightness", OFF + (ON - OFF) * duty)
end

function on_settle(timestamp_ns)
  settle_handle = nil
  update(timestamp_ns)
end

function on_init()
  nodus.set_pin_mode("A", "INPUT")
  nodus.set_pin_mode("K", "INPUT")
  update(0)
end

function on_pin_edge(pin_id, state, timestamp_ns)
  update(timestamp_ns)
end
