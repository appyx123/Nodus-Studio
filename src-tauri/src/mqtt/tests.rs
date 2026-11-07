use super::packet::*;
use super::types::*;
use super::MqttBridgeController;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

#[test]
fn packet_encoding_and_decoding_round_trip() {
    // CONNECT packet
    let connect_bytes = encode_connect("test-client");
    assert!(!connect_bytes.is_empty());
    assert_eq!(connect_bytes[0], 0x10);

    // CONNACK packet decoding
    let connack_bytes = vec![0x20, 0x02, 0x00, 0x00];
    let (pkt, consumed) = decode_packet(&connack_bytes).unwrap().unwrap();
    assert_eq!(consumed, 4);
    assert_eq!(
        pkt,
        MqttPacket::Connack {
            session_present: false,
            code: 0
        }
    );

    // PUBLISH packet
    let pub_bytes = encode_publish("sensor/temp", b"25.5", 0, None);
    let (pkt, consumed) = decode_packet(&pub_bytes).unwrap().unwrap();
    assert_eq!(consumed, pub_bytes.len());
    match pkt {
        MqttPacket::Publish {
            topic,
            payload,
            packet_id,
        } => {
            assert_eq!(topic, "sensor/temp");
            assert_eq!(payload, b"25.5");
            assert_eq!(packet_id, None);
        }
        _ => panic!("Expected PUBLISH packet"),
    }

    // SUBSCRIBE packet
    let sub_bytes = encode_subscribe("actuator/servo", 42, 1);
    assert_eq!(sub_bytes[0], 0x82);

    // SUBACK packet decoding
    let suback_bytes = vec![0x90, 0x03, 0x00, 0x2A, 0x01];
    let (pkt, consumed) = decode_packet(&suback_bytes).unwrap().unwrap();
    assert_eq!(consumed, 5);
    assert_eq!(
        pkt,
        MqttPacket::Suback {
            packet_id: 42,
            return_code: 1
        }
    );
}

#[test]
fn mqtt_bridge_connects_publishes_and_receives_with_mock_broker() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0);
        assert_eq!(buf[0] >> 4, TYPE_CONNECT);

        // Send CONNACK OK
        stream.write_all(&[0x20, 0x02, 0x00, 0x00]).unwrap();

        // Read SUBSCRIBE
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0);
        assert_eq!(buf[0] >> 4, TYPE_SUBSCRIBE);

        // Send SUBACK
        stream.write_all(&[0x90, 0x03, 0x00, 0x01, 0x00]).unwrap();

        // Send an incoming PUBLISH to the client: topic "nodus/workbench/servo/set", payload "180"
        let mock_pub = encode_publish("nodus/workbench/servo/set", b"180", 0, None);
        stream.write_all(&mock_pub).unwrap();

        // Read outgoing PUBLISH from client
        let n = stream.read(&mut buf).unwrap();
        assert!(n > 0);
        assert_eq!(buf[0] >> 4, TYPE_PUBLISH);
    });

    let controller = MqttBridgeController::default();
    controller.set_config(MqttConfig {
        enabled: true,
        host: "127.0.0.1".into(),
        port,
        client_id: "test-client".into(),
        topic_prefix: "nodus/workbench".into(),
        qos: 0,
    });

    // Wait up to 1 second for connection
    for _ in 0..20 {
        if controller.status().connected {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(controller.status().connected);

    // Publish a sensor reading
    controller.publish("nodus/workbench/dht22/temperature".into(), "26.4".into());

    // Wait for incoming message to be queued
    let mut received = Vec::new();
    for _ in 0..20 {
        received = controller.drain_incoming();
        if !received.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].topic, "nodus/workbench/servo/set");
    assert_eq!(received[0].payload, "180");

    controller.set_config(MqttConfig {
        enabled: false,
        ..controller.config()
    });

    let _ = server_handle.join();
}
