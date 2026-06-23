const ZOOM_KEY = "sf-app-zoom";
const MIN_ZOOM = 0.6;
const MAX_ZOOM = 2;
const STEP = 0.1;

export function loadAppZoom(): number {
    const raw = localStorage.getItem(ZOOM_KEY);
    const value = raw ? parseFloat(raw) : 1;
    return Number.isFinite(value) ? value : 1;
}

// document.documentElement.style.zoom is non-standard and doesn't compose
// reliably with 100vh layouts / fixed-position elements / overflow scrolling
// across Chromium/WebView2 - zooming out left a transparent gap (window looked
// "not fullscreen") and zooming in clipped content instead of scrolling it.
// transform: scale() on a wrapper sized to 100/zoom% before scaling always
// paints exactly the real window's area, avoiding both issues.
const ZOOM_ROOT_ID = "zoom-root";

export function applyAppZoom(zoom: number): void {
    const root = document.getElementById(ZOOM_ROOT_ID);
    if (!root) return;
    root.style.transform = `scale(${zoom})`;
    root.style.width = `${100 / zoom}%`;
    root.style.height = `${100 / zoom}%`;
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
