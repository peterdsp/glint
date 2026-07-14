#!/usr/bin/env python3
"""Marketing images for the App Store and Ko-fi, built from the same Glint
panel + tokens as generate_marketing.py. Outputs to docs/assets/store/.

App Store macOS screenshots must be one of 1280x800 / 1440x900 / 2560x1600 /
2880x1800. We author at 1280x800 and rasterize at 2x -> 2560x1600."""
import subprocess, os
from generate_marketing import THEMES, panel, txt, desktop_bg, esc

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "store")
os.makedirs(OUT, exist_ok=True)


def grad(idp, stops):
    a, b, c = stops
    return (f'<linearGradient id="{idp}" x1="0" y1="0" x2="1" y2="1">'
            f'<stop offset="0" stop-color="{a}"/><stop offset="0.55" stop-color="{b}"/>'
            f'<stop offset="1" stop-color="{c}"/></linearGradient>')


def shadow(idp="sh", dy=22, blur=28, op=0.4):
    return (f'<filter id="{idp}" x="-40%" y="-40%" width="180%" height="180%">'
            f'<feDropShadow dx="0" dy="{dy}" stdDeviation="{blur}" '
            f'flood-color="#0e1024" flood-opacity="{op}"/></filter>')


def svg_open(w, h):
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">'


def bullet(x, y, text, color, sub=None):
    g = [f'<rect x="{x}" y="{y-11}" width="12" height="12" rx="3" fill="{color}" transform="rotate(45 {x+6} {y-5})"/>']
    g.append(txt(x + 26, y, text, 22, "#ffffff", 600))
    if sub:
        g.append(txt(x + 26, y + 26, sub, 16, "rgba(255,255,255,0.8)"))
    return "".join(g)


# ---------------------------------------------------------------- Ko-fi cover
def kofi_cover():
    W, H = 1600, 1000
    p, ph = panel("aurora")
    scale = 1.5
    px = W - 340 * scale - 110
    py = (H - ph * scale) / 2 - 10
    b = [svg_open(W, H)]
    b.append(f'<defs>{grad("g", THEMES["aurora"]["bg"])}{shadow()}</defs>')
    b.append(f'<rect width="{W}" height="{H}" fill="url(#g)"/>')
    b.append('<circle cx="300" cy="240" r="360" fill="rgba(255,255,255,0.16)"/>')
    b.append('<circle cx="120" cy="820" r="220" fill="rgba(255,255,255,0.10)"/>')
    b.append(txt(120, 300, "Glint", 128, "#ffffff", 600))
    b.append(txt(124, 372, "A glint of Git in your menu bar.", 34, "rgba(255,255,255,0.95)"))
    b.append(bullet(126, 470, "Native menu-bar app, real liquid glass", "#ffffff"))
    b.append(bullet(126, 530, "One-click commit, pull, push and diffs", "#ffffff"))
    b.append(bullet(126, 590, "Five themes, three languages, auto-updates", "#ffffff"))
    # price pill
    b.append('<rect x="124" y="660" width="290" height="72" rx="36" fill="#ffffff"/>')
    b.append(txt(150, 706, "4.99", 40, "#3a54b8", 700))
    b.append(txt(250, 700, "one-time", 18, "#5a5f6b", 600))
    b.append(txt(250, 722, "macOS - Win - Linux", 14, "#5a5f6b"))
    b.append(f'<g filter="url(#sh)" transform="translate({px},{py}) scale({scale})">{p}</g>')
    b.append('</svg>')
    return "\n".join(b), W, H


# ------------------------------------------------------ App Store screenshots
def shot_hero():
    W, H = 1280, 800
    p, ph = panel("aurora")
    scale = 1.18
    px = W - 340 * scale - 90
    py = (H - ph * scale) / 2 + 10
    b = [svg_open(W, H)]
    b.append(f'<defs>{grad("g", THEMES["aurora"]["bg"])}{shadow()}</defs>')
    b.append(f'<rect width="{W}" height="{H}" fill="url(#g)"/>')
    b.append('<circle cx="230" cy="180" r="300" fill="rgba(255,255,255,0.16)"/>')
    b.append(txt(90, 250, "Your repo, one click", 60, "#ffffff", 700))
    b.append(txt(90, 320, "from the menu bar.", 60, "#ffffff", 700))
    b.append(txt(94, 380, "Stage, commit, pull, push and read diffs", 24, "rgba(255,255,255,0.92)"))
    b.append(txt(94, 414, "from a tiny glass panel.", 24, "rgba(255,255,255,0.92)"))
    b.append(f'<g filter="url(#sh)" transform="translate({px},{py}) scale({scale})">{p}</g>')
    b.append('</svg>')
    return "\n".join(b), W, H


def shot_themes():
    names = ["aurora", "midnight", "sunset", "forest", "graphite"]
    W, H = 1280, 800
    b = [svg_open(W, H)]
    b.append(f'<defs>{shadow("sh2", 14, 18, 0.5)}')
    for n in names:
        b.append(grad(f"bg_{n}", THEMES[n]["bg"]))
    b.append('</defs>')
    b.append(f'<rect width="{W}" height="{H}" fill="#0e1017"/>')
    b.append(txt(W / 2, 108, "Five themes. Or your own.", 52, "#ffffff", 700, anchor="middle"))
    b.append(txt(W / 2, 150, "Same glass, swapped tokens. Light or dark, it never changes shape.",
                 22, "rgba(255,255,255,0.72)", anchor="middle"))
    scale = 0.6
    pw = 340 * scale
    _, ph = panel("aurora")
    gap = 24
    total = len(names) * pw + (len(names) - 1) * gap
    x = (W - total) / 2
    for n in names:
        b.append(f'<g transform="translate({x},210)">')
        b.append(f'<rect x="-12" y="-12" width="{pw+24}" height="{ph*scale+24}" rx="20" fill="url(#bg_{n})"/>')
        pp, _ = panel(n)
        b.append(f'<g filter="url(#sh2)" transform="scale({scale})">{pp}</g>')
        b.append(txt(pw / 2, ph * scale + 26, THEMES[n]["label"], 16, "#ffffff", 600, anchor="middle"))
        b.append('</g>')
        x += pw + gap
    b.append('</svg>')
    return "\n".join(b), W, H


def shot_features():
    W, H = 1280, 800
    p, ph = panel("midnight")
    scale = 1.12
    px = 110
    py = (H - ph * scale) / 2
    b = [svg_open(W, H)]
    b.append(f'<defs>{grad("g", THEMES["midnight"]["bg"])}{shadow()}</defs>')
    b.append(f'<rect width="{W}" height="{H}" fill="url(#g)"/>')
    tx = px + 340 * scale + 80
    b.append(txt(tx, 210, "Tiny. Native. Yours.", 52, "#ffffff", 700))
    b.append(bullet(tx, 300, "Megabytes, not hundreds of them", "#7c8cff",
                    "Tauri + Rust, no bundled Chromium"))
    b.append(bullet(tx, 400, "Real macOS vibrancy and Mica on Windows", "#7c8cff",
                    "Falls back to a CSS tint on Linux"))
    b.append(bullet(tx, 500, "Diff pop-out and live PR + CI status", "#7c8cff",
                    "English, Greek and Albanian"))
    b.append(bullet(tx, 600, "Automatic background updates", "#7c8cff",
                    "Signed, verified, no prompts"))
    b.append(f'<g filter="url(#sh)" transform="translate({px},{py}) scale({scale})">{p}</g>')
    b.append('</svg>')
    return "\n".join(b), W, H


ASSETS = {
    "kofi-cover": (kofi_cover, 2),
    "appstore-1-hero": (shot_hero, 2),
    "appstore-2-themes": (shot_themes, 2),
    "appstore-3-features": (shot_features, 2),
}

for name, (fn, zoom) in ASSETS.items():
    svg, w, h = fn()
    svg_path = os.path.join(OUT, f"{name}.svg")
    png_path = os.path.join(OUT, f"{name}.png")
    open(svg_path, "w").write(svg)
    subprocess.run(["rsvg-convert", "-z", str(zoom), svg_path, "-o", png_path], check=True)
    print(f"{name}: {w*zoom}x{h*zoom}  {os.path.getsize(png_path)//1024} KB")
