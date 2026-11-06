export interface Point {
  x: number;
  y: number;
}

export interface Viewport extends Point {
  zoom: number;
}

export const GRID = 25.4;
export const MIN_ZOOM = 0.2;
export const MAX_ZOOM = 4;

export function screenToWorld(point: Point, viewport: Viewport): Point {
  return {
    x: (point.x - viewport.x) / viewport.zoom,
    y: (point.y - viewport.y) / viewport.zoom,
  };
}

export function worldToScreen(point: Point, viewport: Viewport): Point {
  return {
    x: point.x * viewport.zoom + viewport.x,
    y: point.y * viewport.zoom + viewport.y,
  };
}

export function snapPoint(point: Point): Point {
  // Grid coordinates have one decimal place; normalize multiplication noise and -0.
  const snap = (value: number) => Number((Math.round(value / GRID) * GRID).toFixed(1)) || 0;
  return { x: snap(point.x), y: snap(point.y) };
}

export function zoomAt(viewport: Viewport, screenPoint: Point, requestedZoom: number): Viewport {
  const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, requestedZoom));
  const world = screenToWorld(screenPoint, viewport);
  return {
    x: screenPoint.x - world.x * zoom,
    y: screenPoint.y - world.y * zoom,
    zoom,
  };
}

export function dragPosition(
  startPosition: Point,
  startScreen: Point,
  currentScreen: Point,
  viewport: Viewport,
): Point {
  return snapPoint({
    x: startPosition.x + (currentScreen.x - startScreen.x) / viewport.zoom,
    y: startPosition.y + (currentScreen.y - startScreen.y) / viewport.zoom,
  });
}

export function pinWorldPoint(
  instancePosition: Point,
  localPin: Point,
  viewBoxOrigin: Point = { x: 0, y: 0 },
): Point {
  const round = (value: number) => Number(value.toFixed(4)) || 0;
  return {
    x: round(instancePosition.x + localPin.x - viewBoxOrigin.x),
    y: round(instancePosition.y + localPin.y - viewBoxOrigin.y),
  };
}

export function manhattanRoute(a: Point, b: Point): Point[] {
  const round = (value: number) => Number(value.toFixed(4)) || 0;
  const start = { x: round(a.x), y: round(a.y) };
  const end = { x: round(b.x), y: round(b.y) };
  if (start.x === end.x || start.y === end.y) {
    return [start, end];
  }
  const midX = round((start.x + end.x) / 2);
  return [
    start,
    { x: midX, y: start.y },
    { x: midX, y: end.y },
    end,
  ];
}

export function manhattanPath(a: Point, b: Point): string {
  return manhattanRoute(a, b)
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x} ${point.y}`)
    .join(" ");
}
