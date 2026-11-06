pub mod avr_uno;
pub mod traits;

pub use avr_uno::AvrUno;
pub use traits::{parse_avr_elf, parse_intel_hex, McuPinState, Microcontroller};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::{CompileRequest, CompilerController};
    use crate::engine::LogicLevel;

    fn d13_level(uno: &AvrUno) -> LogicLevel {
        uno.pin_drives()
            .into_iter()
            .find(|p| p.pin_id == "D13")
            .map(|p| p.level)
            .unwrap_or(LogicLevel::HighZ)
    }

    #[test]
    fn avr_uno_executes_blink_pwm_input_and_serial_from_compiled_sketch() {
        let compiler = CompilerController::default();
        let sketch = r#"
void setup() {
  Serial.begin(9600);
  pinMode(13, OUTPUT);
  pinMode(2, INPUT_PULLUP);
  analogWrite(9, 64);
  Serial.print("BOOT:");
}
void loop() {
  if (Serial.available() > 0) {
    char c = Serial.read();
    Serial.write(c + 1);
  }
  digitalWrite(13, HIGH);
  delay(2);
  digitalWrite(13, LOW);
  delay(2);
}
"#;
        let res = compiler
            .compile(CompileRequest {
                sketch_source: sketch.into(),
                fqbn: Some("arduino:avr:uno".into()),
            })
            .unwrap();
        assert!(res.success, "compile stderr: {}", res.stderr);
        let artifact = compiler.latest_artifact().unwrap();

        let mut uno = AvrUno::new();
        uno.load_elf(&artifact.elf).unwrap();
        uno.load_hex(&artifact.hex).unwrap();

        // Advance ~1 ms (16,000 cycles) for setup() + first HIGH in loop()
        uno.step_cycles(16_000).unwrap();
        let boot_out = String::from_utf8_lossy(&uno.drain_serial_tx()).into_owned();
        assert_eq!(boot_out, "BOOT:");
        assert_eq!(d13_level(&uno), LogicLevel::High);

        // Advance ~2.2 ms (35,200 cycles) into the LOW half of loop()
        uno.step_cycles(35_200).unwrap();
        assert_eq!(d13_level(&uno), LogicLevel::Low);

        // Advance ~2 ms (32,000 cycles) back into HIGH and verify RX -> TX echo (+1)
        uno.push_serial_rx(b"A");
        uno.step_cycles(32_000).unwrap();
        assert_eq!(d13_level(&uno), LogicLevel::High);
        assert_eq!(uno.drain_serial_tx(), b"B");

        // Sample PWM duty cycle on D9 (analogWrite(9, 64) => 64/256 = 25% HIGH)
        let mut high_samples = 0usize;
        for _ in 0..256 {
            uno.step_cycles(64).unwrap();
            let d9 = uno
                .pin_drives()
                .into_iter()
                .find(|p| p.pin_id == "D9")
                .unwrap();
            if d9.level == LogicLevel::High {
                high_samples += 1;
            }
        }
        assert_eq!(high_samples, 64);

        // Verify external input pin D2 affects PINx register via digitalRead(2)
        let input_sketch = r#"
void setup() {
  pinMode(2, INPUT_PULLUP);
  pinMode(13, OUTPUT);
}
void loop() {
  digitalWrite(13, digitalRead(2));
}
"#;
        let res2 = compiler
            .compile(CompileRequest {
                sketch_source: input_sketch.into(),
                fqbn: Some("arduino:avr:uno".into()),
            })
            .unwrap();
        assert!(res2.success);
        let art2 = compiler.latest_artifact().unwrap();
        uno.load_hex(&art2.hex).unwrap();
        uno.set_input_pin("D2", LogicLevel::High);
        uno.step_cycles(8_000).unwrap();
        assert_eq!(d13_level(&uno), LogicLevel::High);
        uno.set_input_pin("D2", LogicLevel::Low);
        uno.step_cycles(8_000).unwrap();
        assert_eq!(d13_level(&uno), LogicLevel::Low);
    }
}
