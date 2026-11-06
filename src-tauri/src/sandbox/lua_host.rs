use super::manifest::{Manifest, NumberVisual, Pin, PinType, SliderControl};
use crate::engine::{
    CircuitState as C, Endpoint, EventPayload, LogicLevel as L, PinMode, Scheduler as S, SimTime,
};
use mlua::{HookTriggers, IntoLuaMulti, Lua, LuaOptions, StdLib, Table, Value, VmState};
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

pub const MAX_LUA_MEMORY_BYTES: usize = 256 * 1024;
pub const MAX_INSTRUCTIONS_PER_CALL: usize = 10_000;
pub const HOOK_INSTRUCTION_GRANULARITY: u32 = 250;
pub const MAX_TIMERS_PER_INSTANCE: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstanceDiagnostic {
    pub instance_id: String,
    pub component_id: String,
    pub callback: String,
    pub message: String,
}

pub type HostResult = Result<(), InstanceDiagnostic>;

#[derive(Debug, Clone)]
enum HostOp {
    SetPinMode(String, PinMode),
    SetPin(String, bool),
    ScheduleTimer(u64, SimTime, String),
    CancelTimer(u64),
    SetVisual(String, f64),
}

struct BridgeState {
    pins: BTreeMap<String, Pin>,
    visuals: BTreeMap<String, NumberVisual>,
    resolved_pins: BTreeMap<String, L>,
    active_timers: BTreeSet<u64>,
    next_local_handle: u64,
    ops: Vec<HostOp>,
}

fn lua_err(msg: impl Into<String>) -> mlua::Error {
    mlua::Error::RuntimeError(msg.into())
}

impl BridgeState {
    fn new(m: &Manifest) -> Self {
        let pins = BTreeMap::from_iter(m.pins.iter().map(|p| (p.id.clone(), p.clone())));
        let visuals = BTreeMap::from_iter(m.visuals.iter().map(|v| (v.id.clone(), v.clone())));
        let resolved_pins = BTreeMap::from_iter(pins.keys().map(|k| (k.clone(), L::HighZ)));
        Self {
            pins,
            visuals,
            resolved_pins,
            active_timers: BTreeSet::new(),
            next_local_handle: 1,
            ops: Vec::new(),
        }
    }

    fn check_pin(&self, id: &str) -> mlua::Result<()> {
        let Some(p) = self.pins.get(id) else {
            return Err(lua_err(format!("Unknown pin: {id}")));
        };
        if matches!(p.kind, PinType::Nc) {
            return Err(lua_err(format!("NC pin: {id}")));
        }
        Ok(())
    }

    fn set_pin_mode(&mut self, pin_id: String, mode: &str) -> mlua::Result<()> {
        self.check_pin(&pin_id)?;
        let parsed = match mode {
            "INPUT" => PinMode::Input,
            "INPUT_PULLUP" => PinMode::InputPullup,
            "INPUT_PULLDOWN" => PinMode::InputPulldown,
            "OUTPUT" => PinMode::Output,
            "OPEN_DRAIN" => PinMode::OpenDrain,
            _ => return Err(lua_err(format!("Unsupported mode: {mode}"))),
        };
        self.ops.push(HostOp::SetPinMode(pin_id, parsed));
        Ok(())
    }

    fn set_pin(&mut self, pin_id: String, level: Value) -> mlua::Result<()> {
        self.check_pin(&pin_id)?;
        let high = match level {
            Value::Boolean(flag) => flag,
            Value::String(text) => match text.to_str()?.as_ref() {
                "HIGH" => true,
                "LOW" => false,
                other => return Err(lua_err(format!("Bad state: {other}"))),
            },
            _ => return Err(lua_err("Expected HIGH/LOW/bool")),
        };
        self.ops.push(HostOp::SetPin(pin_id, high));
        Ok(())
    }

    fn read_pin(&self, pin_id: &str) -> mlua::Result<String> {
        self.check_pin(pin_id)?;
        let lvl = self.resolved_pins.get(pin_id).copied().unwrap_or(L::HighZ);
        Ok(lvl.as_str().to_owned())
    }

    fn schedule_timer(&mut self, delay_ns: i64, cb: String) -> mlua::Result<u64> {
        if delay_ns < 0 {
            return Err(lua_err("delay_ns must be >= 0"));
        }
        if self.active_timers.len() >= MAX_TIMERS_PER_INSTANCE {
            return Err(lua_err("Instance timer limit (64) exceeded"));
        }
        let delay = SimTime::from_ns(delay_ns as u64).map_err(|e| lua_err(e.to_string()))?;
        let handle = self.next_local_handle;
        self.next_local_handle += 1;
        self.active_timers.insert(handle);
        self.ops.push(HostOp::ScheduleTimer(handle, delay, cb));
        Ok(handle)
    }

    fn cancel_timer(&mut self, handle: u64) -> bool {
        let removed = self.active_timers.remove(&handle);
        if removed {
            self.ops.push(HostOp::CancelTimer(handle));
        }
        removed
    }

    fn set_visual(&mut self, id: String, val: f64) -> mlua::Result<()> {
        let Some(s) = self.visuals.get(&id) else {
            return Err(lua_err(format!("Unknown: {id}")));
        };
        if !val.is_finite() || val < s.min || val > s.max {
            return Err(lua_err(format!("Visual {id} out of range")));
        }
        self.ops.push(HostOp::SetVisual(id, val));
        Ok(())
    }
}

pub struct LuaSandboxInstance {
    instance_id: String,
    component_id: String,
    lua: Lua,
    bridge: Rc<RefCell<BridgeState>>,
    instructions: Arc<AtomicUsize>,
    controls: BTreeMap<String, SliderControl>,
    control_values: BTreeMap<String, f64>,
    visual_values: BTreeMap<String, f64>,
    handle_map: BTreeMap<u64, u64>,
    fault: Option<InstanceDiagnostic>,
}

impl LuaSandboxInstance {
    pub fn new(id: String, m: &Manifest, source: &str) -> Result<Self, InstanceDiagnostic> {
        let cid = m.component.id.clone();
        let load_err = |err: mlua::Error| InstanceDiagnostic {
            instance_id: id.clone(),
            component_id: cid.clone(),
            callback: "load".into(),
            message: err.to_string(),
        };
        let libs = StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8;
        let lua = Lua::new_with(libs, LuaOptions::default()).map_err(load_err)?;
        let _ = lua.set_memory_limit(MAX_LUA_MEMORY_BYTES);
        let instructions = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&instructions);
        let triggers = HookTriggers::new().every_nth_instruction(HOOK_INSTRUCTION_GRANULARITY);
        lua.set_hook(triggers, move |_, _| {
            let step = HOOK_INSTRUCTION_GRANULARITY as usize;
            if counter.fetch_add(step, Ordering::Relaxed) + step > MAX_INSTRUCTIONS_PER_CALL {
                Err(lua_err("Instruction budget exceeded in callback"))
            } else {
                Ok(VmState::Continue)
            }
        })
        .map_err(load_err)?;
        sanitize_globals(&lua).map_err(load_err)?;

        let controls = BTreeMap::from_iter(m.controls.iter().map(|c| (c.id.clone(), c.clone())));
        let control_values =
            BTreeMap::from_iter(m.controls.iter().map(|c| (c.id.clone(), c.default)));
        let visual_values =
            BTreeMap::from_iter(m.visuals.iter().map(|v| (v.id.clone(), v.default)));
        let bridge = Rc::new(RefCell::new(BridgeState::new(m)));
        install_nodus_api(&lua, Rc::clone(&bridge)).map_err(load_err)?;
        instructions.store(0, Ordering::Relaxed);
        let chunk_name = format!("@{cid}/logic.lua");
        lua.load(source)
            .set_name(chunk_name)
            .exec()
            .map_err(load_err)?;

        Ok(Self {
            instance_id: id,
            component_id: cid,
            lua,
            bridge,
            instructions,
            controls,
            control_values,
            visual_values,
            handle_map: BTreeMap::new(),
            fault: None,
        })
    }

    pub fn fault(&self) -> Option<&InstanceDiagnostic> {
        self.fault.as_ref()
    }

    pub fn visual_values(&self) -> &BTreeMap<String, f64> {
        &self.visual_values
    }

    pub fn control_values(&self) -> &BTreeMap<String, f64> {
        &self.control_values
    }

    pub fn call_init(&mut self, c: &mut C, s: &mut S) -> HostResult {
        self.invoke("on_init", (), c, s)
    }

    pub fn call_control_change(&mut self, id: &str, v: f64, c: &mut C, s: &mut S) -> HostResult {
        let cb = "on_control_change";
        let Some(spec) = self.controls.get(id) else {
            return Err(self.fail(cb, format!("Unknown control: {id}"), c, s));
        };
        if !v.is_finite() || v < spec.min || v > spec.max {
            return Err(self.fail(cb, format!("Control {id} out of range"), c, s));
        }
        self.control_values.insert(id.to_owned(), v);
        self.invoke(cb, (id.to_owned(), v), c, s)
    }

    pub fn call_pin_edge(&mut self, p: &str, l: L, t: SimTime, c: &mut C, s: &mut S) -> HostResult {
        let a = (p.to_owned(), l.as_str().to_owned(), t.as_ns_floor());
        self.invoke("on_pin_edge", a, c, s)
    }

    pub fn call_timer(&mut self, h: u64, cb: &str, t: SimTime, c: &mut C, s: &mut S) -> HostResult {
        if let Some((&local, _)) = self.handle_map.iter().find(|(_, &sh)| sh == h) {
            self.handle_map.remove(&local);
            self.bridge.borrow_mut().active_timers.remove(&local);
        }
        self.invoke(cb, t.as_ns_floor(), c, s)
    }

    fn sync_resolved_pins(&self, circuit: &C) {
        let mut guard = self.bridge.borrow_mut();
        let pin_ids: Vec<String> = guard.pins.keys().cloned().collect();
        for pin_id in pin_ids {
            let ep = Endpoint {
                instance_id: self.instance_id.clone(),
                pin_id: pin_id.clone(),
            };
            let lvl = circuit.read_pin(&ep).unwrap_or(L::HighZ);
            guard.resolved_pins.insert(pin_id, lvl);
        }
        guard.ops.clear();
    }

    fn invoke<A: IntoLuaMulti>(&mut self, cb: &str, a: A, c: &mut C, s: &mut S) -> HostResult {
        if let Some(diag) = &self.fault {
            return Err(diag.clone());
        }
        self.sync_resolved_pins(c);
        let val: Value = self
            .lua
            .globals()
            .get(cb)
            .map_err(|e| self.fail(cb, e.to_string(), c, s))?;
        match val {
            Value::Nil => self.commit_ops(cb, c, s),
            Value::Function(func) => {
                self.instructions.store(0, Ordering::Relaxed);
                func.call::<()>(a)
                    .map_err(|e| self.fail(cb, e.to_string(), c, s))?;
                self.commit_ops(cb, c, s)
            }
            _ => Err(self.fail(cb, format!("Invalid callback: {cb}"), c, s)),
        }
    }

    fn commit_ops(&mut self, cb: &str, c: &mut C, s: &mut S) -> HostResult {
        let ops = std::mem::take(&mut self.bridge.borrow_mut().ops);
        for op in ops {
            let id = self.instance_id.clone();
            let ep = |pin_id| Endpoint {
                instance_id: id.clone(),
                pin_id,
            };
            let res = match op {
                HostOp::SetPinMode(p, mode) => c.set_pin_mode(&ep(p), mode),
                HostOp::SetPin(p, high) => c.set_pin_output(&ep(p), high),
                HostOp::ScheduleTimer(local, delay, target) => {
                    let ev = EventPayload::LuaTimer {
                        instance_id: id,
                        callback: target,
                    };
                    s.schedule_after(delay, ev).map(|sh| {
                        self.handle_map.insert(local, sh);
                    })
                }
                HostOp::CancelTimer(local) => {
                    if let Some(sh) = self.handle_map.remove(&local) {
                        s.cancel(sh);
                    }
                    Ok(())
                }
                HostOp::SetVisual(f, v) => {
                    self.visual_values.insert(f, v);
                    Ok(())
                }
            };
            if let Err(err) = res {
                return Err(self.fail(cb, err.to_string(), c, s));
            }
        }
        Ok(())
    }

    fn fail(&mut self, cb: &str, msg: String, c: &mut C, s: &mut S) -> InstanceDiagnostic {
        let _ = c.release_instance_pins(&self.instance_id);
        s.cancel_instance(&self.instance_id);
        self.handle_map.clear();
        let mut guard = self.bridge.borrow_mut();
        guard.active_timers.clear();
        guard.ops.clear();
        let diag = InstanceDiagnostic {
            instance_id: self.instance_id.clone(),
            component_id: self.component_id.clone(),
            callback: cb.to_owned(),
            message: msg,
        };
        self.fault = Some(diag.clone());
        diag
    }
}

fn sanitize_globals(lua: &Lua) -> mlua::Result<()> {
    let globals = lua.globals();
    for name in "io os package debug coroutine dofile load loadfile require collectgarbage"
        .split_whitespace()
    {
        globals.set(name, Value::Nil)?;
    }
    if let Ok(math) = globals.get::<Table>("math") {
        math.set("random", Value::Nil)?;
        math.set("randomseed", Value::Nil)?;
    }
    Ok(())
}

fn install_nodus_api(lua: &Lua, bridge: Rc<RefCell<BridgeState>>) -> mlua::Result<()> {
    let nodus = lua.create_table()?;
    let b = Rc::clone(&bridge);
    let f =
        lua.create_function(move |_, (p, m): (String, String)| b.borrow_mut().set_pin_mode(p, &m))?;
    nodus.set("set_pin_mode", f)?;
    let b = Rc::clone(&bridge);
    let f = lua.create_function(move |_, (p, l): (String, Value)| b.borrow_mut().set_pin(p, l))?;
    nodus.set("set_pin", f)?;
    let b = Rc::clone(&bridge);
    let f = lua.create_function(move |_, p: String| b.borrow().read_pin(&p))?;
    nodus.set("read_pin", f)?;
    let b = Rc::clone(&bridge);
    let f = lua.create_function(move |ctx, (delay_ns, cb): (i64, String)| {
        if !matches!(ctx.globals().get::<Value>(cb.as_str())?, Value::Function(_)) {
            return Err(lua_err(format!("Missing callback: {cb}")));
        }
        b.borrow_mut().schedule_timer(delay_ns, cb)
    })?;
    nodus.set("schedule_timer", f)?;
    let b = Rc::clone(&bridge);
    let f = lua.create_function(move |_, h: u64| Ok(b.borrow_mut().cancel_timer(h)))?;
    nodus.set("cancel_timer", f)?;
    let b = Rc::clone(&bridge);
    let f = lua.create_function(move |_, (k, v): (String, f64)| b.borrow_mut().set_visual(k, v))?;
    nodus.set("set_visual_state", f)?;
    lua.globals().set("nodus", nodus)
}

#[cfg(test)]
#[path = "lua_host_tests.rs"]
mod tests;
