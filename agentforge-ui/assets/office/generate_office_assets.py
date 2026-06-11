from __future__ import annotations

import json
import math
import struct
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parent
SPRITES = ROOT / "sprites"
TILE_SIZE = 40


def rgba(hex_value: str, alpha: int = 255) -> tuple[int, int, int, int]:
    value = hex_value.strip().lstrip("#")
    if len(value) != 6:
        raise ValueError(f"Expected RRGGBB color, got {hex_value!r}")
    return (
        int(value[0:2], 16),
        int(value[2:4], 16),
        int(value[4:6], 16),
        max(0, min(255, alpha)),
    )


TRANSPARENT = (0, 0, 0, 0)
SHADOW = (0, 0, 0, 78)

PAL = {
    "ink": rgba("#090912"),
    "line": rgba("#151525"),
    "line_soft": rgba("#24243a"),
    "skin": rgba("#f2c9a0"),
    "skin_shadow": rgba("#d79c75"),
    "hair": rgba("#171320"),
    "pants": rgba("#171827"),
    "shoe": rgba("#0d0d14"),
    "desk_top": rgba("#6b4630"),
    "desk_edge": rgba("#8b5f3f"),
    "desk_dark": rgba("#3c281f"),
    "desk_light": rgba("#b37a52"),
    "metal": rgba("#33384b"),
    "metal_light": rgba("#525a72"),
    "screen": rgba("#0b1220"),
    "screen_glow": rgba("#37a6ff"),
    "glass": rgba("#12344a", 210),
    "glass_hi": rgba("#88d8ff", 70),
    "plant_dark": rgba("#17391f"),
    "plant_mid": rgba("#23582d"),
    "plant_light": rgba("#31a052"),
    "pot": rgba("#563621"),
    "sofa_dark": rgba("#351728"),
    "sofa_mid": rgba("#5b2440"),
    "sofa_light": rgba("#823257"),
    "gold": rgba("#f2b84b"),
    "paper": rgba("#d7d8e8"),
    "white": rgba("#eef0ff"),
}


class Canvas:
    def __init__(self, width: int, height: int, bg: tuple[int, int, int, int] = TRANSPARENT):
        self.w = width
        self.h = height
        self.pixels = bytearray(width * height * 4)
        if bg[3] > 0:
            self.fill_rect(0, 0, width, height, bg)

    def _idx(self, x: int, y: int) -> int:
        return (y * self.w + x) * 4

    def set(self, x: int, y: int, color: tuple[int, int, int, int]):
        if x < 0 or y < 0 or x >= self.w or y >= self.h:
            return
        sr, sg, sb, sa = color
        idx = self._idx(x, y)
        if sa >= 255:
            self.pixels[idx : idx + 4] = bytes((sr, sg, sb, 255))
            return
        if sa <= 0:
            return
        dr, dg, db, da = self.pixels[idx : idx + 4]
        sa_f = sa / 255.0
        da_f = da / 255.0
        out_a = sa_f + da_f * (1.0 - sa_f)
        if out_a <= 0:
            return
        out_r = int((sr * sa_f + dr * da_f * (1.0 - sa_f)) / out_a)
        out_g = int((sg * sa_f + dg * da_f * (1.0 - sa_f)) / out_a)
        out_b = int((sb * sa_f + db * da_f * (1.0 - sa_f)) / out_a)
        self.pixels[idx : idx + 4] = bytes((out_r, out_g, out_b, int(out_a * 255)))

    def fill_rect(self, x: int, y: int, w: int, h: int, color: tuple[int, int, int, int]):
        x0 = max(0, x)
        y0 = max(0, y)
        x1 = min(self.w, x + w)
        y1 = min(self.h, y + h)
        for py in range(y0, y1):
            for px in range(x0, x1):
                self.set(px, py, color)

    def gradient_rect(
        self,
        x: int,
        y: int,
        w: int,
        h: int,
        top: tuple[int, int, int, int],
        bottom: tuple[int, int, int, int],
    ):
        if h <= 0:
            return
        for row in range(h):
            t = row / max(1, h - 1)
            col = tuple(int(top[i] * (1.0 - t) + bottom[i] * t) for i in range(4))
            self.fill_rect(x, y + row, w, 1, col)

    def outline_rect(self, x: int, y: int, w: int, h: int, color: tuple[int, int, int, int]):
        self.fill_rect(x, y, w, 1, color)
        self.fill_rect(x, y + h - 1, w, 1, color)
        self.fill_rect(x, y, 1, h, color)
        self.fill_rect(x + w - 1, y, 1, h, color)

    def ellipse(self, cx: int, cy: int, rx: int, ry: int, color: tuple[int, int, int, int]):
        if rx <= 0 or ry <= 0:
            return
        for y in range(cy - ry, cy + ry + 1):
            for x in range(cx - rx, cx + rx + 1):
                dx = (x - cx) / rx
                dy = (y - cy) / ry
                if dx * dx + dy * dy <= 1.0:
                    self.set(x, y, color)

    def polygon(self, points: list[tuple[int, int]], color: tuple[int, int, int, int]):
        if len(points) < 3:
            return
        min_y = max(0, min(y for _, y in points))
        max_y = min(self.h - 1, max(y for _, y in points))
        count = len(points)
        for y in range(min_y, max_y + 1):
            nodes: list[float] = []
            j = count - 1
            for i in range(count):
                xi, yi = points[i]
                xj, yj = points[j]
                if (yi < y <= yj) or (yj < y <= yi):
                    x = xi + (y - yi) / (yj - yi) * (xj - xi)
                    nodes.append(x)
                j = i
            nodes.sort()
            for i in range(0, len(nodes), 2):
                if i + 1 >= len(nodes):
                    break
                x0 = max(0, int(math.ceil(nodes[i])))
                x1 = min(self.w - 1, int(math.floor(nodes[i + 1])))
                for x in range(x0, x1 + 1):
                    self.set(x, y, color)

    def line(
        self,
        x0: int,
        y0: int,
        x1: int,
        y1: int,
        color: tuple[int, int, int, int],
        width: int = 1,
    ):
        dx = abs(x1 - x0)
        dy = -abs(y1 - y0)
        sx = 1 if x0 < x1 else -1
        sy = 1 if y0 < y1 else -1
        err = dx + dy
        while True:
            r = max(0, width // 2)
            self.fill_rect(x0 - r, y0 - r, max(1, width), max(1, width), color)
            if x0 == x1 and y0 == y1:
                break
            e2 = 2 * err
            if e2 >= dy:
                err += dy
                x0 += sx
            if e2 <= dx:
                err += dx
                y0 += sy

    def paste(self, other: "Canvas", x: int, y: int):
        for py in range(other.h):
            for px in range(other.w):
                idx = (py * other.w + px) * 4
                self.set(x + px, y + py, tuple(other.pixels[idx : idx + 4]))


def save_png(canvas: Canvas, path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = bytearray()
    stride = canvas.w * 4
    for y in range(canvas.h):
        raw.append(0)
        raw.extend(canvas.pixels[y * stride : (y + 1) * stride])

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    png = b"\x89PNG\r\n\x1a\n"
    png += chunk(b"IHDR", struct.pack(">IIBBBBB", canvas.w, canvas.h, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9))
    png += chunk(b"IEND", b"")
    path.write_bytes(png)


def add_shadow(c: Canvas, cx: int, cy: int, rx: int, ry: int, alpha: int = 78):
    c.ellipse(cx, cy, rx, ry, (0, 0, 0, alpha))


def draw_monitor(c: Canvas, x: int, y: int, glow: int = 85):
    c.fill_rect(x, y + 3, 28, 17, PAL["line"])
    c.fill_rect(x + 3, y + 6, 22, 10, PAL["screen"])
    c.fill_rect(x + 6, y + 18, 16, 3, PAL["metal"])
    c.fill_rect(x + 12, y + 21, 5, 4, PAL["metal_light"])
    c.fill_rect(x + 4, y + 7, 19, 1, rgba("#56c7ff", glow))
    c.fill_rect(x + 5, y + 10, 9, 1, rgba("#7c5cfc", glow))
    c.fill_rect(x + 17, y + 13, 5, 1, rgba("#4ade80", glow))


def draw_keyboard(c: Canvas, x: int, y: int):
    c.fill_rect(x, y, 27, 8, rgba("#151828"))
    c.outline_rect(x, y, 27, 8, rgba("#2b3146"))
    for i in range(4):
        c.fill_rect(x + 4 + i * 5, y + 3, 3, 2, rgba("#5f6782"))


def draw_employee_frame(c: Canvas, accent: tuple[int, int, int, int], state: str, frame: int):
    frame_count = 8 if state == "walk" else 6
    phase = (frame / frame_count) * math.tau
    bob = int(round(math.sin(phase) * (2 if state in {"idle", "talk", "wave"} else 3)))
    if state == "typing":
        bob = int(round(math.sin(phase * 2.0) * 1))
    cx = 24
    base_y = 55 + max(0, bob // 2)
    body_y = 30 + bob

    add_shadow(c, cx, 56, 15, 4, 70)

    leg_shift = int(round(math.sin(phase) * 4)) if state == "walk" else 0
    c.fill_rect(cx - 8 - leg_shift // 2, body_y + 17, 6, 15, PAL["pants"])
    c.fill_rect(cx + 2 + leg_shift // 2, body_y + 17, 6, 15, PAL["pants"])
    c.fill_rect(cx - 9 - leg_shift // 2, base_y - 3, 9, 4, PAL["shoe"])
    c.fill_rect(cx + 1 + leg_shift // 2, base_y - 3, 9, 4, PAL["shoe"])

    c.fill_rect(cx - 10, body_y, 20, 21, accent)
    c.fill_rect(cx - 8, body_y + 2, 16, 3, (255, 255, 255, 34))
    c.fill_rect(cx - 1, body_y, 2, 21, (0, 0, 0, 36))
    c.fill_rect(cx - 4, body_y - 5, 8, 6, PAL["skin"])

    if state == "typing":
        c.fill_rect(cx - 17, body_y + 4, 7, 5, PAL["skin"])
        c.fill_rect(cx + 10, body_y + 4, 7, 5, PAL["skin"])
        c.fill_rect(cx - 14, body_y + 9, 28, 8, rgba("#161b2a"))
        c.fill_rect(cx - 12, body_y + 11, 24, 2, rgba("#5bd1ff", 95 + frame * 18))
    elif state == "wave":
        arm_up = 8 + int(round(math.sin(phase) * 3))
        c.fill_rect(cx - 15, body_y + 4, 6, 13, accent)
        c.fill_rect(cx + 9, body_y - arm_up, 6, 18, accent)
        c.fill_rect(cx + 9, body_y - arm_up - 4, 6, 5, PAL["skin"])
    else:
        arm_shift = int(round(math.sin(phase) * 3)) if state == "walk" else 0
        c.fill_rect(cx - 15, body_y + 3 + arm_shift, 6, 15, accent)
        c.fill_rect(cx + 9, body_y + 3 - arm_shift, 6, 15, accent)
        c.fill_rect(cx - 15, body_y + 15 + arm_shift, 6, 5, PAL["skin"])
        c.fill_rect(cx + 9, body_y + 15 - arm_shift, 6, 5, PAL["skin"])

    head_y = body_y - 18
    c.fill_rect(cx - 10, head_y, 20, 16, PAL["skin"])
    c.fill_rect(cx - 11, head_y - 4, 22, 8, PAL["hair"])
    c.fill_rect(cx - 11, head_y + 3, 4, 7, PAL["hair"])
    c.fill_rect(cx + 7, head_y + 3, 4, 7, PAL["hair"])
    c.fill_rect(cx - 6, head_y + 7, 4, 3, PAL["ink"])
    c.fill_rect(cx + 2, head_y + 7, 4, 3, PAL["ink"])
    c.fill_rect(cx - 5, head_y + 8, 2, 1, PAL["white"])
    c.fill_rect(cx + 3, head_y + 8, 2, 1, PAL["white"])
    mouth_color = rgba("#b45a5f")
    if state == "talk" and frame % 2 == 0:
        c.fill_rect(cx - 3, head_y + 12, 6, 2, mouth_color)
    else:
        c.fill_rect(cx - 2, head_y + 12, 4, 1, mouth_color)


def make_employee_sheet(accent: tuple[int, int, int, int], state: str) -> tuple[Canvas, int, int, int]:
    frames = 8 if state == "walk" else 6
    fw, fh = 48, 64
    sheet = Canvas(fw * frames, fh)
    for i in range(frames):
        frame = Canvas(fw, fh)
        draw_employee_frame(frame, accent, state, i)
        sheet.paste(frame, i * fw, 0)
    return sheet, fw, fh, frames


def make_sheet(frame_w: int, frame_h: int, frames: int, draw_fn) -> Canvas:
    sheet = Canvas(frame_w * frames, frame_h)
    for i in range(frames):
        frame = Canvas(frame_w, frame_h)
        draw_fn(frame, i)
        sheet.paste(frame, i * frame_w, 0)
    return sheet


def desk_single() -> Canvas:
    c = Canvas(96, 64)
    add_shadow(c, 48, 55, 41, 7, 80)
    c.polygon([(9, 19), (86, 14), (91, 42), (5, 49)], PAL["desk_top"])
    c.polygon([(5, 49), (91, 42), (87, 50), (9, 58)], PAL["desk_dark"])
    c.line(10, 22, 85, 17, PAL["desk_light"], 2)
    c.fill_rect(18, 48, 8, 12, PAL["desk_dark"])
    c.fill_rect(72, 44, 8, 12, PAL["desk_dark"])
    draw_monitor(c, 35, 18, 120)
    draw_keyboard(c, 34, 39)
    c.fill_rect(66, 27, 12, 15, rgba("#202536"))
    c.fill_rect(68, 29, 8, 9, rgba("#111521"))
    return c


def desk_dual() -> Canvas:
    c = Canvas(128, 80)
    add_shadow(c, 64, 69, 56, 8, 82)
    c.polygon([(10, 22), (118, 17), (122, 51), (7, 59)], PAL["desk_top"])
    c.polygon([(7, 59), (122, 51), (116, 62), (13, 72)], PAL["desk_dark"])
    c.line(12, 25, 116, 20, PAL["desk_light"], 2)
    for x in (24, 95):
        c.fill_rect(x, 57, 8, 15, PAL["desk_dark"])
    draw_monitor(c, 29, 22, 115)
    draw_monitor(c, 71, 20, 115)
    draw_keyboard(c, 30, 44)
    draw_keyboard(c, 73, 43)
    c.fill_rect(61, 25, 4, 28, rgba("#2b1e17"))
    return c


def standing_desk() -> Canvas:
    c = Canvas(96, 72)
    add_shadow(c, 48, 64, 39, 6, 72)
    c.polygon([(12, 18), (84, 15), (88, 34), (8, 39)], rgba("#74533a"))
    c.polygon([(8, 39), (88, 34), (84, 40), (12, 46)], PAL["desk_dark"])
    c.fill_rect(24, 43, 8, 21, PAL["metal"])
    c.fill_rect(64, 41, 8, 23, PAL["metal"])
    draw_monitor(c, 35, 17, 130)
    draw_keyboard(c, 34, 36)
    return c


def office_chair(accent: tuple[int, int, int, int]) -> Canvas:
    c = Canvas(40, 48)
    add_shadow(c, 20, 42, 15, 4, 70)
    c.fill_rect(13, 13, 14, 19, PAL["metal"])
    c.fill_rect(10, 10, 20, 18, accent)
    c.fill_rect(12, 12, 16, 4, (255, 255, 255, 38))
    c.fill_rect(8, 28, 24, 11, accent)
    c.fill_rect(11, 31, 18, 3, (255, 255, 255, 28))
    c.fill_rect(18, 38, 4, 7, PAL["metal_light"])
    c.line(20, 43, 11, 47, PAL["metal"], 2)
    c.line(20, 43, 29, 47, PAL["metal"], 2)
    c.line(20, 43, 20, 47, PAL["metal"], 2)
    return c


def guest_chair() -> Canvas:
    c = Canvas(44, 46)
    add_shadow(c, 22, 40, 17, 4, 64)
    c.fill_rect(10, 12, 24, 15, rgba("#3b4256"))
    c.fill_rect(12, 14, 20, 4, rgba("#6f7890"))
    c.fill_rect(8, 25, 28, 12, rgba("#4a5266"))
    c.fill_rect(11, 28, 22, 3, rgba("#818aa0"))
    c.fill_rect(11, 35, 4, 9, PAL["metal"])
    c.fill_rect(29, 35, 4, 9, PAL["metal"])
    return c


def sofa_3seat() -> Canvas:
    c = Canvas(128, 64)
    add_shadow(c, 64, 57, 54, 6, 82)
    c.fill_rect(9, 18, 110, 34, PAL["sofa_dark"])
    c.fill_rect(14, 14, 100, 31, PAL["sofa_mid"])
    c.fill_rect(15, 17, 31, 22, rgba("#6c2b4d"))
    c.fill_rect(49, 17, 30, 22, rgba("#702c50"))
    c.fill_rect(82, 17, 31, 22, rgba("#6c2b4d"))
    c.fill_rect(9, 17, 9, 35, PAL["sofa_light"])
    c.fill_rect(110, 17, 9, 35, PAL["sofa_light"])
    c.fill_rect(16, 15, 96, 3, (255, 255, 255, 30))
    return c


def sofa_side() -> Canvas:
    c = Canvas(64, 120)
    add_shadow(c, 33, 109, 24, 7, 80)
    c.fill_rect(16, 12, 34, 96, PAL["sofa_dark"])
    c.fill_rect(12, 18, 31, 85, PAL["sofa_mid"])
    c.fill_rect(15, 22, 22, 36, rgba("#6c2b4d"))
    c.fill_rect(15, 62, 22, 36, rgba("#743056"))
    c.fill_rect(12, 18, 7, 85, PAL["sofa_light"])
    c.fill_rect(39, 18, 8, 85, PAL["sofa_light"])
    return c


def coffee_table() -> Canvas:
    c = Canvas(96, 56)
    add_shadow(c, 48, 49, 38, 5, 72)
    c.polygon([(12, 17), (82, 13), (88, 36), (8, 42)], rgba("#14283b"))
    c.polygon([(17, 20), (77, 17), (82, 33), (14, 38)], PAL["glass"])
    c.line(20, 21, 75, 18, PAL["glass_hi"], 2)
    c.fill_rect(20, 39, 5, 10, rgba("#263246"))
    c.fill_rect(70, 36, 5, 10, rgba("#263246"))
    c.fill_rect(36, 26, 9, 7, PAL["paper"])
    c.ellipse(66, 25, 5, 4, PAL["gold"])
    return c


def meeting_table() -> Canvas:
    c = Canvas(144, 72)
    add_shadow(c, 72, 64, 62, 6, 78)
    c.polygon([(16, 20), (128, 15), (136, 45), (8, 54)], rgba("#65422e"))
    c.polygon([(8, 54), (136, 45), (129, 55), (15, 66)], rgba("#32221b"))
    c.line(20, 23, 124, 18, rgba("#a8744f"), 2)
    for x in (35, 69, 103):
        c.fill_rect(x, 31, 13, 10, rgba("#202536"))
        c.fill_rect(x + 2, 33, 9, 2, rgba("#56c7ff", 80))
    c.fill_rect(65, 50, 9, 14, rgba("#33231a"))
    c.fill_rect(99, 47, 9, 13, rgba("#33231a"))
    return c


def reception_counter() -> Canvas:
    c = Canvas(160, 72)
    add_shadow(c, 80, 65, 66, 6, 82)
    c.polygon([(11, 21), (146, 15), (153, 45), (5, 55)], rgba("#4a3442"))
    c.polygon([(5, 55), (153, 45), (143, 61), (15, 70)], rgba("#221826"))
    c.fill_rect(31, 30, 51, 15, rgba("#6c3f58"))
    c.fill_rect(87, 28, 41, 14, rgba("#6c3f58"))
    c.line(18, 25, 142, 19, rgba("#c48bd6"), 2)
    draw_monitor(c, 65, 18, 115)
    c.fill_rect(23, 41, 18, 7, rgba("#f0d48a"))
    c.fill_rect(118, 36, 18, 8, rgba("#96e6ff", 90))
    return c


def bookshelf() -> Canvas:
    c = Canvas(120, 52)
    add_shadow(c, 60, 47, 50, 4, 56)
    c.fill_rect(6, 9, 108, 36, rgba("#2d1c13"))
    c.fill_rect(10, 12, 100, 28, rgba("#3b2818"))
    c.fill_rect(10, 24, 100, 3, rgba("#23170e"))
    c.fill_rect(58, 12, 3, 28, rgba("#23170e"))
    colors = [rgba("#f472b6"), rgba("#60a5fa"), rgba("#4ade80"), rgba("#fbbf24"), rgba("#a78bfa"), rgba("#fb923c")]
    x = 14
    for i in range(16):
        bw = 5 + (i % 4)
        h = 22 if i % 3 else 16
        c.fill_rect(x, 17 + (22 - h), bw, h, colors[i % len(colors)])
        c.fill_rect(x + 1, 18 + (22 - h), 1, h - 2, (255, 255, 255, 45))
        x += bw + 2
        if x > 104:
            break
    return c


def waiting_bench() -> Canvas:
    c = Canvas(144, 64)
    add_shadow(c, 72, 57, 60, 5, 76)
    for x in (16, 51, 86):
        c.fill_rect(x, 17, 33, 20, rgba("#2f3850"))
        c.fill_rect(x + 3, 20, 27, 4, rgba("#71809d"))
        c.fill_rect(x, 37, 33, 10, rgba("#465372"))
    c.fill_rect(12, 47, 112, 5, PAL["metal"])
    for x in (18, 68, 118):
        c.fill_rect(x, 51, 5, 10, PAL["metal_light"])
    return c


def queue_rope() -> Canvas:
    c = Canvas(96, 48)
    add_shadow(c, 48, 43, 37, 3, 56)
    for x in (18, 78):
        c.fill_rect(x - 3, 18, 6, 22, PAL["metal"])
        c.ellipse(x, 17, 5, 4, PAL["gold"])
        c.fill_rect(x - 7, 39, 14, 3, PAL["metal_light"])
    for i in range(18, 79):
        y = 23 + int(math.sin((i - 18) / 60 * math.pi) * 9)
        c.fill_rect(i, y, 2, 4, rgba("#b34263"))
    return c


def lounge_rug() -> Canvas:
    c = Canvas(128, 80)
    c.ellipse(64, 41, 59, 30, rgba("#183653", 210))
    c.ellipse(64, 41, 50, 23, rgba("#23466a", 180))
    c.ellipse(64, 41, 29, 13, rgba("#7c5cfc", 80))
    c.line(24, 39, 104, 35, rgba("#8bd6ff", 54), 2)
    return c


def floor_arrow() -> Canvas:
    c = Canvas(80, 40)
    c.polygon([(8, 16), (50, 16), (50, 8), (72, 20), (50, 32), (50, 24), (8, 24)], rgba("#60a5fa", 150))
    c.polygon([(11, 18), (52, 18), (52, 13), (66, 20), (52, 27), (52, 22), (11, 22)], rgba("#e2e0ff", 90))
    return c


def plant_tall(offset: int = 0) -> Canvas:
    c = Canvas(48, 72)
    add_shadow(c, 24, 66, 14, 4, 54)
    c.fill_rect(17, 53, 14, 12, PAL["pot"])
    c.fill_rect(14, 50, 20, 5, rgba("#6a4327"))
    c.line(24, 51, 24 + offset, 20, PAL["plant_dark"], 3)
    leaves = [
        (22 + offset, 31, 12, 6, -6, PAL["plant_mid"]),
        (29 + offset, 27, 11, 6, 6, PAL["plant_light"]),
        (18 + offset, 22, 11, 6, -10, PAL["plant_light"]),
        (27 + offset, 18, 10, 6, 8, PAL["plant_mid"]),
        (23 + offset, 13, 8, 5, 0, PAL["plant_light"]),
    ]
    for cx, cy, rx, ry, _rot, col in leaves:
        c.ellipse(cx, cy, rx, ry, col)
    c.fill_rect(20, 55, 8, 3, (255, 255, 255, 30))
    return c


def plant_small() -> Canvas:
    c = Canvas(40, 48)
    add_shadow(c, 20, 43, 12, 3, 50)
    c.fill_rect(14, 34, 12, 9, PAL["pot"])
    c.fill_rect(12, 31, 16, 4, rgba("#6a4327"))
    c.ellipse(16, 27, 9, 6, PAL["plant_mid"])
    c.ellipse(24, 26, 9, 6, PAL["plant_light"])
    c.ellipse(20, 19, 10, 8, PAL["plant_light"])
    return c


def floor_lamp() -> Canvas:
    c = Canvas(48, 80)
    add_shadow(c, 24, 73, 14, 4, 52)
    c.fill_rect(22, 31, 4, 39, PAL["metal"])
    c.fill_rect(15, 69, 18, 4, PAL["metal_light"])
    c.polygon([(13, 15), (35, 15), (31, 33), (17, 33)], rgba("#f2b84b", 210))
    c.polygon([(16, 17), (32, 17), (29, 29), (19, 29)], rgba("#ffe8a3", 95))
    c.ellipse(24, 28, 18, 11, rgba("#f2b84b", 35))
    return c


def wall_frame() -> Canvas:
    c = Canvas(56, 40)
    add_shadow(c, 29, 35, 21, 3, 34)
    c.fill_rect(6, 6, 44, 28, rgba("#4d2f1e"))
    c.fill_rect(10, 10, 36, 20, rgba("#151a32"))
    c.polygon([(12, 28), (25, 17), (34, 25), (45, 13), (45, 30), (12, 30)], rgba("#7c5cfc", 130))
    c.ellipse(19, 16, 4, 4, rgba("#f2b84b"))
    c.outline_rect(6, 6, 44, 28, rgba("#b98255"))
    return c


def wall_clock() -> Canvas:
    c = Canvas(40, 40)
    c.ellipse(20, 20, 15, 15, rgba("#dfe7ff"))
    c.ellipse(20, 20, 12, 12, rgba("#172034"))
    c.line(20, 20, 20, 11, rgba("#dfe7ff"), 2)
    c.line(20, 20, 28, 22, rgba("#7c5cfc"), 2)
    c.ellipse(20, 20, 2, 2, rgba("#f2b84b"))
    return c


def whiteboard() -> Canvas:
    c = Canvas(96, 56)
    add_shadow(c, 48, 50, 38, 3, 36)
    c.fill_rect(6, 8, 84, 38, rgba("#dfe5f2"))
    c.outline_rect(6, 8, 84, 38, rgba("#586177"))
    c.line(18, 21, 35, 17, rgba("#60a5fa"), 2)
    c.line(18, 30, 54, 28, rgba("#4ade80"), 2)
    c.line(58, 18, 79, 34, rgba("#f472b6"), 2)
    c.fill_rect(62, 43, 17, 3, rgba("#33384b"))
    return c


def water_cooler() -> Canvas:
    c = Canvas(44, 72)
    add_shadow(c, 22, 66, 14, 4, 50)
    c.ellipse(22, 16, 11, 12, rgba("#8bd6ff", 150))
    c.fill_rect(13, 22, 18, 29, rgba("#dce4f3"))
    c.fill_rect(15, 25, 14, 7, rgba("#93c5fd", 130))
    c.fill_rect(17, 38, 10, 4, rgba("#384152"))
    c.fill_rect(14, 51, 16, 13, rgba("#4b5568"))
    return c


def glass_divider() -> Canvas:
    c = Canvas(120, 52)
    add_shadow(c, 60, 48, 51, 3, 40)
    for x in (9, 39, 69, 99):
        c.fill_rect(x, 7, 3, 37, rgba("#586177"))
    c.fill_rect(7, 9, 106, 30, rgba("#77d7ff", 42))
    c.line(14, 13, 103, 35, rgba("#d8fbff", 65), 2)
    c.fill_rect(7, 39, 106, 5, rgba("#33384b"))
    return c


def neon_status_panel() -> Canvas:
    c = Canvas(64, 48)
    add_shadow(c, 32, 43, 24, 3, 34)
    c.fill_rect(8, 8, 48, 30, rgba("#10131f"))
    c.outline_rect(8, 8, 48, 30, rgba("#7c5cfc"))
    for i, col in enumerate([rgba("#4ade80"), rgba("#60a5fa"), rgba("#fbbf24")]):
        c.ellipse(18 + i * 13, 22, 4, 4, col)
        c.ellipse(18 + i * 13, 22, 8, 8, (col[0], col[1], col[2], 35))
    c.fill_rect(15, 31, 34, 2, rgba("#e2e0ff", 85))
    return c


def floor_tile_dev() -> Canvas:
    c = Canvas(40, 40, rgba("#1e191b"))
    c.fill_rect(0, 0, 40, 1, rgba("#3b2c32"))
    c.fill_rect(0, 0, 1, 40, rgba("#3b2c32"))
    c.fill_rect(18, 18, 2, 2, rgba("#7c5cfc", 42))
    c.fill_rect(30, 8, 1, 1, rgba("#f472b6", 45))
    return c


def floor_tile_lounge() -> Canvas:
    c = Canvas(40, 40, rgba("#0c1a2e"))
    c.fill_rect(0, 0, 40, 1, rgba("#173553"))
    c.fill_rect(0, 0, 1, 40, rgba("#173553"))
    c.fill_rect(8, 29, 12, 1, rgba("#60a5fa", 32))
    return c


def wall_segment() -> Canvas:
    c = Canvas(40, 40)
    c.fill_rect(0, 0, 40, 22, rgba("#1a1a2e"))
    c.fill_rect(0, 22, 40, 18, rgba("#13131f"))
    c.fill_rect(0, 20, 40, 3, rgba("#252540"))
    c.fill_rect(0, 0, 1, 40, rgba("#2a2a44"))
    return c


def glass_wall_tile() -> Canvas:
    c = Canvas(40, 40)
    c.fill_rect(2, 3, 36, 30, rgba("#66d9ff", 42))
    c.fill_rect(0, 0, 40, 3, rgba("#33384b"))
    c.fill_rect(0, 33, 40, 4, rgba("#33384b"))
    c.line(8, 6, 32, 28, rgba("#d8fbff", 70), 2)
    return c


STATIC_BUILDERS = {
    "desk_single": ("sprites/furniture/desk_single.png", desk_single, ["desk", "workstation", "furniture"], {"x": 48, "y": 58}),
    "desk_dual": ("sprites/furniture/desk_dual.png", desk_dual, ["desk", "dual", "workstation"], {"x": 64, "y": 72}),
    "standing_desk": ("sprites/furniture/standing_desk.png", standing_desk, ["desk", "standing", "workstation"], {"x": 48, "y": 64}),
    "office_chair_blue": (
        "sprites/furniture/office_chair_blue.png",
        lambda: office_chair(rgba("#60a5fa")),
        ["chair", "office", "blue"],
        {"x": 20, "y": 44},
    ),
    "office_chair_violet": (
        "sprites/furniture/office_chair_violet.png",
        lambda: office_chair(rgba("#7c5cfc")),
        ["chair", "office", "violet"],
        {"x": 20, "y": 44},
    ),
    "guest_chair": ("sprites/furniture/guest_chair.png", guest_chair, ["chair", "waiting-room"], {"x": 22, "y": 44}),
    "sofa_3seat": ("sprites/furniture/sofa_3seat.png", sofa_3seat, ["sofa", "lounge"], {"x": 64, "y": 57}),
    "sofa_side": ("sprites/furniture/sofa_side.png", sofa_side, ["sofa", "lounge", "side"], {"x": 33, "y": 109}),
    "coffee_table": ("sprites/furniture/coffee_table.png", coffee_table, ["table", "lounge"], {"x": 48, "y": 49}),
    "meeting_table": ("sprites/furniture/meeting_table.png", meeting_table, ["table", "meeting"], {"x": 72, "y": 66}),
    "reception_counter": (
        "sprites/waiting_room/reception_counter.png",
        reception_counter,
        ["reception", "counter", "waiting-room"],
        {"x": 80, "y": 70},
    ),
    "bookshelf": ("sprites/furniture/bookshelf.png", bookshelf, ["bookshelf", "storage", "decor"], {"x": 60, "y": 47}),
    "waiting_bench": (
        "sprites/waiting_room/waiting_bench.png",
        waiting_bench,
        ["bench", "waiting-room", "chair"],
        {"x": 72, "y": 61},
    ),
    "queue_rope": ("sprites/waiting_room/queue_rope.png", queue_rope, ["queue", "waiting-room"], {"x": 48, "y": 43}),
    "lounge_rug": ("sprites/waiting_room/lounge_rug.png", lounge_rug, ["rug", "waiting-room"], {"x": 64, "y": 70}),
    "floor_wayfinding_arrow": (
        "sprites/waiting_room/floor_wayfinding_arrow.png",
        floor_arrow,
        ["wayfinding", "waiting-room", "floor"],
        {"x": 40, "y": 32},
    ),
    "plant_tall": ("sprites/decor/plant_tall.png", plant_tall, ["plant", "decor"], {"x": 24, "y": 66}),
    "plant_small": ("sprites/decor/plant_small.png", plant_small, ["plant", "decor", "desk"], {"x": 20, "y": 43}),
    "floor_lamp": ("sprites/decor/floor_lamp.png", floor_lamp, ["lamp", "decor"], {"x": 24, "y": 73}),
    "wall_frame": ("sprites/decor/wall_frame.png", wall_frame, ["frame", "wall", "decor"], {"x": 28, "y": 34}),
    "wall_clock": ("sprites/decor/wall_clock.png", wall_clock, ["clock", "wall", "decor"], {"x": 20, "y": 36}),
    "whiteboard": ("sprites/decor/whiteboard.png", whiteboard, ["whiteboard", "wall", "decor"], {"x": 48, "y": 48}),
    "water_cooler": ("sprites/decor/water_cooler.png", water_cooler, ["water", "decor"], {"x": 22, "y": 66}),
    "glass_divider": ("sprites/decor/glass_divider.png", glass_divider, ["divider", "glass"], {"x": 60, "y": 48}),
    "neon_status_panel": ("sprites/decor/neon_status_panel.png", neon_status_panel, ["panel", "status", "decor"], {"x": 32, "y": 43}),
    "floor_dev_tile": ("sprites/tiles/floor_dev_tile.png", floor_tile_dev, ["tile", "floor", "dev-room"], {"x": 20, "y": 20}),
    "floor_lounge_tile": ("sprites/tiles/floor_lounge_tile.png", floor_tile_lounge, ["tile", "floor", "lounge"], {"x": 20, "y": 20}),
    "wall_segment": ("sprites/tiles/wall_segment.png", wall_segment, ["tile", "wall"], {"x": 20, "y": 39}),
    "glass_wall_tile": ("sprites/tiles/glass_wall_tile.png", glass_wall_tile, ["tile", "wall", "glass"], {"x": 20, "y": 37}),
}


def draw_monitor_glow(c: Canvas, frame: int):
    level = 80 + int((math.sin(frame / 6 * math.tau) + 1) * 70)
    add_shadow(c, 24, 34, 17, 3, 45)
    draw_monitor(c, 10, 7, level)
    c.ellipse(24, 17, 19, 11, rgba("#37a6ff", 20 + level // 6))


def draw_coffee_steam(c: Canvas, frame: int):
    add_shadow(c, 16, 42, 10, 3, 42)
    c.ellipse(16, 36, 9, 5, rgba("#2c3345"))
    c.ellipse(16, 34, 8, 4, rgba("#f2b84b"))
    c.ellipse(16, 34, 5, 2, rgba("#4a2d19"))
    for i in range(3):
        x = 10 + i * 5 + int(math.sin((frame + i) * 0.8) * 2)
        y0 = 26 - i * 2
        c.line(x, y0, x + 4, y0 - 9, rgba("#d8fbff", 70), 1)


def draw_plant_sway(c: Canvas, frame: int):
    offset = int(round(math.sin(frame / 8 * math.tau) * 3))
    c.paste(plant_tall(offset), 0, 0)


def draw_status_light(c: Canvas, frame: int):
    pulse = (math.sin(frame / 8 * math.tau) + 1.0) / 2.0
    col = rgba("#4ade80")
    c.ellipse(12, 12, 11, 11, (col[0], col[1], col[2], int(24 + pulse * 75)))
    c.ellipse(12, 12, 6, 6, (col[0], col[1], col[2], int(165 + pulse * 80)))
    c.ellipse(12, 12, 2, 2, rgba("#eef0ff", int(150 + pulse * 70)))


def draw_typing_dots(c: Canvas, frame: int):
    c.fill_rect(4, 5, 56, 15, rgba("#080812", 225))
    c.outline_rect(4, 5, 56, 15, rgba("#7c5cfc", 100))
    for i in range(3):
        active = (frame + i) % 6 < 3
        alpha = 210 if active else 75
        c.ellipse(22 + i * 10, 12, 3, 3, rgba("#a78bfa", alpha))


def draw_door_scan(c: Canvas, frame: int):
    c.fill_rect(9, 6, 46, 35, rgba("#111827"))
    c.outline_rect(9, 6, 46, 35, rgba("#3f4964"))
    c.fill_rect(15, 12, 34, 22, rgba("#66d9ff", 32))
    y = 13 + int((frame / 8) * 19)
    c.fill_rect(15, y, 34, 2, rgba("#60a5fa", 180))
    c.fill_rect(25, 37, 14, 5, rgba("#33384b"))


def draw_waiting_tv(c: Canvas, frame: int):
    c.fill_rect(9, 8, 62, 35, rgba("#151525"))
    c.fill_rect(13, 12, 54, 25, rgba("#0b1220"))
    c.outline_rect(9, 8, 62, 35, rgba("#4f5b78"))
    for x in range(16, 63, 5):
        y = 24 + int(math.sin((x + frame * 6) * 0.25) * 7)
        c.fill_rect(x, y, 3, 2, rgba("#60a5fa", 145))
    c.fill_rect(34, 43, 12, 4, rgba("#33384b"))


ANIMATION_BUILDERS = {
    "monitor_glow": ("sprites/animations/monitor_glow.png", 48, 40, 6, 8, draw_monitor_glow, ["monitor", "glow"]),
    "coffee_steam": ("sprites/animations/coffee_steam.png", 32, 48, 8, 10, draw_coffee_steam, ["coffee", "steam"]),
    "plant_sway": ("sprites/animations/plant_sway.png", 48, 72, 8, 8, draw_plant_sway, ["plant", "sway"]),
    "status_light_pulse": ("sprites/animations/status_light_pulse.png", 24, 24, 8, 12, draw_status_light, ["status", "pulse"]),
    "typing_dots": ("sprites/animations/typing_dots.png", 64, 24, 6, 8, draw_typing_dots, ["typing", "chat"]),
    "door_scan": ("sprites/animations/door_scan.png", 64, 48, 8, 10, draw_door_scan, ["door", "scan"]),
    "waiting_room_tv": ("sprites/animations/waiting_room_tv.png", 80, 56, 8, 8, draw_waiting_tv, ["waiting-room", "screen"]),
}


EMPLOYEES = {
    "engineer": rgba("#60a5fa"),
    "designer": rgba("#f472b6"),
    "manager": rgba("#fbbf24"),
    "ops": rgba("#4ade80"),
}


def build_static_assets() -> list[dict]:
    entries = []
    for asset_id, (rel, builder, tags, anchor) in STATIC_BUILDERS.items():
        canvas = builder()
        save_png(canvas, ROOT / rel)
        entries.append(
            {
                "id": asset_id,
                "file": rel.replace("\\", "/"),
                "type": "image",
                "width": canvas.w,
                "height": canvas.h,
                "anchor": anchor,
                "tags": tags,
                "_canvas": canvas,
            }
        )
    return entries


def build_character_assets() -> list[dict]:
    entries = []
    for role, accent in EMPLOYEES.items():
        for state in ("idle", "walk", "typing", "talk", "wave"):
            sheet, fw, fh, frames = make_employee_sheet(accent, state)
            rel = f"sprites/characters/employee_{role}_{state}.png"
            save_png(sheet, ROOT / rel)
            entries.append(
                {
                    "id": f"employee_{role}_{state}",
                    "file": rel,
                    "type": "sprite_sheet",
                    "frame_width": fw,
                    "frame_height": fh,
                    "frames": frames,
                    "fps": 10 if state == "walk" else 8,
                    "anchor": {"x": 24, "y": 55},
                    "tags": ["employee", role, state],
                }
            )
    return entries


def build_animation_assets() -> list[dict]:
    entries = []
    for asset_id, (rel, fw, fh, frames, fps, draw_fn, tags) in ANIMATION_BUILDERS.items():
        sheet = make_sheet(fw, fh, frames, draw_fn)
        save_png(sheet, ROOT / rel)
        entries.append(
            {
                "id": asset_id,
                "file": rel.replace("\\", "/"),
                "type": "sprite_sheet",
                "frame_width": fw,
                "frame_height": fh,
                "frames": frames,
                "fps": fps,
                "anchor": {"x": fw // 2, "y": fh - 2},
                "tags": tags,
            }
        )
    return entries


def build_static_atlas(static_entries: list[dict]) -> dict:
    padding = 4
    max_w = 512
    x = padding
    y = padding
    row_h = 0
    placements = []
    for entry in static_entries:
        canvas = entry["_canvas"]
        if x + canvas.w + padding > max_w:
            x = padding
            y += row_h + padding
            row_h = 0
        placements.append((entry, x, y))
        x += canvas.w + padding
        row_h = max(row_h, canvas.h)
    atlas_h = y + row_h + padding
    atlas = Canvas(max_w, atlas_h)
    frames = {}
    for entry, px, py in placements:
        canvas = entry["_canvas"]
        atlas.paste(canvas, px, py)
        frames[entry["id"]] = {
            "x": px,
            "y": py,
            "w": canvas.w,
            "h": canvas.h,
            "anchor": entry["anchor"],
            "file": entry["file"],
        }
    save_png(atlas, ROOT / "office_static_atlas.png")
    return {
        "file": "office_static_atlas.png",
        "width": atlas.w,
        "height": atlas.h,
        "frames": frames,
    }


def build_preview(static_entries: list[dict], character_entries: list[dict], animation_entries: list[dict]):
    cell_w, cell_h = 96, 84
    cols = 6
    samples: list[Canvas] = []
    for entry in static_entries:
        samples.append(entry["_canvas"])
    for entry in character_entries[:8]:
        samples.append(first_frame(ROOT / entry["file"], entry["frame_width"], entry["frame_height"]))
    for entry in animation_entries:
        samples.append(first_frame(ROOT / entry["file"], entry["frame_width"], entry["frame_height"]))
    rows = math.ceil(len(samples) / cols)
    c = Canvas(cols * cell_w, rows * cell_h, rgba("#0a0a12"))
    for y in range(0, c.h, 12):
        c.fill_rect(0, y, c.w, 1, rgba("#17172a"))
    for x in range(0, c.w, 12):
        c.fill_rect(x, 0, 1, c.h, rgba("#17172a"))
    for i, sprite in enumerate(samples):
        col = i % cols
        row = i // cols
        px = col * cell_w + (cell_w - sprite.w) // 2
        py = row * cell_h + (cell_h - sprite.h) // 2
        c.paste(sprite, px, py)
    save_png(c, ROOT / "office_asset_preview.png")


def first_frame(path: Path, frame_w: int, frame_h: int) -> Canvas:
    # The generator keeps the source canvases in memory for static assets, but
    # sprite sheets are written before preview. Recreate the first frame from
    # metadata by drawing the same sources where possible.
    name = path.name
    for role, accent in EMPLOYEES.items():
        prefix = f"employee_{role}_"
        if name.startswith(prefix):
            state = name[len(prefix) : -4]
            c = Canvas(frame_w, frame_h)
            draw_employee_frame(c, accent, state, 0)
            return c
    for asset_id, (_rel, fw, fh, _frames, _fps, draw_fn, _tags) in ANIMATION_BUILDERS.items():
        if name == f"{asset_id}.png":
            c = Canvas(fw, fh)
            draw_fn(c, 0)
            return c
    return Canvas(frame_w, frame_h)


def write_manifest(static_entries: list[dict], character_entries: list[dict], animation_entries: list[dict], atlas: dict):
    clean_static = []
    for entry in static_entries:
        entry = dict(entry)
        entry.pop("_canvas", None)
        clean_static.append(entry)
    manifest = {
        "schema": "agentforge.office.assets.v1",
        "tile_size": TILE_SIZE,
        "style": {
            "projection": "top_down_pixel",
            "background": "transparent_png",
            "palette": "dark office with violet, blue, green, and warm wood accents",
        },
        "static_atlas": atlas,
        "categories": {
            "characters": character_entries,
            "furniture": [e for e in clean_static if "furniture" in e["tags"] or "desk" in e["tags"] or "chair" in e["tags"] or "table" in e["tags"]],
            "waiting_room": [e for e in clean_static if "waiting-room" in e["tags"] or "reception" in e["tags"]],
            "decor": [e for e in clean_static if "decor" in e["tags"] or "wall" in e["tags"] or "plant" in e["tags"]],
            "tiles": [e for e in clean_static if "tile" in e["tags"]],
            "animations": animation_entries,
        },
        "usage_notes": [
            "For sprite sheets, draw source rect x = frame_index * frame_width.",
            "Use anchor as the floor contact point when placing assets on the 40 px grid.",
            "Static assets are available as individual PNG files and inside office_static_atlas.png.",
        ],
    }
    (ROOT / "office_assets_manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")


def write_readme():
    text = """# Office Asset Pack

Generated canvas-ready PNG assets for the AgentForge virtual office.

- `sprites/characters`: employee sprite sheets. Each sheet is horizontal and uses fixed frames.
- `sprites/furniture`: desks, chairs, sofas, tables, shelves, and workstation pieces.
- `sprites/waiting_room`: reception and lounge objects.
- `sprites/decor`: plants, lamps, panels, wall items, dividers, and utility props.
- `sprites/tiles`: 40 px floor and wall tiles.
- `sprites/animations`: loopable sprite sheets for monitor glow, steam, plant sway, status lights, typing dots, door scan, and waiting room TV.
- `office_static_atlas.png`: packed atlas for static sprites.
- `office_assets_manifest.json`: frame sizes, anchors, tags, and atlas coordinates.
- `office_asset_preview.png`: visual contact sheet for quick inspection.

All PNG files use transparent backgrounds unless they are tile textures or the preview sheet.
"""
    (ROOT / "README.assets.md").write_text(text, encoding="utf-8")


def main():
    SPRITES.mkdir(parents=True, exist_ok=True)
    static_entries = build_static_assets()
    character_entries = build_character_assets()
    animation_entries = build_animation_assets()
    atlas = build_static_atlas(static_entries)
    build_preview(static_entries, character_entries, animation_entries)
    write_manifest(static_entries, character_entries, animation_entries, atlas)
    write_readme()
    print(f"Generated {len(static_entries)} static assets")
    print(f"Generated {len(character_entries)} character sprite sheets")
    print(f"Generated {len(animation_entries)} animation sprite sheets")
    print(f"Wrote {ROOT / 'office_assets_manifest.json'}")


if __name__ == "__main__":
    main()
