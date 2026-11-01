"""Сверка порта логотипов с `trace.py`.

Условие приёмки: SSIM >= 0.98 и разница числа узлов <= 15 % на всех логотипах.
Запуск: .venv\\Scripts\\python scripts\\compare_with_trace_py.py <potrace.exe>

`foreground_mask` эталона подменяется фиксированным порогом: Оцу намеренно не
перенесён, и сравнивать порт с его дырами бессмысленно.
"""

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import cv2
import numpy as np

ROOT = Path(__file__).parent.parent
sys.path.insert(0, str(ROOT / "legacy"))
import trace as reference  # noqa: E402

EXAMPLE = ROOT / "src-tauri" / "target" / "release" / "examples" / "trace_file.exe"
FIXTURES = ROOT / "tests" / "fixtures"
RENDER_WIDTH = 1024
DISTANCE = 20.0
SSIM_MIN = 0.98
NODES_MAX = 0.15

LOGOS = [
    (FIXTURES / "generated" / "logo-2.png", 2),
    (FIXTURES / "generated" / "logo-7-white.png", 7),
    (FIXTURES / "generated" / "logo-transparent.png", 4),
    (FIXTURES / "generated" / "logo-jpeg.jpg", 5),
    (FIXTURES / "logo-7-colors.jpg", 4),
]


def fixed_mask(rgba, background, blur):
    alpha = rgba[..., 3]
    if alpha.min() < 128:
        return alpha >= 128
    rgb = cv2.GaussianBlur(rgba[..., :3], (0, 0), blur)
    corners = np.concatenate(
        [rgb[:8, :8].reshape(-1, 3), rgb[:8, -8:].reshape(-1, 3),
         rgb[-8:, :8].reshape(-1, 3), rgb[-8:, -8:].reshape(-1, 3)]
    )
    target = reference.to_lab(np.median(corners, axis=0).astype(np.uint8).reshape(1, 1, 3))
    return np.linalg.norm(reference.to_lab(rgb) - target, axis=2) > DISTANCE


def run_reference(path, colors, output):
    reference.foreground_mask = fixed_mask
    args = argparse.Namespace(
        input=path, crop=None, background="auto", fill=None, polygon=0, scale=4, padding=0,
        speckle=4, alphamax=1.0, tolerance=0.4, colors=colors,
    )
    output.write_text(reference.build(args), encoding="utf-8")


def settings(colors):
    speckle = {"speckle": 4, "alphamax": 1.0, "tolerance": 0.4, "fill": "source"}
    return {
        "mode": "logo",
        "preprocess": {
            "crop": None, "scale": 4, "background": "auto", "backgroundDistance": DISTANCE,
            "cropToArtwork": True, "padding": 0,
        },
        "color": {
            "hierarchical": "stacked", "lines": "spline", "speckle": 4, "colorPrecision": 6,
            "gradientStep": 16, "simplify": 1, "maxColors": 0,
        },
        "outline": {**speckle, "threshold": 128, "invert": False},
        "logo": {**speckle, "colors": colors, "polygon": 0},
    }


def run_port(path, colors, output, potrace, workdir):
    config = workdir / "settings.json"
    config.write_text(json.dumps(settings(colors)), encoding="utf-8")
    done = subprocess.run(
        [EXAMPLE, "trace", path, config, output, potrace], capture_output=True, text=True, check=True
    )
    return int(done.stdout.strip())


def render(svg, png):
    subprocess.run([EXAMPLE, "render", svg, png, str(RENDER_WIDTH)], check=True)
    return cv2.cvtColor(cv2.imread(str(png)), cv2.COLOR_BGR2GRAY).astype(np.float64)


def ssim(a, b):
    """SSIM с гауссовым окном, как в исходной статье."""
    c1, c2 = (0.01 * 255) ** 2, (0.03 * 255) ** 2
    blur = lambda image: cv2.GaussianBlur(image, (11, 11), 1.5)
    mean_a, mean_b = blur(a), blur(b)
    var_a = blur(a * a) - mean_a**2
    var_b = blur(b * b) - mean_b**2
    cov = blur(a * b) - mean_a * mean_b
    numerator = (2 * mean_a * mean_b + c1) * (2 * cov + c2)
    denominator = (mean_a**2 + mean_b**2 + c1) * (var_a + var_b + c2)
    return float((numerator / denominator).mean())


def nodes(svg):
    text = svg.read_text(encoding="utf-8")
    return sum(len(re.findall(r"[a-zA-Z]", d)) for d in re.findall(r'\sd="([^"]*)"', text))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("potrace", type=Path)
    args = parser.parse_args()

    failed = 0
    print(f"{'файл':<24}{'SSIM':>8}{'узлы эталон':>13}{'узлы порт':>11}{'разница':>9}{'мс':>7}")
    with tempfile.TemporaryDirectory() as temp:
        work = Path(temp)
        for path, colors in LOGOS:
            ours, theirs = work / "port.svg", work / "reference.svg"
            run_reference(path, colors, theirs)
            elapsed = run_port(path, colors, ours, args.potrace, work)
            score = ssim(render(ours, work / "a.png"), render(theirs, work / "b.png"))
            count_a, count_b = nodes(ours), nodes(theirs)
            delta = abs(count_a - count_b) / max(count_b, 1)
            good = score >= SSIM_MIN and delta <= NODES_MAX
            failed += not good
            mark = "" if good else "  НЕ ПРОШЁЛ"
            print(f"{path.name:<24}{score:>8.4f}{count_b:>13}{count_a:>11}{delta:>8.1%}{elapsed:>7}{mark}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
