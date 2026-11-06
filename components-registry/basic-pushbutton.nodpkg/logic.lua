local closed = false

local function sync()
  if not closed then
    nodus.set_pin_mode("OUT", "INPUT")
    nodus.set_visual_state("cap", 0.35)
    return
  end
  nodus.set_visual_state("cap", 1.0)
  local level = nodus.read_pin("IN")
  if level == "HIGH" or level == "LOW" then
    nodus.set_pin_mode("OUT", "OUTPUT")
    nodus.set_pin("OUT", level)
  else
    nodus.set_pin_mode("OUT", "INPUT")
  end
end

function on_init()
  nodus.set_pin_mode("IN", "INPUT")
  sync()
end

function on_control_change(control_id, value)
  if control_id == "pressed" then
    closed = value >= 0.5
    sync()
  end
end

function on_pin_edge(pin_id, state, timestamp_ns)
  if pin_id == "IN" then
    sync()
  end
end
