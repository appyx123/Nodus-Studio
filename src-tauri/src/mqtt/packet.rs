pub const TYPE_CONNECT: u8 = 1;
pub const TYPE_CONNACK: u8 = 2;
pub const TYPE_PUBLISH: u8 = 3;
pub const TYPE_PUBACK: u8 = 4;
pub const TYPE_SUBSCRIBE: u8 = 8;
pub const TYPE_SUBACK: u8 = 9;
pub const TYPE_PINGREQ: u8 = 12;
pub const TYPE_PINGRESP: u8 = 13;
pub const TYPE_DISCONNECT: u8 = 14;

#[derive(Debug, PartialEq, Eq)]
pub enum MqttPacket {
    Connack {
        session_present: bool,
        code: u8,
    },
    Publish {
        topic: String,
        payload: Vec<u8>,
        packet_id: Option<u16>,
    },
    Puback {
        packet_id: u16,
    },
    Suback {
        packet_id: u16,
        return_code: u8,
    },
    Pingresp,
}

fn encode_remaining_length(len: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut x = len;
    loop {
        let mut encoded_byte = (x % 128) as u8;
        x /= 128;
        if x > 0 {
            encoded_byte |= 128;
        }
        bytes.push(encoded_byte);
        if x == 0 {
            break;
        }
    }
    bytes
}

fn decode_remaining_length(bytes: &[u8]) -> Result<Option<(usize, usize)>, String> {
    let mut multiplier = 1usize;
    let mut value = 0usize;
    let mut len_bytes = 0usize;

    for &b in bytes {
        len_bytes += 1;
        value += (b as usize & 127) * multiplier;
        if multiplier > 128 * 128 * 128 {
            return Err("Malformed remaining length".into());
        }
        multiplier *= 128;
        if (b & 128) == 0 {
            return Ok(Some((value, len_bytes)));
        }
    }
    Ok(None) // Incomplete length
}

fn write_str(buf: &mut Vec<u8>, s: &str) {
    let len = s.len() as u16;
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(s.as_bytes());
}

pub fn encode_connect(client_id: &str) -> Vec<u8> {
    let mut var_header = Vec::new();
    write_str(&mut var_header, "MQTT");
    var_header.push(4); // Protocol level v3.1.1
    var_header.push(2); // Clean session
    var_header.extend_from_slice(&60u16.to_be_bytes()); // Keep alive 60s
    write_str(&mut var_header, client_id);

    let mut pkt = vec![TYPE_CONNECT << 4];
    pkt.extend(encode_remaining_length(var_header.len()));
    pkt.extend(var_header);
    pkt
}

pub fn encode_publish(topic: &str, payload: &[u8], qos: u8, packet_id: Option<u16>) -> Vec<u8> {
    let mut body = Vec::new();
    write_str(&mut body, topic);
    if qos > 0 {
        if let Some(pid) = packet_id {
            body.extend_from_slice(&pid.to_be_bytes());
        }
    }
    body.extend_from_slice(payload);

    let header_byte = (TYPE_PUBLISH << 4) | ((qos & 0x03) << 1);
    let mut pkt = vec![header_byte];
    pkt.extend(encode_remaining_length(body.len()));
    pkt.extend(body);
    pkt
}

pub fn encode_subscribe(topic: &str, packet_id: u16, qos: u8) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&packet_id.to_be_bytes());
    write_str(&mut body, topic);
    body.push(qos & 0x03);

    let mut pkt = vec![(TYPE_SUBSCRIBE << 4) | 0x02];
    pkt.extend(encode_remaining_length(body.len()));
    pkt.extend(body);
    pkt
}

pub fn encode_pingreq() -> [u8; 2] {
    [TYPE_PINGREQ << 4, 0]
}

pub fn encode_disconnect() -> [u8; 2] {
    [TYPE_DISCONNECT << 4, 0]
}

pub fn decode_packet(bytes: &[u8]) -> Result<Option<(MqttPacket, usize)>, String> {
    if bytes.is_empty() {
        return Ok(None);
    }
    let first = bytes[0];
    let pkt_type = first >> 4;
    let flags = first & 0x0F;

    let Some((rem_len, len_bytes)) = decode_remaining_length(&bytes[1..])? else {
        return Ok(None);
    };
    let total_len = 1 + len_bytes + rem_len;
    if bytes.len() < total_len {
        return Ok(None); // Need more bytes
    }
    let payload = &bytes[1 + len_bytes..total_len];

    let packet = match pkt_type {
        TYPE_CONNACK => {
            if payload.len() < 2 {
                return Err("CONNACK payload too short".into());
            }
            MqttPacket::Connack {
                session_present: (payload[0] & 1) != 0,
                code: payload[1],
            }
        }
        TYPE_PUBLISH => {
            if payload.len() < 2 {
                return Err("PUBLISH payload too short".into());
            }
            let topic_len = u16::from_be_bytes([payload[0], payload[1]]) as usize;
            if payload.len() < 2 + topic_len {
                return Err("PUBLISH topic truncated".into());
            }
            let topic = String::from_utf8_lossy(&payload[2..2 + topic_len]).into_owned();
            let mut offset = 2 + topic_len;
            let qos = (flags >> 1) & 0x03;
            let packet_id = if qos > 0 {
                if payload.len() < offset + 2 {
                    return Err("PUBLISH packet ID truncated".into());
                }
                let pid = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
                offset += 2;
                Some(pid)
            } else {
                None
            };
            let body = payload[offset..].to_vec();
            MqttPacket::Publish {
                topic,
                payload: body,
                packet_id,
            }
        }
        TYPE_PUBACK => {
            if payload.len() < 2 {
                return Err("PUBACK payload too short".into());
            }
            let packet_id = u16::from_be_bytes([payload[0], payload[1]]);
            MqttPacket::Puback { packet_id }
        }
        TYPE_SUBACK => {
            if payload.len() < 3 {
                return Err("SUBACK payload too short".into());
            }
            let packet_id = u16::from_be_bytes([payload[0], payload[1]]);
            MqttPacket::Suback {
                packet_id,
                return_code: payload[2],
            }
        }
        TYPE_PINGRESP => MqttPacket::Pingresp,
        _ => return Err(format!("Unsupported packet type: {pkt_type}")),
    };

    Ok(Some((packet, total_len)))
}
