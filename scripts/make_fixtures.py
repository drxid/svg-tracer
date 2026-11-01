"""Эталонные входы для проверки качества. Все картинки синтетические и воспроизводимые.

Запуск: .venv\\Scripts\\python scripts\\make_fixtures.py
"""

import math
import random
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

OUT = Path(__file__).parent.parent / "tests" / "fixtures" / "generated"
SUPERSAMPLE = 4


def smooth(image, size):
    """Рисуем крупно и уменьшаем: края получаются как у настоящего растра."""
    return image.resize((size, size), Image.LANCZOS)


def badge(size, background, colors):
    big = size * SUPERSAMPLE
    image = Image.new("RGBA", (big, big), background)
    draw = ImageDraw.Draw(image)
    margin = big * 0.08
    draw.ellipse((margin, margin, big - margin, big - margin), fill=colors[0])
    center = big / 2
    for index, color in enumerate(colors[1:] * 2):
        angle = index * math.tau / (len(colors[1:]) * 2)
        x, y = center + math.cos(angle) * big * 0.24, center + math.sin(angle) * big * 0.24
        radius = big * 0.11
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=color)
    return smooth(image, size)


def two_colors():
    big = 2048
    image = Image.new("RGB", (big, big), "white")
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((200, 200, 1848, 1848), 300, fill="#1a56db")
    draw.polygon([(700, 1400), (1024, 500), (1348, 1400)], fill="#f7c948")
    return image.resize((512, 512), Image.LANCZOS)


def scan():
    rng = random.Random(7)
    big = 1600
    image = Image.new("L", (big, big), 235)
    draw = ImageDraw.Draw(image)
    for y in range(180, big - 150, 160):
        points = [(x, y + 34 * math.sin(x / 90 + y)) for x in range(120, big - 120, 16)]
        draw.line(points, fill=25, width=16)
    image = image.resize((800, 800), Image.LANCZOS).filter(ImageFilter.GaussianBlur(0.6))
    for _ in range(4000):
        image.putpixel((rng.randrange(800), rng.randrange(800)), rng.randrange(180, 255))
    return image.convert("RGB")


def illustration():
    # Шум обязателен: идеально гладкий градиент VTracer сводит к одному цвету
    # (0.6 и 1.0 одинаково), а настоящие иллюстрации такими не бывают.
    rng = random.Random(5)
    size = 800
    image = Image.new("RGB", (size, size))
    pixels = image.load()
    for y in range(size):
        for x in range(size):
            wave = math.sin(x / 70) * math.cos(y / 90)
            noise = rng.randint(-12, 12)
            pixels[x, y] = (
                min(255, max(0, int(120 + 110 * wave) + noise)),
                min(255, max(0, int(x * 255 / size) + noise)),
                min(255, max(0, int(y * 255 / size) + noise)),
            )
    return image


def pixel_art():
    rng = random.Random(3)
    colors = ["#2b2d42", "#ef233c", "#edf2f4", "#8d99ae"]
    tiles = Image.new("RGB", (24, 24), colors[2])
    for y in range(24):
        for x in range(12):
            if rng.random() < 0.55:
                color = rng.choice(colors)
                tiles.putpixel((x, y), Image.new("RGB", (1, 1), color).getpixel((0, 0)))
                tiles.putpixel((23 - x, y), Image.new("RGB", (1, 1), color).getpixel((0, 0)))
    return tiles.resize((480, 480), Image.NEAREST)


def large():
    image = illustration().resize((6400, 6400), Image.BICUBIC)
    draw = ImageDraw.Draw(image)
    draw.ellipse((800, 800, 5600, 5600), outline="#000000", width=120)
    return image


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    seven = ["#1f2a5a", "#f4582c", "#17a398", "#f7b32b", "#e8e0d0", "#7b2d8b", "#2e8b57"]

    two_colors().save(OUT / "logo-2.png")
    badge(800, (255, 255, 255, 255), seven).convert("RGB").save(OUT / "logo-7-white.png")
    badge(800, (0, 0, 0, 0), seven[:4]).save(OUT / "logo-transparent.png")
    badge(640, (255, 255, 255, 255), seven[:5]).convert("RGB").save(OUT / "logo-jpeg.jpg", quality=55)
    badge(32, (255, 255, 255, 255), seven[:2]).convert("RGB").save(OUT / "icon-32.png")
    scan().save(OUT / "scan.png")
    illustration().save(OUT / "illustration.png")
    pixel_art().save(OUT / "pixel-art.png")
    large().save(OUT / "large-40mp.jpg", quality=90)
    print(f"готово: {OUT}")


if __name__ == "__main__":
    main()
