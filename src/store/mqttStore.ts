import { createSignal } from "solid-js";

export interface MqttConfig {
  enabled: boolean;
  host: string;
  port: number;
  client_id: string;
  topic_prefix: string;
  qos: number;
}

export interface MqttStatus {
  connected: boolean;
  last_error: string | null;
  published_count: number;
  received_count: number;
}

export const [mqttOpen, setMqttOpen] = createSignal(false);
