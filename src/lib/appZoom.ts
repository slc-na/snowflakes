const ZOOM_KEY = "sf-app-zoom";
const MIN_ZOOM = 0.6;
const MAX_ZOOM = 2;
const STEP = 0.1;

export function loadAppZoom(): number {
    const raw = localStorage.getItem(ZOOM_KEY);
    const value = raw ? parseFloat(raw) : 1;
    return Number.isFinite(value) ? value : 1;
}

export function applyAppZoom(zoom: number): void {
    document.documentElement.style.zoom = String(zoom);
}

export function setAppZoom(zoom: number): number {
    const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
    localStorage.setItem(ZOOM_KEY, String(clamped));
    applyAppZoom(clamped);
    return clamped;
}

export function incrementAppZoom(current: number, direction: 1 | -1): number {
    return setAppZoom(current + direction * STEP);
}

export function resetAppZoom(): number {
    return setAppZoom(1);
}
