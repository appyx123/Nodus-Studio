use super::*;

fn ep(inst: &str, pin: &str) -> Endpoint {
    Endpoint {
        instance_id: inst.into(),
        pin_id: pin.into(),
    }
}

fn sample_frame(seq: u64) -> SimulationFrame {
    let mut visuals = BTreeMap::new();
    visuals.insert("brightness".to_string(), 0.75);
    SimulationFrame {
        seq,
        run_state: RunState::Running,
        sim_time: SimTime::from_cycles_16mhz(16_000_000).unwrap(),
        speed_ratio: 0.5,
        instances: vec![InstanceVisualSnapshot {
            instance_id: "node-a".into(),
            faulted: false,
            visuals,
        }],
        nets: vec![NetState {
            endpoints: vec![ep("node-a", "A"), ep("node-b", "K")],
            state: LogicLevel::Unknown,
            conflict: true,
            drivers: vec![],
        }],
    }
}

#[test]
fn binary_frame_round_trips_with_exact_header_layout() {
    let frame = sample_frame(42);
    let bytes = encode_frame(&frame).unwrap();
    assert_eq!(&bytes[0..4], b"NDFR");
    assert_eq!(bytes[4], FRAME_PROTOCOL_VERSION);
    assert_eq!(bytes[5], 1);
    assert_eq!(u16::from_le_bytes([bytes[6], bytes[7]]), FLAG_HAS_CONFLICT);
    assert_eq!(u64::from_le_bytes(bytes[8..16].try_into().unwrap()), 42);
    assert_eq!(
        u64::from_le_bytes(bytes[16..24].try_into().unwrap()),
        2_000_000_000
    );
    assert_eq!(decode_frame(&bytes).unwrap(), frame);
}

#[test]
fn decoder_rejects_truncated_unknown_version_bad_magic_and_trailing_bytes() {
    let bytes = encode_frame(&sample_frame(1)).unwrap();
    for cut in 0..bytes.len() {
        assert!(decode_frame(&bytes[..cut]).is_err(), "cut at {cut}");
    }
    let mut bad_version = bytes.clone();
    bad_version[4] = 99;
    assert!(decode_frame(&bad_version).is_err());
    let mut bad_magic = bytes.clone();
    bad_magic[0] = b'X';
    assert!(decode_frame(&bad_magic).is_err());
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_frame(&trailing).is_err());
    let mut bad_state = bytes;
    bad_state[5] = 9;
    assert!(decode_frame(&bad_state).is_err());
}

#[test]
fn encoder_rejects_non_finite_values_and_over_limit_frames() {
    let mut frame = sample_frame(1);
    frame.instances[0]
        .visuals
        .insert("brightness".into(), f64::NAN);
    assert!(encode_frame(&frame).is_err());

    let mut frame = sample_frame(1);
    frame.speed_ratio = f32::INFINITY;
    assert!(encode_frame(&frame).is_err());

    let mut frame = sample_frame(1);
    for i in 0..=MAX_FRAME_INSTANCES {
        frame.instances.push(InstanceVisualSnapshot {
            instance_id: format!("node-{i}"),
            faulted: false,
            visuals: BTreeMap::new(),
        });
    }
    assert!(encode_frame(&frame).is_err());
}

#[test]
fn bounded_queue_keeps_only_latest_frame_without_blocking_and_resyncs() {
    let streamer = FrameStreamer::default();
    for seq in 1..=1_000u64 {
        streamer.publish(&sample_frame(seq)).unwrap();
    }
    assert_eq!(streamer.replaced_frames(), 999);
    let first = decode_frame(&streamer.read_latest_bytes().unwrap()).unwrap();
    assert_eq!(first.seq, 1_000);
    // Queue now empty; a reader still receives the full cached snapshot for resync.
    let resync = decode_frame(&streamer.read_latest_bytes().unwrap()).unwrap();
    assert_eq!(resync, first);
    assert_eq!(streamer.next_seq(), 1);
    assert_eq!(streamer.next_seq(), 2);
}
