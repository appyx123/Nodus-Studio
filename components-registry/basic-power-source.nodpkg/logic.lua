function on_init()
  nodus.set_pin_mode("VCC", "OUTPUT")
  nodus.set_pin("VCC", "HIGH")
  nodus.set_pin_mode("GND", "OUTPUT")
  nodus.set_pin("GND", "LOW")
end
