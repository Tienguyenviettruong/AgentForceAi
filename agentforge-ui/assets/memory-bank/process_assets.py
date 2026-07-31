from __future__ import annotations

import json
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont, ImageOps


ROOT = Path(__file__).resolve().parent
COMPONENTS = (
    "project-core.png",
    "project-brief.png",
    "architecture.png",
    "context.png",
    "progress.png",
    "lessons.png",
    "decisions.png",
    "data-bridge-ne.png",
    "memory-sentinel.png",
)
MAX_EDGE = 1024
PADDING_RATIO = 0.045


def crop_and_resize(path: Path) -> Image.Image:
    image = Image.open(path).convert("RGBA")
    alpha_bbox = image.getchannel("A").getbbox()
    if alpha_bbox is None:
        raise ValueError(f"{path.name} contains no visible pixels")

    left, top, right, bottom = alpha_bbox
    padding = max(12, round(max(right - left, bottom - top) * PADDING_RATIO))
    crop_box = (
        max(0, left - padding),
        max(0, top - padding),
        min(image.width, right + padding),
        min(image.height, bottom + padding),
    )
    image = image.crop(crop_box)

    scale = min(1.0, MAX_EDGE / max(image.size))
    if scale < 1.0:
        size = (max(1, round(image.width * scale)), max(1, round(image.height * scale)))
        image = image.resize(size, Image.Resampling.LANCZOS)

    image.save(path, optimize=True)
    return image


def make_preview(paths: list[Path]) -> None:
    cell_width = 300
    cell_height = 330
    columns = 4
    rows = (len(paths) + columns - 1) // columns
    preview = Image.new("RGB", (columns * cell_width, rows * cell_height), "#0a1119")
    draw = ImageDraw.Draw(preview)
    font = ImageFont.load_default(size=18)

    for index, path in enumerate(paths):
        image = Image.open(path).convert("RGBA")
        image.thumbnail((260, 270), Image.Resampling.LANCZOS)
        column = index % columns
        row = index // columns
        x = column * cell_width
        y = row * cell_height

        draw.rounded_rectangle(
            (x + 8, y + 8, x + cell_width - 8, y + cell_height - 8),
            radius=8,
            fill="#101a25",
            outline="#26384a",
            width=1,
        )
        image_x = x + (cell_width - image.width) // 2
        image_y = y + 20 + (270 - image.height) // 2
        preview.paste(image, (image_x, image_y), image)
        label = path.stem.replace("-", " ").title()
        draw.text((x + 18, y + 298), label, fill="#d8e8f4", font=font)

    preview.save(ROOT / "memory-bank-assets-preview.png", optimize=True)


def main() -> None:
    processed: list[Path] = []
    for filename in COMPONENTS:
        path = ROOT / filename
        if not path.exists():
            raise FileNotFoundError(path)
        crop_and_resize(path)
        processed.append(path)

    bridge_ne = Image.open(ROOT / "data-bridge-ne.png").convert("RGBA")
    bridge_nw_path = ROOT / "data-bridge-nw.png"
    ImageOps.mirror(bridge_ne).save(bridge_nw_path, optimize=True)
    processed.append(bridge_nw_path)

    manifest = []
    for path in processed:
        with Image.open(path) as image:
            alpha = image.getchannel("A")
            manifest.append(
                {
                    "id": path.stem,
                    "file": path.name,
                    "width": image.width,
                    "height": image.height,
                    "alpha_bbox": list(alpha.getbbox() or (0, 0, 0, 0)),
                    "bytes": path.stat().st_size,
                }
            )

    (ROOT / "assets-manifest.json").write_text(
        json.dumps({"version": 1, "assets": manifest}, indent=2) + "\n",
        encoding="utf-8",
    )
    make_preview(processed)


if __name__ == "__main__":
    main()
