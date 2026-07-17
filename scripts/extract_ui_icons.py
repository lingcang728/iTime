"""Slice iTime icon sheets into transparent individual PNG assets."""
from __future__ import annotations

from pathlib import Path

import cv2
import numpy as np
from PIL import Image
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "src" / "assets" / "ui-icons"
SIZE = 256

SHEETS = [
    {
        "path": Path(r"C:\Users\15pro\Desktop\ChatGPT Image Jul 17, 2026, 10_03_10 AM (1).png"),
        "names": [
            "brand-itime",
            "page-home",
            "page-ai",
            "page-timeline",
            "page-input",
            "page-weekly",
            "page-goals",
            "page-settings",
            "section-ranking",
        ],
    },
    {
        "path": Path(r"C:\Users\15pro\Desktop\ChatGPT Image Jul 17, 2026, 10_03_11 AM (2).png"),
        "names": [
            "metric-computer",
            "metric-attention",
            "metric-ai-agent",
            "metric-voice",
            "metric-media",
            "metric-coverage",
            "metric-leverage",
            "metric-concurrency",
            "metric-parallel",
        ],
    },
    {
        "path": Path(r"C:\Users\15pro\Desktop\ChatGPT Image Jul 17, 2026, 10_03_11 AM (3).png"),
        "names": [
            "input-keystrokes",
            "input-left-click",
            "input-right-click",
            "input-mouse-move",
            "input-scroll",
            "input-heatmap",
            "input-top-keys",
            "input-shortcuts",
            "input-data-source",
        ],
    },
    {
        "path": Path(r"C:\Users\15pro\Desktop\ChatGPT Image Jul 17, 2026, 10_03_12 AM (4).png"),
        "names": [
            "goal-learning",
            "goal-development",
            "goal-ai",
            "goal-continuous",
            "goal-quiet",
            "goal-ai-notify",
            "weekly-achievements",
            "weekly-best-day",
            "goal-save",
        ],
    },
]

ROW_Y = [(160, 420), (520, 770), (850, 1110)]
COL_X = [(100, 380), (480, 770), (870, 1130)]


def matte_cell(rgba: np.ndarray) -> np.ndarray:
    """Remove border-connected sheet background; keep interior light surfaces."""
    rgb = rgba[..., :3].astype(np.float32)
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    lightness = (r + g + b) / 3.0
    chroma = np.maximum(np.maximum(r, g), b) - np.minimum(np.minimum(r, g), b)
    h, w = lightness.shape

    # Sheet backdrop is near-uniform light gray (~242-250).
    # Allow flood only through low-chroma light pixels.
    passable = (lightness >= 228) & (chroma <= 16)

    bg = np.zeros((h, w), dtype=bool)
    stack = [(0, x) for x in range(w)] + [(h - 1, x) for x in range(w)]
    stack += [(y, 0) for y in range(h)] + [(y, w - 1) for y in range(h)]
    while stack:
        y, x = stack.pop()
        if y < 0 or y >= h or x < 0 or x >= w or bg[y, x] or not passable[y, x]:
            continue
        bg[y, x] = True
        stack.extend(((y - 1, x), (y + 1, x), (y, x - 1), (y, x + 1)))

    # Foreground is everything not border-flooded.
    # Fill enclosed holes so book pages / paper interiors return as solid.
    fg = ~bg
    fg = ndimage.binary_fill_holes(fg)

    # True transparent centers (open rings connected to outside) stay open
    # because fill_holes only fills fully enclosed regions.
    # Re-open large enclosed holes that are pure sheet-gray (gear/ring centers
    # that were sealed by anti-aliasing). Leave small holes (text on pages).
    labels, n = ndimage.label(~fg)
    for i in range(1, n + 1):
        region = labels == i
        # skip the exterior (touches border)
        if region[0, :].any() or region[-1, :].any() or region[:, 0].any() or region[:, -1].any():
            continue
        area = int(region.sum())
        if area < 120:
            continue
        mean_l = float(lightness[region].mean())
        mean_c = float(chroma[region].mean())
        # pure backdrop hole (gear eye, logo ring center)
        if mean_l >= 232 and mean_c <= 12:
            fg[region] = False

    alpha = np.zeros((h, w), np.float32)
    alpha[fg] = 255.0

    # Feather 1px soft edge where lightness is high
    edge = ndimage.binary_dilation(bg, iterations=1) & fg
    soft = edge & (lightness > 210) & (chroma < 20)
    alpha[soft] = np.clip((245.0 - lightness[soft]) / 28.0, 0.15, 1.0) * 220.0

    out = rgba.copy()
    out[..., 3] = alpha.astype(np.uint8)
    out[out[..., 3] == 0, 0:3] = 0
    return out


def content_bbox(alpha: np.ndarray, thr: int = 10) -> tuple[int, int, int, int] | None:
    ys, xs = np.where(alpha > thr)
    if len(xs) == 0:
        return None
    return int(xs.min()), int(ys.min()), int(xs.max()) + 1, int(ys.max()) + 1


def extract_icon(sheet: np.ndarray, col: int, row: int) -> Image.Image:
    x0, x1 = COL_X[col]
    y0, y1 = ROW_Y[row]
    cell = sheet[y0:y1, x0:x1].copy()
    cleaned = matte_cell(cell)
    bbox = content_bbox(cleaned[..., 3])
    if bbox is None:
        return Image.fromarray(cleaned)
    cx0, cy0, cx1, cy1 = bbox
    pad = 8
    cx0 = max(0, cx0 - pad)
    cy0 = max(0, cy0 - pad)
    cx1 = min(cleaned.shape[1], cx1 + pad)
    cy1 = min(cleaned.shape[0], cy1 + pad)
    icon = cleaned[cy0:cy1, cx0:cx1]
    ih, iw = icon.shape[:2]
    side = max(ih, iw)
    canvas = np.zeros((side, side, 4), dtype=np.uint8)
    ox = (side - iw) // 2
    oy = (side - ih) // 2
    canvas[oy : oy + ih, ox : ox + iw] = icon
    return Image.fromarray(canvas, "RGBA").resize((SIZE, SIZE), Image.Resampling.LANCZOS)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for sheet_info in SHEETS:
        path = sheet_info["path"]
        if not path.exists():
            raise FileNotFoundError(path)
        sheet = np.array(Image.open(path).convert("RGBA"))
        for idx, name in enumerate(sheet_info["names"]):
            row, col = divmod(idx, 3)
            icon = extract_icon(sheet, col, row)
            out_path = OUT / f"{name}.png"
            icon.save(out_path, "PNG")
            arr = np.array(icon)
            opaque = int((arr[..., 3] > 10).sum())
            print(f"saved {name}.png opaque_px={opaque} corner_a={int(arr[0, 0, 3])}")
    print(f"done -> {OUT} ({len(list(OUT.glob('*.png')))} files)")


if __name__ == "__main__":
    main()
