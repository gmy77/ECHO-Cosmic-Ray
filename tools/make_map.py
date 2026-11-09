"""Genera la sovrapposizione cartografica (coste + graticola) per il dashboard.

Output: assets/worldmap.png  (1000x500, proiezione equirettangolare, sfondo trasparente)
"""
import json
import math
import os

from PIL import Image, ImageDraw, ImageFilter

W, H = 1200, 600
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(ROOT, "tools", "ne_110m_land.geojson")
OUT = os.path.join(ROOT, "assets", "worldmap.png")


def project(lon, lat):
    x = (lon + 180.0) / 360.0 * W
    y = (90.0 - lat) / 180.0 * H
    return x, y


def load_rings(path):
    with open(path, "r", encoding="utf-8") as handle:
        data = json.load(handle)
    rings = []
    for feature in data["features"]:
        geom = feature["geometry"]
        kind = geom["type"]
        coords = geom["coordinates"]
        polygons = coords if kind == "MultiPolygon" else [coords]
        for polygon in polygons:
            for ring in polygon:
                rings.append([project(lon, lat) for lon, lat in ring])
    return rings


def main():
    rings = load_rings(SRC)

    land = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    draw = ImageDraw.Draw(land)
    for ring in rings:
        draw.polygon(ring, fill=(10, 20, 30, 70))

    # alone luminosa sotto le coste
    glow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    for ring in rings:
        gd.line(ring + [ring[0]], fill=(56, 189, 248, 110), width=3)
    glow = glow.filter(ImageFilter.GaussianBlur(3.0))

    coast = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    cd = ImageDraw.Draw(coast)
    for ring in rings:
        cd.line(ring + [ring[0]], fill=(125, 211, 252, 190), width=1)

    grid = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    gd = ImageDraw.Draw(grid)
    for lon in range(-150, 180, 30):
        x, _ = project(lon, 0)
        a = 60 if lon == 0 else 26
        gd.line([(x, 0), (x, H)], fill=(148, 163, 184, a), width=1)
    for lat in range(-60, 90, 30):
        _, y = project(0, lat)
        a = 60 if lat == 0 else 26
        gd.line([(0, y), (W, y)], fill=(148, 163, 184, a), width=1)

    out = Image.alpha_composite(land, glow)
    out = Image.alpha_composite(out, coast)
    out = Image.alpha_composite(out, grid)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    out.save(OUT, optimize=True)
    print("scritto", OUT, out.size, os.path.getsize(OUT), "byte")


if __name__ == "__main__":
    main()
