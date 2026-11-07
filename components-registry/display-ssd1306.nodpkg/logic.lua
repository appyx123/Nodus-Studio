function on_init()
  nodus.set_pin_mode("SCL", "INPUT")
  nodus.set_pin_mode("SDA", "INPUT")
  nodus.set_visual_state("screen", 0.05)
end

function on_control_change(control_id, value)
end

function on_pin_edge(pin_id, state, timestamp_ns)
  if pin_id == "SCL" or pin_id == "SDA" then
    nodus.set_visual_state("screen", 0.85)
  end
end
