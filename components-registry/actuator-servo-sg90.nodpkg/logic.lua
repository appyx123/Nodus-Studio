local pwm_start = nil
local current_angle = 90.0

function on_init()
  nodus.set_pin_mode("PWM", "INPUT")
  nodus.set_visual_state("angle", current_angle)
end

function on_control_change(control_id, value)
end

function on_pin_edge(pin_id, state, timestamp_ns)
  if pin_id == "PWM" then
    if state == "HIGH" then
      pwm_start = timestamp_ns
    elseif state == "LOW" and pwm_start ~= nil then
      local pulse_ns = timestamp_ns - pwm_start
      pwm_start = nil
      -- Standard 50 Hz RC servo pulse: 1.0 ms (0 deg) to 2.0 ms (180 deg)
      -- Allow extended range 500 us to 2500 us
      if pulse_ns >= 500000 and pulse_ns <= 2500000 then
        local angle = ((pulse_ns - 1000000) / 1000000.0) * 180.0
        if angle < 0.0 then angle = 0.0 end
        if angle > 180.0 then angle = 180.0 end
        current_angle = angle
        nodus.set_visual_state("angle", current_angle)
      end
    end
  end
end
