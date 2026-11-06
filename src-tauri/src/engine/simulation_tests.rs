use super::{
    scheduler::{HALF_NS_PER_16MHZ_CYCLE, HALF_NS_PER_SECOND, MAX_ZERO_ADVANCE_EVENTS},
    test_support::*,
    *,
};

#[test]
fn sixteen_million_cycles_at_16mhz_equal_exactly_one_virtual_second_without_drift() {
    let mut scheduler = Scheduler::default();
    scheduler.start();
    for _ in 0..16_000 {
        scheduler.advance_cycles_16mhz(1_000).unwrap();
    }
    assert_eq!(scheduler.now().half_ns(), HALF_NS_PER_SECOND);
    assert_eq!(scheduler.now().as_ns_floor(), 1_000_000_000);

    // Odd single cycles preserve exact 0.5 ns remainder across step boundaries.
    let mut odd = Scheduler::default();
    odd.start();
    odd.advance_cycles_16mhz(1).unwrap();
    assert_eq!(odd.now().half_ns(), HALF_NS_PER_16MHZ_CYCLE);
    assert_eq!(odd.now().as_ns_floor(), 62);
    odd.advance_cycles_16mhz(1).unwrap();
    assert_eq!(odd.now().half_ns(), 250);
    assert_eq!(odd.now().as_ns_floor(), 125);
}

#[test]
fn equal_timestamp_events_execute_in_stable_fifo_order_and_respect_cancellation() {
    let mut scheduler = Scheduler::default();
    scheduler.start();
    let delay = SimTime::from_ns(100).unwrap();
    let h1 = scheduler
        .schedule_after(
            delay,
            EventPayload::LuaTimer {
                instance_id: node(0),
                callback: "first".into(),
            },
        )
        .unwrap();
    let h2 = scheduler
        .schedule_after(
            delay,
            EventPayload::LuaTimer {
                instance_id: node(0),
                callback: "second".into(),
            },
        )
        .unwrap();
    let h3 = scheduler
        .schedule_after(
            delay,
            EventPayload::LuaTimer {
                instance_id: node(1),
                callback: "third".into(),
            },
        )
        .unwrap();
    assert!(scheduler.cancel(h2));
    assert!(!scheduler.cancel(h2));

    let target = SimTime::from_ns(200).unwrap();
    let e1 = scheduler.pop_next_until(target).unwrap().unwrap();
    let e3 = scheduler.pop_next_until(target).unwrap().unwrap();
    assert_eq!(e1.handle, h1);
    assert_eq!(e3.handle, h3);
    assert!(scheduler.pop_next_until(target).unwrap().is_none());
    assert_eq!(scheduler.now(), target);
}

#[test]
fn lifecycle_transitions_and_reset_clear_pending_timers_and_zero_advance_loops_stop() {
    let mut scheduler = Scheduler::default();
    assert_eq!(scheduler.state(), RunState::Stopped);
    assert!(scheduler.pause().is_err());
    scheduler.start();
    let delay = SimTime::from_ns(50).unwrap();
    scheduler
        .schedule_after(
            delay,
            EventPayload::LuaTimer {
                instance_id: node(0),
                callback: "tick".into(),
            },
        )
        .unwrap();
    scheduler.pause().unwrap();
    assert_eq!(scheduler.state(), RunState::Paused);
    assert!(scheduler.pop_next_until(delay).unwrap().is_none());
    scheduler.resume().unwrap();
    assert!(scheduler.pop_next_until(delay).unwrap().is_some());
    assert_eq!(scheduler.now(), delay);

    for _ in 0..=MAX_ZERO_ADVANCE_EVENTS {
        scheduler
            .schedule_after(
                SimTime::ZERO,
                EventPayload::LuaTimer {
                    instance_id: node(0),
                    callback: "spin".into(),
                },
            )
            .unwrap();
    }
    let mut err_seen = false;
    loop {
        match scheduler.pop_next_until(delay) {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => {
                err_seen = true;
                break;
            }
        }
    }
    assert!(err_seen, "Zero-advance loop must fail closed");

    scheduler.reset();
    assert_eq!(scheduler.state(), RunState::Stopped);
    assert_eq!(scheduler.now(), SimTime::ZERO);
    assert_eq!(scheduler.pending_count(), 0);
}

#[test]
fn digital_net_resolution_covers_floating_pulls_open_drain_and_push_pull_conflict() {
    let mut state = populated(3);
    state
        .edit(connect(0, endpoint(0, "A"), endpoint(1, "A")))
        .unwrap();
    state
        .edit(connect(1, endpoint(1, "A"), endpoint(2, "A")))
        .unwrap();

    // Unconnected / input-only net floats at HIGH_Z, never implicitly LOW.
    assert_eq!(
        state.read_pin(&endpoint(0, "A")).unwrap(),
        LogicLevel::HighZ
    );

    // Weak pull-up resolves to HIGH; weak pull-down alone resolves to LOW; both together -> UNKNOWN.
    state
        .set_pin_mode(&endpoint(0, "A"), PinMode::InputPullup)
        .unwrap();
    assert_eq!(state.read_pin(&endpoint(1, "A")).unwrap(), LogicLevel::High);
    state
        .set_pin_mode(&endpoint(1, "A"), PinMode::InputPulldown)
        .unwrap();
    assert_eq!(
        state.read_pin(&endpoint(2, "A")).unwrap(),
        LogicLevel::Unknown
    );

    // Open-drain released (output_high = true) lets pull-up keep net HIGH; driving low pulls net LOW.
    state
        .set_pin_mode(&endpoint(1, "A"), PinMode::OpenDrain)
        .unwrap();
    state.set_pin_output(&endpoint(1, "A"), true).unwrap();
    assert_eq!(state.read_pin(&endpoint(0, "A")).unwrap(), LogicLevel::High);
    state.set_pin_output(&endpoint(1, "A"), false).unwrap();
    assert_eq!(state.read_pin(&endpoint(0, "A")).unwrap(), LogicLevel::Low);

    // Strong HIGH vs strong LOW push-pull conflict resolves to UNKNOWN with conflict = true.
    state
        .set_pin_mode(&endpoint(2, "A"), PinMode::Output)
        .unwrap();
    state.set_pin_output(&endpoint(2, "A"), true).unwrap();
    let resolved = state
        .resolve_nets()
        .into_iter()
        .find(|net| net.endpoints.contains(&endpoint(0, "A")))
        .unwrap();
    assert_eq!(resolved.state, LogicLevel::Unknown);
    assert!(resolved.conflict);
    assert_eq!(resolved.drivers, vec![endpoint(1, "A"), endpoint(2, "A")]);

    // Releasing an instance returns its pins to input HIGH_Z and clears the conflict.
    state.release_instance_pins(&node(2)).unwrap();
    assert_eq!(state.read_pin(&endpoint(0, "A")).unwrap(), LogicLevel::Low);
    state.reset_all_pins();
    assert_eq!(
        state.read_pin(&endpoint(0, "A")).unwrap(),
        LogicLevel::HighZ
    );
}
