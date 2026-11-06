import type { Pin } from "../../types/nodpkg";
import type { Point } from "./geometry";

export interface InstanceGraphic {
  element: SVGSVGElement;
  width: number;
  height: number;
  viewBoxOrigin: Point;
  pinPositions: Map<string, Point>;
}

function elementCenter(element: Element): Point {
  const num = (attr: string) => Number(element.getAttribute(attr) ?? "0") || 0;
  const tag = element.localName;
  if (tag === "circle" || tag === "ellipse") {
    return { x: num("cx"), y: num("cy") };
  }
  if (tag === "rect") {
    return { x: num("x") + num("width") / 2, y: num("y") + num("height") / 2 };
  }
  if (tag === "line") {
    return { x: (num("x1") + num("x2")) / 2, y: (num("y1") + num("y2")) / 2 };
  }
  return { x: num("x"), y: num("y") };
}

/** Input must be the normalized SVG returned by the Rust package validator. */
export function instanceGraphic(source: string, instanceId: string, pins: Pin[] = []): InstanceGraphic {
  if (!/^node-[a-f0-9-]{36}$/.test(instanceId)) throw new Error("Invalid canvas instance ID");
  const parsed = new DOMParser().parseFromString(source, "image/svg+xml");
  const root = parsed.documentElement;
  if (root.localName !== "svg" || root.namespaceURI !== "http://www.w3.org/2000/svg"
    || parsed.querySelector("parsererror")) throw new Error("Invalid validated component graphic");
  const viewBox = (root.getAttribute("viewBox") ?? "").trim().split(/[\s,]+/).map(Number);
  if (viewBox.length !== 4 || !viewBox.every(Number.isFinite) || viewBox[2] <= 0 || viewBox[3] <= 0) {
    throw new Error("Component graphic requires a valid viewBox");
  }
  const element = document.importNode(root, true) as unknown as SVGSVGElement;
  const pinByAnchor = new Map(pins.map((pin) => [pin.svg_anchor, pin]));
  const pinPositions = new Map<string, Point>();
  const ids = new Map<string, string>();
  for (const item of [element, ...element.querySelectorAll("[id]")]) {
    const id = item.getAttribute("id");
    if (id) {
      const namespaced = `${instanceId}--${id}`;
      ids.set(id, namespaced);
      item.setAttribute("data-original-id", id);
      item.setAttribute("id", namespaced);
      const pin = pinByAnchor.get(id);
      if (pin) {
        pinPositions.set(pin.id, elementCenter(item));
        item.setAttribute("data-pin-id", pin.id);
        item.setAttribute("data-pin-type", pin.type);
        item.setAttribute("data-instance-id", instanceId);
        item.setAttribute("role", "button");
        item.setAttribute("tabindex", "0");
        item.setAttribute("aria-label", `${pin.name} (${pin.id}) on ${instanceId}`);
        item.classList.add("canvas-pin");
      }
    }
  }
  // Current Rust SVG contract rejects references; keep local ID references isolated if it expands.
  for (const item of [element, ...element.querySelectorAll("*")]) {
    for (const attribute of [...item.attributes]) {
      let value = attribute.value.replace(/url\(#([A-Za-z][A-Za-z0-9_-]*)\)/g,
        (match, id: string) => ids.has(id) ? `url(#${ids.get(id)})` : match);
      if (attribute.localName === "href" && value.startsWith("#") && ids.has(value.slice(1))) {
        value = `#${ids.get(value.slice(1))}`;
      }
      if (attribute.name === "aria-labelledby" || attribute.name === "aria-describedby") {
        value = value.split(/\s+/).map((id) => ids.get(id) ?? id).join(" ");
      }
      if (value !== attribute.value) item.setAttributeNS(attribute.namespaceURI, attribute.name, value);
    }
  }
  element.setAttribute("width", String(viewBox[2]));
  element.setAttribute("height", String(viewBox[3]));
  element.setAttribute("x", "0");
  element.setAttribute("y", "0");

  return {
    element,
    width: viewBox[2],
    height: viewBox[3],
    viewBoxOrigin: { x: viewBox[0], y: viewBox[1] },
    pinPositions,
  };
}
