"""Disegna l'icona dell'app (sciame di raggi cosmici su crinale) e la esporta in .ico multi-size."""
import math
import os
from PIL import Image, ImageDraw, ImageFilter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_ICO = os.path.join(ROOT, "assets", "app.ico")
OUT_PNG = os.path.join(ROOT, "assets", "app-icon.png")

S = 4096  # canvas di lavoro (4x supersampling di 1024)
K = S / 1024.0


def sc(v):
    return v * K


def lerp(a, b, t):
    return tuple(int(round(a[i] + (b[i] - a[i]) * t)) for i in range(len(a)))


def main():
    # --- sfondo con gradiente radiale ---
    bg = Image.new("RGBA", (S, S), (7, 11, 16, 255))
    px = bg.load()
    cx, cy = S * 0.5, S * 0.42
    hi = (26, 48, 66)
    lo = (7, 11, 16)
    maxd = math.hypot(S * 0.62, S * 0.62)
    step = 4

    for y in range(0, S, step):
        for x in range(0, S, step):
            d = math.hypot(x - cx, y - cy) / maxd
            d = min(1.0, d) ** 1.35
            c = lerp(hi, lo, d)
            for dy in range(step):
                for dx in range(step):
                    if x + dx < S and y + dy < S:
                        px[x + dx, y + dy] = (c[0], c[1], c[2], 255)

    # --- graticola tenue ---
    grid = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    gd = ImageDraw.Draw(grid)
    for i in range(1, 6):
        y = S * i / 6.0
        gd.line([(sc(90), y), (S - sc(90), y)], fill=(56, 189, 248, 26), width=int(sc(2)))
    for i in range(1, 6):
        x = S * i / 6.0
        gd.line([(x, sc(90)), (x, S - sc(90))], fill=(56, 189, 248, 22), width=int(sc(2)))
    gd.arc([sc(60), sc(60), S - sc(60), S - sc(60)], 200, 340, fill=(56, 189, 248, 34), width=int(sc(3)))

    # --- crinale montuoso ---
    ridge_pts = [
        (sc(70), sc(870)), (sc(230), sc(790)), (sc(360), sc(830)), (sc(470), sc(742)),
        (sc(560), sc(800)), (sc(648), sc(700)), (sc(720), sc(770)), (sc(820), sc(712)),
        (sc(954), sc(830)), (sc(954), sc(954)), (sc(70), sc(954)),
    ]
    ridge = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    rd = ImageDraw.Draw(ridge)
    rd.polygon(ridge_pts, fill=(11, 20, 29, 255))
    rd.line(ridge_pts[:9], fill=(34, 211, 238, 120), width=int(sc(3)))

    # --- sciame ---
    shower = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shower)
    top = (sc(512), sc(120))
    core = (sc(512), sc(560))

    # raggio primario
    sd.line([top, core], fill=(34, 211, 238, 235), width=int(sc(11)))
    sd.line([top, core], fill=(224, 252, 255, 190), width=int(sc(4)))

    # rami dello sciame
    branches = [
        (sc(330), sc(840), 4, 118), (sc(392), sc(792), 4, 132), (sc(452), sc(760), 3, 150),
        (sc(572), sc(752), 3, 150), (sc(632), sc(786), 4, 132), (sc(694), sc(842), 4, 118),
        (sc(258), sc(880), 3, 92), (sc(766), sc(884), 3, 92),
    ]
    for bx, by, width, alpha in branches:
        sd.line([core, (bx, by)], fill=(34, 211, 238, alpha), width=int(sc(width)))
        # particelle lungo il ramo
        for t in (0.35, 0.62, 0.85):
            px_ = core[0] + (bx - core[0]) * t
            py_ = core[1] + (by - core[1]) * t
            r = sc(7 - 3 * t)
            sd.ellipse([px_ - r, py_ - r, px_ + r, py_ + r], fill=(165, 243, 252, int(alpha * 0.9)))

    # punto d'impatto (ambra)
    r = sc(46)
    sd.ellipse([core[0] - r, core[1] - r, core[0] + r, core[1] + r], fill=(245, 158, 11, 60))
    r = sc(20)
    sd.ellipse([core[0] - r, core[1] - r, core[0] + r, core[1] + r], fill=(251, 191, 36, 210))
    r = sc(9)
    sd.ellipse([core[0] - r, core[1] - r, core[0] + r, core[1] + r], fill=(255, 251, 235, 255))

    # alone
    glow = shower.filter(ImageFilter.GaussianBlur(sc(26)))
    core_glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    cgd = ImageDraw.Draw(core_glow)
    r = sc(120)
    cgd.ellipse([top[0] - r, top[1] - r, top[0] + r, top[1] + r], fill=(34, 211, 238, 70))
    core_glow = core_glow.filter(ImageFilter.GaussianBlur(sc(40)))

    img = Image.alpha_composite(bg, grid)
    img = Image.alpha_composite(img, ridge)
    img = Image.alpha_composite(img, glow)
    img = Image.alpha_composite(img, core_glow)
    img = Image.alpha_composite(img, shower)

    # --- maschera arrotondata ---
    mask = Image.new("L", (S, S), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=int(sc(190)), fill=255)
    out = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    out.paste(img, (0, 0), mask)

    big = out.resize((1024, 1024), Image.LANCZOS)
    big.save(OUT_PNG)

    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    frames = [big.resize((s, s), Image.LANCZOS) for s in sizes]
    frames[-1].save(OUT_ICO, format="ICO", sizes=[(s, s) for s in sizes], append_images=frames[:-1])
    print("scritti", OUT_PNG, "e", OUT_ICO, os.path.getsize(OUT_ICO), "byte")


if __name__ == "__main__":
    main()
