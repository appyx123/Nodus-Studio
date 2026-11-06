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

export interface NumberVisual {
  id: string;
  type: "number";
  target: string;
  property: "opacity";
  min: number;
  max: number;
  default: number;
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
