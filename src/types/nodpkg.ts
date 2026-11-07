export type PinType = "input" | "output" | "bidirectional" | "power" | "ground" | "nc";

export interface Pin {
  id: string;
  name: string;
  type: PinType;
  svg_anchor: string;
}

export interface SliderControl {
  id: string;
  label: string;
  type: "slider";
  min: number;
  max: number;
  step: number;
  default: number;
}

export type VisualProperty = "opacity" | "rotation";

export interface NumberVisual {
  id: string;
  type: "number";
  target: string;
  property: VisualProperty;
  min: number;
  max: number;
  default: number;
}

export type McuBackend = "avr_atmega328p";

export interface McuConfig {
  backend: McuBackend;
}

export interface ComponentManifest {
  schema_version: 1;
  compatibility: { engine_api: 1 };
  component: {
    id: string;
    name: string;
    category: string;
    author: string;
    version: string;
    description: string;
  };
  electrical: { operating_voltage_min: number; operating_voltage_max: number };
  pins: Pin[];
  controls: SliderControl[];
  visuals: NumberVisual[];
  mcu?: McuConfig;
}

export interface ComponentPackage {
  manifest: ComponentManifest;
  svg: string;
}

export interface PackageDiagnostic {
  path: string;
  message: string;
}

export interface RegistryCatalog {
  packages: ComponentPackage[];
  diagnostics: PackageDiagnostic[];
}

export type WireColor = "green" | "blue" | "amber" | "red" | "violet";

export interface Endpoint {
  instance_id: string;
  pin_id: string;
}

export type CircuitEdit =
  | {
      type: "add_instance";
      instance_id: string;
      component_id: string;
      component_version: string;
    }
  | {
      type: "remove_instance";
      instance_id: string;
    }
  | {
      type: "connect";
      wire_id: string;
      a: Endpoint;
      b: Endpoint;
      color: WireColor;
    }
  | {
      type: "remove_wire";
      wire_id: string;
    }
  | {
      type: "set_wire_color";
      wire_id: string;
      color: WireColor;
    };

export interface InstanceSnapshot {
  instance_id: string;
  component_id: string;
  component_version: string;
  pins: string[];
}

export interface Wire {
  wire_id: string;
  a: Endpoint;
  b: Endpoint;
  color: WireColor;
}

export interface Net {
  endpoints: Endpoint[];
}

export interface CircuitSnapshot {
  instances: InstanceSnapshot[];
  wires: Wire[];
  nets: Net[];
}
