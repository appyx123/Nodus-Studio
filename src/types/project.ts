import type { WireColor } from "./nodpkg";

export interface Point {
  x: number;
  y: number;
}

export interface ProjectMetadata {
  name: string;
  created_at: number;
  modified_at: number;
  description?: string;
}

export interface InstanceRecord {
  instance_id: string;
  component_id: string;
  component_version: string;
  position: Point;
  control_values?: Record<string, number>;
}

export interface EndpointRecord {
  instance_id: string;
  pin_id: string;
}

export interface WireRecord {
  wire_id: string;
  a: EndpointRecord;
  b: EndpointRecord;
  color: WireColor;
}

export interface CircuitSection {
  instances: InstanceRecord[];
  wires: WireRecord[];
}

export interface ViewportSection {
  pan: Point;
  zoom: number;
}

export interface FirmwareSection {
  sketch_source: string;
  fqbn: string;
}

export interface MqttBridgeSection {
  enabled: boolean;
  host: string;
  port: number;
  client_id: string;
  sub_topics: string[];
  pub_topics: string[];
}

export interface NodusProjectFile {
  format: string;
  version: string;
  app_id: string;
  metadata: ProjectMetadata;
  viewport: ViewportSection;
  circuit: CircuitSection;
  firmware: FirmwareSection;
  mqtt_bridge?: MqttBridgeSection;
}

export type WarningLevel = "info" | "warning" | "error";

export interface ProjectWarning {
  level: WarningLevel;
  message: string;
}

export interface LoadProjectResponse {
  project: NodusProjectFile;
  warnings: ProjectWarning[];
}
