# AVR Emulation Backend Spike (Phase 3 Step 1)

## 1. Objective

Evaluate `simavr` Rust wrapper candidates (`simavr-sys`, `simavr-ffi`, `avr-simulator`) against Nodus Studio's Phase 3 requirements:
- `.elf` and Intel `.hex` firmware loading
- Cycle-accurate instruction stepping integrated with `SimTime` (`1 cycle @ 16 MHz = 125 half-ns`)
- ATmega328P GPIO (`DDRx`/`PORTx`/`PINx`), Timer0/Timer1/Timer2 & PWM, USART0 TX/RX, and reset
- License and redistribution compatibility with Nodus Studio (`MIT`)
- Native build reproducibility on Fedora Workstation 45 without root/system package pollution
- FFI memory safety and multi-instance isolation

## 2. Candidate Evaluation Summary

| Criterion | `simavr-sys 1.5.9` | `simavr-ffi 1.2.7` / `avr-simulator 0.6.4` | `avr-emulator 0.1.2` + `avr_uno.rs` (Selected) |
| --- | --- | --- | --- |
| License | `GPL-3.0` | Wrapper `MIT`, bundled `simavr` C core `GPL-3.0` (`vendor/simavr/COPYING`) | `BSD-3-Clause` (`avr-emulator`) + `MIT` (`avr_uno.rs`) |
| MIT Redistribution | Incompatible (copyleft static link) | Incompatible (`rustc-link-lib=static=simavr`) | Compatible |
| Fedora 45 Build | Fails (missing submodule, writes into Cargo source dir, `bindgen 0.32`) | Fails (`rustc-link-lib=static=elf` requires absent `libelf.a`, requires `libclang.so` and `avr-gcc`) | Passes (`no_std` pure Rust, zero native/C dependencies) |
| FFI / Crash Safety | Unsafe C FFI; `exit(1)` on bad ELF | Unsafe C FFI; global logger, signal handlers, `exit(1)` on bad ELF | 100% safe Rust; returns typed `Result` errors |
| Multi-Instance | Global state in C core | `logging::init()` global state; unsafe raw pointer borrows | Fully isolated per struct instance |

## 3. Detailed Findings on `simavr` Wrappers

1. **Redistribution & License Conflict**:
   - Nodus Studio (`LICENSE`) is distributed under the MIT License.
   - `simavr-ffi 1.2.7` vendors `simavr` under GNU GPL v3.0 (`vendor/simavr/COPYING`, 35,147 bytes) and statically links `libsimavr.a` into the final binary. Static linking of GPL-3.0 code imposes GPL-3.0 terms on binary distribution.
2. **Fedora 45 Build Blockers**:
   - `simavr-ffi-1.2.7/build.rs` executes `pkg_config::probe_library("libelf").unwrap()` and emits `cargo:rustc-link-lib=static=elf`. Fedora 45 (`elfutils-libelf-0.197-1.fc45`) ships shared `/usr/lib64/libelf.so.1` and does not ship a static `libelf.a` in `elfutils-libelf-devel`.
   - `simavr-ffi` runs `bindgen 0.69` at build time, requiring `libclang` (`clang-libs`/`clang-devel`), and invokes `make libsimavr` which requires `avr-gcc`.
3. **FFI Safety Hazard**:
   - `simavr`'s `elf_read_firmware` and `avr_init` invoke C `exit()`/`abort()` on malformed firmware or unknown MCU strings, which would terminate the entire Tauri desktop process instead of returning a recoverable diagnostic to the workbench.

## 4. Approved Architecture: `avr-emulator = "=0.1.2"` + `avr_uno.rs`

With user approval, Nodus Studio adopts `avr-emulator = "=0.1.2"` (BSD-3-Clause, zero transitive dependencies) as the AVR8 CPU instruction execution core (`Profile::avre_plus().with_program_counter_bits(16)` for ATmega328P) paired with a deterministic Rust peripheral layer in `src-tauri/src/mcu/avr_uno.rs`:

1. **Firmware Loader (`.hex` and `.elf`)**:
   - Native safe Rust parser supporting Intel HEX (`.hex` records `00` Data, `01` EOF, `02` Extended Segment, `04` Extended Linear) and 32-bit little-endian ELF (`EM_AVR = 0x53`, `PT_LOAD` flash & `.data` segments) bounded to ATmega328P 32 KiB flash (`0x8000` bytes).
2. **Cycle & Scheduler Integration**:
   - `Cpu::step` returns exact instruction cycle counts ($1\text{--}5$ cycles).
   - At $16\text{ MHz}$, $1\text{ cycle} = 125\text{ half-ns}$ (`SimTime::from_cycles_16mhz(cycles)`), preserving zero-drift synchronization with `Scheduler`.
3. **ATmega328P Peripherals Modeled in `avr_uno.rs`**:
   - **GPIO Ports B, C, D**: `PINB/DDRB/PORTB` (`0x03..0x05`), `PINC/DDRC/PORTC` (`0x06..0x08`), `PIND/DDRD/PORTD` (`0x09..0x0B`), including AVR writing-1-to-`PINx` toggle semantics and mapping to Arduino Uno pins `D0..D13` and `A0..A5`.
   - **Timer0 (8-bit)**: Normal, Phase-Correct PWM, CTC, and Fast PWM modes, prescalers ($1, 8, 64, 256, 1024$), `TCNT0`, `OCR0A`, `OCR0B`, `TIMSK0`/`TIFR0`, and `TIMER0_OVF` (vector 16) / `TIMER0_COMPA` (vector 14) / `TIMER0_COMPB` (vector 15) interrupts so Arduino `init()`, `millis()`, `micros()`, `delay()`, and `analogWrite()` on pins D5/D6 work out of the box.
   - **Timer1 (16-bit) & Timer2 (8-bit)**: PWM duty-cycle generation (`OCR1A`/`OCR1B` on D9/D10, `OCR2A`/`OCR2B` on D11/D3) and overflow/compare interrupts.
   - **USART0**: `UDR0` (`0xC6`), `UCSR0A/B/C` (`0xC0..0xC2`), `UBRR0L/H` (`0xC4..0xC5`), TX complete / UDRE flags (`UDRE0`, `TXC0`, `RXC0`) and RX buffer queue for bidirectional Serial Monitor communication.
4. **Documented Scope Boundaries**:
   - Analog comparator, EEPROM self-programming (`SPM`), watchdog reset, TWI/SPI hardware registers (scheduled for Phase 4 bus integration), and sleep current consumption are not claimed cycle-accurate in Phase 3.
