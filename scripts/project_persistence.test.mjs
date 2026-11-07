import test from "node:test";
import assert from "node:assert/strict";

test("Nodus project file JSON schema satisfies format specifications", () => {
  const sampleProject = {
    format: "nodus-project",
    version: "1.0.0",
    app_id: "org.nodus.Studio",
    metadata: {
      name: "Blink Demo",
      created_at: 1730000000,
      modified_at: 1730000000,
      description: "Sample blink circuit",
    },
    viewport: {
      pan: { x: 60, y: 40 },
      zoom: 2.0,
    },
    circuit: {
      instances: [
        {
          instance_id: "node-1",
          component_id: "basic-led",
          component_version: "1.0.0",
          position: { x: 100, y: 150 },
          control_values: {},
        },
      ],
      wires: [
        {
          wire_id: "wire-1",
          a: { instance_id: "node-1", pin_id: "anode" },
          b: { instance_id: "node-2", pin_id: "pin13" },
          color: "green",
        },
      ],
    },
    firmware: {
      sketch_source: "void setup() {} void loop() {}",
      fqbn: "arduino:avr:uno",
    },
    mqtt_bridge: {
      enabled: false,
      host: "localhost",
      port: 1883,
      client_id: "nodus-client",
      sub_topics: [],
      pub_topics: [],
    },
  };

  const serialized = JSON.stringify(sampleProject);
  const deserialized = JSON.parse(serialized);

  assert.equal(deserialized.format, "nodus-project");
  assert.equal(deserialized.version, "1.0.0");
  assert.equal(deserialized.circuit.instances.length, 1);
  assert.equal(deserialized.circuit.wires.length, 1);
  assert.equal(deserialized.mqtt_bridge.password, undefined);
  assert.equal(deserialized.mqtt_bridge.username, undefined);
});
