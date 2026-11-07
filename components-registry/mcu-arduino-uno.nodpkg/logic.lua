local OFF = 0.15
local ON = 1.0

local function update()
  local high = nodus.read_pin("D13") == "HIGH"
  nodus.set_visual_state("led_builtin", high and ON or OFF)
end

function on_init()
  nodus.set_pin_mode("VCC", "OUTPUT")
  nodus.set_pin("VCC", "HIGH")
  nodus.set_pin_mode("GND", "OUTPUT")
  nodus.set_pin("GND", "LOW")
  update()
end

function on_pin_edge(pin_id, state, timestamp_ns)
  if pin_id == "D13" then
    update()
  end
end
