"""Trace a raster logo into a clean SVG.

Works on flat artwork: logos, icons, glyphs. The background is removed, the
image is reduced to a few solid colours, and each one is traced with potrace.
Colour layers are grown slightly and clipped to the traced silhouette, so no
hairline gaps show between them and the outline stays smooth.
"""

import argparse
import sys
from pathlib import Path

import cv2
import numpy as np
import potrace
from PIL import Image


def load(path, crop):
    image = Image.open(path).convert("RGBA")
    if crop:
        x, y, w, h = crop
        image = image.crop((x, y, x + w, y + h))
    return np.array(image)


def upscale(rgba, scale):
    height, width = rgba.shape[:2]
    return cv2.resize(rgba, (width * scale, height * scale), interpolation=cv2.INTER_CUBIC)


def parse_hex(value):
    value = value.lstrip("#")
    return np.array([int(value[i : i + 2], 16) for i in (0, 2, 4)], dtype=np.uint8)


def to_lab(rgb):
    return cv2.cvtColor(rgb.astype(np.uint8), cv2.COLOR_RGB2LAB).astype(np.float32)


def foreground_mask(rgba, background, blur):
    """Pixels that belong to the artwork, as a boolean array."""
    alpha = rgba[..., 3]
    if alpha.min() < 128:
        return alpha >= 128

    rgb = cv2.GaussianBlur(rgba[..., :3], (0, 0), blur)
    if background == "auto":
        corners = np.concatenate(
            [rgb[:8, :8].reshape(-1, 3), rgb[:8, -8:].reshape(-1, 3),
             rgb[-8:, :8].reshape(-1, 3), rgb[-8:, -8:].reshape(-1, 3)]
        )
        reference = np.median(corners, axis=0).astype(np.uint8)
    else:
        reference = parse_hex(background)

    lab = to_lab(rgb)
    target = to_lab(reference.reshape(1, 1, 3))
    distance = np.linalg.norm(lab - target, axis=2)
    scaled = np.clip(distance / max(distance.max(), 1) * 255, 0, 255).astype(np.uint8)
    _, mask = cv2.threshold(scaled, 0, 255, cv2.THRESH_BINARY | cv2.THRESH_OTSU)
    return mask > 0


def quantize(rgba, mask, colors, blur):
    """Reduce the artwork to `colors` flat colours; returns (labels, palette)."""
    rgb = cv2.GaussianBlur(rgba[..., :3], (0, 0), blur)
    # Sample only the inside of the shape: anti-aliased edge pixels are a blend
    # with the background and would otherwise become a colour of their own.
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (9, 9))
    core = cv2.erode(mask.astype(np.uint8), kernel) > 0
    if core.sum() < colors * 16:
        core = mask

    samples = to_lab(rgb[core].reshape(-1, 1, 3)).reshape(-1, 3)
    criteria = (cv2.TERM_CRITERIA_EPS + cv2.TERM_CRITERIA_MAX_ITER, 60, 0.2)
    cv2.setRNGSeed(1)
    _, _, centers = cv2.kmeans(samples, colors, None, criteria, 6, cv2.KMEANS_PP_CENTERS)

    lab = to_lab(rgb)
    distances = np.linalg.norm(lab[:, :, None, :] - centers[None, None, :, :], axis=3)
    labels = distances.argmin(axis=2).astype(np.uint8)

    palette = []
    for index in range(colors):
        chosen = core & (labels == index)
        pixels = rgb[chosen] if chosen.any() else rgb[labels == index]
        palette.append(np.median(pixels, axis=0).astype(np.uint8))
    return labels, palette, core


def extend_from_core(labels, core, min_area):
    """Give edge pixels the colour of the nearest interior pixel.

    An anti-aliased edge is a blend with the background, so it lands on some
    unrelated mid-tone in the palette and shows as a thin fringe.
    """
    # Tiny islands are leftovers of the same blending, typically at sharp tips.
    # Without them the surrounding colour simply grows over those pixels.
    core = core.copy()
    for index in np.unique(labels[core]):
        layer = (core & (labels == index)).astype(np.uint8)
        count, regions, stats, _ = cv2.connectedComponentsWithStats(layer)
        for region in range(1, count):
            if stats[region, cv2.CC_STAT_AREA] < min_area:
                core[regions == region] = False

    _, nearest = cv2.distanceTransformWithLabels(
        (~core).astype(np.uint8), cv2.DIST_L2, 5, labelType=cv2.DIST_LABEL_PIXEL
    )
    lookup = np.zeros(nearest.max() + 1, dtype=np.uint8)
    lookup[nearest[core]] = labels[core]
    return lookup[nearest]


def smooth_labels(labels, colors, radius):
    """Vote each pixel with its neighbours, dropping specks along colour seams."""
    size = radius * 2 + 1
    votes = np.stack(
        [cv2.blur((labels == i).astype(np.float32), (size, size)) for i in range(colors)],
        axis=2,
    )
    return votes.argmax(axis=2).astype(np.uint8)


def curve_path(mask, turdsize, alphamax, tolerance, scale, offset):
    # potracer follows the original tool: dark pixels are the shape.
    bitmap = potrace.Bitmap(np.where(mask, 0, 255).astype(np.uint8))
    curves = bitmap.trace(
        turdsize=turdsize, alphamax=alphamax, opticurve=True, opttolerance=tolerance
    )
    ox, oy = offset

    def point(p):
        return f"{(p.x - ox) / scale:.2f} {(p.y - oy) / scale:.2f}"

    parts = []
    for curve in curves:
        parts.append(f"M{point(curve.start_point)}")
        for segment in curve.segments:
            if segment.is_corner:
                parts.append(f"L{point(segment.c)}L{point(segment.end_point)}")
            else:
                parts.append(
                    f"C{point(segment.c1)} {point(segment.c2)} {point(segment.end_point)}"
                )
        parts.append("Z")
    return "".join(parts)


def polygon_path(mask, epsilon, min_area, scale, offset):
    """Straight-edged outlines, for artwork with no curves in it."""
    contours, hierarchy = cv2.findContours(
        mask.astype(np.uint8), cv2.RETR_CCOMP, cv2.CHAIN_APPROX_NONE
    )
    ox, oy = offset
    parts = []
    for contour in contours:
        if cv2.contourArea(contour) < min_area:
            continue
        points = cv2.approxPolyDP(contour, epsilon * scale, True).reshape(-1, 2)
        coords = "L".join(f"{(x - ox) / scale:.2f} {(y - oy) / scale:.2f}" for x, y in points)
        parts.append(f"M{coords}Z")
    return "".join(parts)


def hex_color(rgb):
    return "#{:02x}{:02x}{:02x}".format(*(int(v) for v in rgb))


def build(args):
    rgba = upscale(load(args.input, args.crop), args.scale)
    blur = 0.35 * args.scale
    mask = foreground_mask(rgba, args.background, blur)

    # Close the JPEG noise out of the mask before anything is measured from it.
    kernel = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (3, 3))
    mask = cv2.morphologyEx(mask.astype(np.uint8), cv2.MORPH_OPEN, kernel) > 0
    if not mask.any():
        sys.exit("Не нашёл рисунка: всё изображение совпало с фоном")

    ys, xs = np.where(mask)
    pad = args.padding * args.scale
    offset = (int(xs.min()) - pad, int(ys.min()) - pad)
    width = (int(xs.max()) - int(xs.min()) + 1 + 2 * pad) / args.scale
    height = (int(ys.max()) - int(ys.min()) + 1 + 2 * pad) / args.scale

    turdsize = args.speckle * args.scale**2
    trace = lambda m: curve_path(m, turdsize, args.alphamax, args.tolerance, args.scale, offset)

    body = []
    if args.colors == 1 or args.polygon:
        path = (
            polygon_path(mask, args.polygon, turdsize, args.scale, offset)
            if args.polygon
            else trace(mask)
        )
        if args.fill:
            fill = args.fill
        else:
            fill = hex_color(np.median(rgba[..., :3][mask], axis=0))
        body.append(f'<path fill="{fill}" fill-rule="evenodd" d="{path}"/>')
    else:
        labels, palette, core = quantize(rgba, mask, args.colors, blur)
        labels = smooth_labels(labels, args.colors, args.scale * 2)
        labels = extend_from_core(labels, core, turdsize * 8)
        body.append(f'<clipPath id="shape"><path clip-rule="evenodd" d="{trace(mask)}"/></clipPath>')
        body.append('<g clip-path="url(#shape)">')

        # Largest first, so smaller patches sit on top of the overlap.
        order = sorted(range(args.colors), key=lambda i: -(mask & (labels == i)).sum())
        grow = cv2.getStructuringElement(cv2.MORPH_ELLIPSE, (args.scale * 2 + 1,) * 2)
        for index in order:
            layer = cv2.dilate((labels == index).astype(np.uint8), grow) > 0
            layer &= cv2.dilate(mask.astype(np.uint8), grow) > 0
            if layer.any():
                body.append(f'<path fill="{hex_color(palette[index])}" d="{trace(layer)}"/>')
        body.append("</g>")

    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width:.2f} {height:.2f}">'
        + "".join(body)
        + "</svg>\n"
    )


def parse_crop(value):
    parts = [int(v) for v in value.split(",")]
    if len(parts) != 4:
        raise argparse.ArgumentTypeError("нужно x,y,ширина,высота")
    return parts


def main():
    parser = argparse.ArgumentParser(description="Трассировка логотипа из растра в SVG")
    parser.add_argument("input", type=Path, help="PNG, JPG или WebP")
    parser.add_argument("-o", "--output", type=Path, help="по умолчанию рядом с исходником")
    parser.add_argument("-c", "--colors", type=int, default=1, help="число цветов (по умолчанию 1)")
    parser.add_argument("--crop", type=parse_crop, help="x,y,ширина,высота в исходных пикселях")
    parser.add_argument("--background", default="auto", help="цвет фона #rrggbb или auto (по углам)")
    parser.add_argument("--fill", help="цвет для одноцветного режима, например currentColor")
    parser.add_argument("--polygon", type=float, default=0, metavar="ЭПСИЛОН",
                        help="только прямые линии, допуск в исходных пикселях (например 1.5)")
    parser.add_argument("--scale", type=int, default=4, help="увеличение перед трассировкой")
    parser.add_argument("--padding", type=int, default=0, help="поля вокруг рисунка, в пикселях")
    parser.add_argument("--speckle", type=int, default=4, help="убирать пятна мельче N пикселей")
    parser.add_argument("--alphamax", type=float, default=1.0,
                        help="скругление углов: 0 острые, 1.3 плавные")
    parser.add_argument("--tolerance", type=float, default=0.4, help="упрощение кривых")
    args = parser.parse_args()

    output = args.output or args.input.with_suffix(".svg")
    svg = build(args)
    output.write_text(svg, encoding="utf-8")
    print(f"{output}: {len(svg) / 1024:.1f} КБ")


if __name__ == "__main__":
    main()
