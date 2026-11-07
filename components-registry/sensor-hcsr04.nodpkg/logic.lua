local distance_cm = 20.0
local trig_start = nil

function echo_high()
  nodus.set_pin("ECHO", "HIGH")
  nodus.set_visual_state("pulse", 1.0)
  -- 58 microseconds per centimeter round-trip speed of sound
  local pulse_ns = math.floor(distance_cm * 58000)
  if pulse_ns < 116000 then pulse_ns = 116000 end
  nodus.schedule_timer(pulse_ns, "echo_low")
end

function echo_low()
  nodus.set_pin("ECHO", "LOW")
  nodus.set_visual_state("pulse", 0.2)
end

function on_init()
  nodus.set_pin_mode("TRIG", "INPUT")
  nodus.set_pin_mode("ECHO", "OUTPUT")
  nodus.set_pin("ECHO", "LOW")
  nodus.set_visual_state("pulse", 0.2)
end

function on_control_change(control_id, value)
  if control_id == "distance" then
    distance_cm = value
  end
end

function on_pin_edge(pin_id, state, timestamp_ns)
  if pin_id == "TRIG" then
    if state == "HIGH" then
      trig_start = timestamp_ns
    elseif state == "LOW" and trig_start ~= nil then
      local dur = timestamp_ns - trig_start
      trig_start = nil
      -- Standard HC-SR04 trigger pulse is >= 10us (allow >= 8us)
      if dur >= 8000 then
        -- 200us internal acoustic burst propagation before echo asserts HIGH
        nodus.schedule_timer(200000, "echo_high")
      end
    end
  end
end
