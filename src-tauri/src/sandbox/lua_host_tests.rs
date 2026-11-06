use super::*;
use crate::engine::{CircuitEdit, CircuitState, EventPayload, LogicLevel, Scheduler};
use crate::sandbox::manifest::parse_manifest;
use crate::sandbox::pkg_loader::{ComponentPackage, RegistryCatalog};

fn test_manifest() -> Manifest {
    parse_manifest(
        r#"
schema_version = 1
[compatibility]
engine_api = 1
[component]
id = "test-part"
name = "Test Part"
category = "Basic"
author = "Nodus"
version = "1.0.0"
description = "Lua host fixture"
[electrical]
operating_voltage_min = 0.0
operating_voltage_max = 5.0
[[pins]]
id = "A"
name = "Pin A"
type = "bidirectional"
svg_anchor = "pin-a"
[[pins]]
id = "z"
name = "Pin Z"
type = "output"
svg_anchor = "pin-z"
[[pins]]
id = "NC"
name = "Not connected"
type = "nc"
svg_anchor = "pin-nc"
[[controls]]
id = "level"
label = "Level"
type = "slider"
min = 0.0
max = 10.0
step = 1.0
default = 5.0
[[visuals]]
id = "brightness"
type = "number"
target = "dyn-led"
property = "opacity"
min = 0.0
max = 1.0
default = 0.15
"#,
    )
    .unwrap()
}

fn setup_circuit(manifest: &Manifest) -> (CircuitState, Scheduler, String) {
    let mut circuit = CircuitState::default();
    circuit.register_catalog(&RegistryCatalog {
        packages: vec![ComponentPackage {
            manifest: manifest.clone(),
            svg: "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"/>".into(),
            lua_source: "-- fixture".into(),
        }],
        diagnostics: vec![],
    });
    let id = "node-00000000-0000-4000-8000-000000000000".to_string();
    circuit
        .edit(CircuitEdit::AddInstance {
            instance_id: id.clone(),
            component_id: manifest.component.id.clone(),
            component_version: manifest.component.version.clone(),
        })
        .unwrap();
    let mut scheduler = Scheduler::default();
    scheduler.start();
    (circuit, scheduler, id)
}

#[test]
fn lua_api_drives_pins_reads_nets_schedules_timers_and_updates_visuals() {
    let manifest = test_manifest();
    let (mut circuit, mut scheduler, id) = setup_circuit(&manifest);
    let script = r#"
        local timer_handle = nil

        function on_init()
            nodus.set_pin_mode("z", "OUTPUT")
            nodus.set_pin("z", "HIGH")
            nodus.set_pin_mode("A", "INPUT_PULLDOWN")
        end

        function on_control_change(cid, val)
            if cid == "level" and val >= 8.0 then
                timer_handle = nodus.schedule_timer(250, "on_timeout")
            end
        end

        function on_pin_edge(pin_id, state, ts_ns)
            if pin_id == "A" and state == "HIGH" then
                nodus.set_visual_state("brightness", 0.9)
            elseif pin_id == "A" and state == "UNKNOWN" then
                nodus.set_visual_state("brightness", 0.5)
            end
        end

        function on_timeout(ts_ns)
            nodus.set_pin("z", "LOW")
            nodus.set_visual_state("brightness", 1.0)
        end
    "#;

    let mut host = LuaSandboxInstance::new(id.clone(), &manifest, script).unwrap();
    host.call_init(&mut circuit, &mut scheduler).unwrap();
    assert_eq!(
        circuit
            .read_pin(&Endpoint {
                instance_id: id.clone(),
                pin_id: "z".into()
            })
            .unwrap(),
        LogicLevel::High
    );
    assert_eq!(
        circuit
            .read_pin(&Endpoint {
                instance_id: id.clone(),
                pin_id: "A".into()
            })
            .unwrap(),
        LogicLevel::Low
    );

    host.call_pin_edge(
        "A",
        LogicLevel::High,
        SimTime::from_ns(100).unwrap(),
        &mut circuit,
        &mut scheduler,
    )
    .unwrap();
    assert_eq!(host.visual_values()["brightness"], 0.9);

    host.call_control_change("level", 9.0, &mut circuit, &mut scheduler)
        .unwrap();
    assert_eq!(scheduler.pending_count(), 1);

    let event = scheduler
        .pop_next_until(SimTime::from_ns(500).unwrap())
        .unwrap()
        .unwrap();
    if let EventPayload::LuaTimer { callback, .. } = event.payload {
        host.call_timer(
            event.handle,
            &callback,
            event.timestamp,
            &mut circuit,
            &mut scheduler,
        )
        .unwrap();
    }
    assert_eq!(
        circuit
            .read_pin(&Endpoint {
                instance_id: id,
                pin_id: "z".into()
            })
            .unwrap(),
        LogicLevel::Low
    );
    assert_eq!(host.visual_values()["brightness"], 1.0);
}

#[test]
fn forbidden_globals_and_non_deterministic_rng_are_removed() {
    let manifest = test_manifest();
    let (mut circuit, mut scheduler, id) = setup_circuit(&manifest);
    let script = r#"
        function on_init()
            assert(io == nil, "io must be nil")
            assert(os == nil, "os must be nil")
            assert(package == nil, "package must be nil")
            assert(debug == nil, "debug must be nil")
            assert(require == nil, "require must be nil")
            assert(dofile == nil, "dofile must be nil")
            assert(load == nil, "load must be nil")
            assert(loadfile == nil, "loadfile must be nil")
            assert(collectgarbage == nil, "collectgarbage must be nil")
            assert(math.random == nil, "math.random must be nil")
            assert(math.randomseed == nil, "math.randomseed must be nil")
        end
    "#;
    let mut host = LuaSandboxInstance::new(id, &manifest, script).unwrap();
    host.call_init(&mut circuit, &mut scheduler).unwrap();
}

#[test]
fn infinite_loop_memory_exhaustion_and_timer_flood_disable_instance_and_release_pins() {
    let manifest = test_manifest();

    // 1. Infinite loop trips instruction budget, releases pin to HIGH_Z, and cancels timers.
    let (mut circuit, mut scheduler, id) = setup_circuit(&manifest);
    let loop_script = r#"
        function on_tick() end
        function on_init()
            nodus.set_pin_mode("z", "OUTPUT")
            nodus.set_pin("z", "HIGH")
            nodus.schedule_timer(100, "on_tick")
        end
        function on_pin_edge(pin_id, state, ts_ns)
            while true do end
        end
    "#;
    let mut loop_host = LuaSandboxInstance::new(id.clone(), &manifest, loop_script).unwrap();
    loop_host.call_init(&mut circuit, &mut scheduler).unwrap();
    assert_eq!(scheduler.pending_count(), 1);
    let diag = loop_host
        .call_pin_edge(
            "A",
            LogicLevel::High,
            SimTime::ZERO,
            &mut circuit,
            &mut scheduler,
        )
        .unwrap_err();
    assert!(diag.message.contains("Instruction budget exceeded"));
    assert!(loop_host.fault().is_some());
    assert_eq!(scheduler.pending_count(), 0);
    assert_eq!(
        circuit
            .read_pin(&Endpoint {
                instance_id: id,
                pin_id: "z".into()
            })
            .unwrap(),
        LogicLevel::HighZ
    );

    // 2. Memory exhaustion fails closed.
    let (mut circuit, mut scheduler, id) = setup_circuit(&manifest);
    let mem_script = r#"
        function on_init()
            local t = {}
            for i = 1, 20000 do
                t[i] = string.rep("x", 1024)
            end
        end
    "#;
    let mut mem_host = LuaSandboxInstance::new(id, &manifest, mem_script).unwrap();
    assert!(mem_host.call_init(&mut circuit, &mut scheduler).is_err());
    assert!(mem_host.fault().is_some());

    // 3. Timer flood (> 64 pending timers) fails closed and cancels queued timers.
    let (mut circuit, mut scheduler, id) = setup_circuit(&manifest);
    let flood_script = r#"
        function cb() end
        function on_init()
            for i = 1, 70 do
                nodus.schedule_timer(i * 10, "cb")
            end
        end
    "#;
    let mut flood_host = LuaSandboxInstance::new(id, &manifest, flood_script).unwrap();
    let err = flood_host
        .call_init(&mut circuit, &mut scheduler)
        .unwrap_err();
    assert!(err.message.contains("timer limit"));
    assert_eq!(scheduler.pending_count(), 0);
}
