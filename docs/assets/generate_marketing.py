#!/usr/bin/env python3
"""Generate Glint marketing art as spec-faithful SVG, then rasterize to PNG.
Every panel is drawn from the exact design tokens in docs/design-language.md."""
import subprocess, os

THEMES = {
    "aurora":   dict(accent="#5b8cff", tint="#eef2ff", tint2="#dfe6ff", field="#ffffff",
                     stroke="#ffffff", ink="#1b1d22", ink2="#5a5f6b",
                     bg=("#8ea8ff", "#c9a3ff", "#ffc4d6"), label="Aurora"),
    "midnight": dict(accent="#7c8cff", tint="#20242f", tint2="#2b3040", field="#2b3040",
                     stroke="#3a4050", ink="#f2f3f8", ink2="#a7adbf",
                     bg=("#161a26", "#1f2333", "#2a2140"), label="Midnight"),
    "sunset":   dict(accent="#ff7a59", tint="#fff4ee", tint2="#ffe6db", field="#ffffff",
                     stroke="#ffffff", ink="#2a1c18", ink2="#7a5a4e",
                     bg=("#ffb37a", "#ff7a9c", "#c85adf"), label="Sunset"),
    "forest":   dict(accent="#1d9e75", tint="#eef8f4", tint2="#dcefe8", field="#ffffff",
                     stroke="#ffffff", ink="#17251d", ink2="#4e6a5c",
                     bg=("#8fd6b0", "#5fb8a0", "#3d9d8f"), label="Forest"),
    "graphite": dict(accent="#3a3f4a", tint="#f5f6f8", tint2="#e7e9ee", field="#ffffff",
                     stroke="#ffffff", ink="#1b1d22", ink2="#5f636e",
                     bg=("#d7d9df", "#c2c5cd", "#aeb2bd"), label="Graphite"),
}
ADD, DEL = "#1d9e75", "#d85a30"

def esc(s): return s.replace("&", "&amp;").replace("<", "&lt;")

def rr(x, y, w, h, r, fill, stroke=None, sw=0.5, extra=""):
    s = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}"'
    if stroke: s += f' stroke="{stroke}" stroke-width="{sw}"'
    return s + f' {extra}/>'

def txt(x, y, s, size, fill, weight=400, mono=False, anchor="start"):
    fam = "SF Mono, Menlo, monospace" if mono else "SF Pro Text, Helvetica, Arial, sans-serif"
    return (f'<text x="{x}" y="{y}" font-family="{fam}" font-size="{size}" '
            f'font-weight="{weight}" fill="{fill}" text-anchor="{anchor}" '
            f'letter-spacing="{0.5 if size<=11 and weight==500 else 0}">{esc(s)}</text>')

def icon(name, cx, cy, s, color):
    """Minimal stroked icons, centered on (cx,cy), roughly s tall."""
    a = dict(fill="none", stroke=color)
    st = f'stroke="{color}" fill="none" stroke-width="{max(1.4,s/9):.2f}" stroke-linecap="round" stroke-linejoin="round"'
    h = s / 2
    if name == "branch":
        return (f'<g {st}><circle cx="{cx-h*0.6}" cy="{cy-h*0.7}" r="{s*0.16}"/>'
                f'<circle cx="{cx-h*0.6}" cy="{cy+h*0.7}" r="{s*0.16}"/>'
                f'<circle cx="{cx+h*0.7}" cy="{cy-h*0.2}" r="{s*0.16}"/>'
                f'<path d="M{cx-h*0.6},{cy-h*0.5} L{cx-h*0.6},{cy+h*0.5}"/>'
                f'<path d="M{cx+h*0.7},{cy} q0,{h*0.6} -{h*1.3},{h*0.6}"/></g>')
    if name == "fork":
        return (f'<g {st}><circle cx="{cx-h*0.6}" cy="{cy-h*0.6}" r="{s*0.15}"/>'
                f'<circle cx="{cx+h*0.6}" cy="{cy-h*0.6}" r="{s*0.15}"/>'
                f'<circle cx="{cx}" cy="{cy+h*0.7}" r="{s*0.15}"/>'
                f'<path d="M{cx-h*0.6},{cy-h*0.4} q0,{h*0.6} {h*0.6},{h*0.7}"/>'
                f'<path d="M{cx+h*0.6},{cy-h*0.4} q0,{h*0.6} -{h*0.6},{h*0.7}"/></g>')
    if name == "down":
        return f'<g {st}><path d="M{cx},{cy-h} L{cx},{cy+h} M{cx-h*0.6},{cy+h*0.3} L{cx},{cy+h} L{cx+h*0.6},{cy+h*0.3}"/></g>'
    if name == "up":
        return f'<g {st}><path d="M{cx},{cy+h} L{cx},{cy-h} M{cx-h*0.6},{cy-h*0.3} L{cx},{cy-h} L{cx+h*0.6},{cy-h*0.3}"/></g>'
    if name == "refresh":
        return (f'<g {st}><path d="M{cx+h*0.9},{cy-h*0.2} a{h*0.9},{h*0.9} 0 1 0 {h*0.2},{h*0.9}"/>'
                f'<path d="M{cx+h*0.55},{cy-h} L{cx+h*0.9},{cy-h*0.2} L{cx+h*0.05},{cy}"/></g>')
    if name == "check":
        return f'<g {st}><path d="M{cx-h*0.55},{cy} L{cx-h*0.1},{cy+h*0.5} L{cx+h*0.6},{cy-h*0.5}"/></g>'
    if name == "selector":
        return (f'<g {st}><path d="M{cx-h*0.5},{cy-h*0.25} L{cx},{cy-h*0.7} L{cx+h*0.5},{cy-h*0.25}"/>'
                f'<path d="M{cx-h*0.5},{cy+h*0.25} L{cx},{cy+h*0.7} L{cx+h*0.5},{cy+h*0.25}"/></g>')
    return ""

def panel(t, x0=0, y0=0, files=True):
    """Draw a 340-wide Glint panel at (x0,y0). Returns (svg, height)."""
    T = THEMES[t]
    W = 340
    g = [f'<g transform="translate({x0},{y0})">']
    H = 430
    # panel body + inner highlight
    g.append(rr(0, 0, W, H, 18, T["tint"], T["stroke"], 0.5))
    g.append(f'<rect x="0.5" y="1" width="{W-1}" height="{H-2}" rx="17.5" fill="none" stroke="rgba(255,255,255,0.35)" stroke-width="1"/>')
    # header
    g.append(rr(14, 14, 30, 30, 9, T["accent"]))
    g.append(icon("branch", 29, 29, 17, "#ffffff"))
    g.append(txt(54, 30, "peterdsp / glint", 14, T["ink"], 500))
    g.append(icon("fork", 60, 40, 12, T["ink2"]))
    g.append(txt(70, 44, "main", 12, T["ink2"]))
    g.append(icon("selector", W-24, 29, 16, T["ink2"]))
    g.append(f'<line x1="14" y1="58" x2="{W-14}" y2="58" stroke="{T["tint2"]}" stroke-width="1"/>')
    # sync
    py = 70
    for i, (ic, num, lbl, ex) in enumerate([("down", "2", "to pull", 0), ("up", "5", "to push", 1)]):
        px = 14 + ex * 150
        g.append(rr(px, py, 142, 44, 11, T["tint2"], T["stroke"], 0.5))
        g.append(icon(ic, px + 18, py + 22, 16, T["accent"]))
        g.append(txt(px + 34, py + 20, num, 15, T["ink"], 500))
        g.append(txt(px + 34, py + 35, lbl, 11, T["ink2"]))
    g.append(rr(W - 54, py, 40, 44, 11, T["accent"]))
    g.append(icon("refresh", W - 34, py + 22, 18, "#ffffff"))
    # section label
    g.append(txt(16, py + 66, "3 CHANGED FILES", 11, T["ink2"], 500))
    # files
    rows = [("src/menu-bar/panel.tsx", "+42", ADD, True),
            ("src/themes/liquid-glass.css", "+118", ADD, True),
            ("docs/architecture.md", "-7", DEL, False)]
    fy = py + 82
    for path, delta, col, staged in rows:
        g.append(f'<g transform="translate(0,{fy})">')
        if staged:
            g.append(rr(16, 0, 16, 16, 5, T["accent"]))
            g.append(icon("check", 24, 8, 11, "#ffffff"))
        else:
            g.append(rr(16.75, 0.75, 14.5, 14.5, 4.5, "none", T["ink2"], 1.5))
        g.append(txt(41, 12, path, 12, T["ink"], mono=True))
        g.append(txt(W - 16, 12, delta, 11, col, 500, anchor="end"))
        g.append('</g>')
        fy += 30
    # commit box
    cy = fy + 8
    g.append(rr(14, cy, W - 28, 44, 12, T["field"], T["stroke"], 0.5))
    g.append(txt(26, cy + 19, "Add liquid-glass theme engine", 13, T["ink"], 500))
    g.append(txt(26, cy + 34, "Summary", 12, T["ink2"]))
    # commit button
    by = cy + 54
    g.append(rr(14, by, W - 28, 38, 12, T["accent"]))
    g.append(f'<rect x="14.5" y="{by+0.5}" width="{W-29}" height="1" fill="rgba(255,255,255,0.35)"/>')
    g.append(txt(W / 2, by + 24, "Commit to main", 13, "#ffffff", 500, anchor="middle"))
    # theme divider + swatches
    ty = by + 52
    g.append(f'<line x1="14" y1="{ty-10}" x2="{W-14}" y2="{ty-10}" stroke="{T["tint2"]}" stroke-width="1"/>')
    g.append(txt(16, ty + 4, "THEME", 11, T["ink2"], 500))
    sx = 16
    for name in ["aurora", "midnight", "sunset", "forest", "graphite"]:
        sw = THEMES[name]["bg"][0] if name != "midnight" else "#1f2333"
        ring = f' stroke="{T["accent"]}" stroke-width="2"' if name == t else ' stroke="rgba(0,0,0,0.12)" stroke-width="1"'
        g.append(f'<rect x="{sx}" y="{ty+14}" width="28" height="28" rx="9" fill="{sw}"{ring}/>')
        sx += 37
    g.append(txt(16, ty + 66, f'{T["label"]} - liquid glass', 12, T["ink2"]))
    g.append('</g>')
    return "\n".join(g), H

def desktop_bg(w, h, stops, idp):
    a, b, c = stops
    return (f'<defs><linearGradient id="{idp}" x1="0" y1="0" x2="1" y2="1">'
            f'<stop offset="0" stop-color="{a}"/><stop offset="0.55" stop-color="{b}"/>'
            f'<stop offset="1" stop-color="{c}"/></linearGradient>'
            f'<filter id="sh" x="-40%" y="-40%" width="180%" height="180%">'
            f'<feDropShadow dx="0" dy="20" stdDeviation="26" flood-color="#141428" flood-opacity="0.38"/></filter></defs>'
            f'<rect width="{w}" height="{h}" fill="url(#{idp})"/>'
            f'<rect width="{w}" height="{h}" fill="url(#{idp})"/>')

def hero():
    W, H = 1200, 760
    p, ph = panel("aurora")
    scale = 1.28
    px = W - 340 * scale - 90
    py = (H - ph * scale) / 2
    body = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">']
    body.append(desktop_bg(W, H, THEMES["aurora"]["bg"], "g0"))
    body.append('<circle cx="250" cy="200" r="320" fill="rgba(255,255,255,0.18)"/>')
    body.append(txt(96, 300, "Glint", 92, "#ffffff", 500))
    body.append(txt(100, 356, "A glint of Git in your menu bar.", 26, "rgba(255,255,255,0.92)"))
    body.append(txt(100, 396, "Ultralightweight · liquid glass · themeable", 18, "rgba(255,255,255,0.82)"))
    body.append(f'<g filter="url(#sh)" transform="translate({px},{py}) scale({scale})">{p}</g>')
    body.append('</svg>')
    return "\n".join(body)

def themes_strip():
    names = ["aurora", "midnight", "sunset", "forest", "graphite"]
    pad, gap = 40, 28
    scale = 0.62
    pw = 340 * scale
    _, ph = panel("aurora")
    W = int(pad * 2 + len(names) * pw + (len(names) - 1) * gap)
    H = int(pad * 2 + ph * scale + 44)
    body = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">']
    body.append(f'<rect width="{W}" height="{H}" rx="0" fill="#0e1017"/>')
    body.append('<defs><filter id="sh2" x="-40%" y="-40%" width="180%" height="180%"><feDropShadow dx="0" dy="12" stdDeviation="16" flood-color="#000000" flood-opacity="0.45"/></filter></defs>')
    x = pad
    for n in names:
        stops = THEMES[n]["bg"]
        gid = f"bg_{n}"
        body.append(f'<defs><linearGradient id="{gid}" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="{stops[0]}"/><stop offset="0.55" stop-color="{stops[1]}"/><stop offset="1" stop-color="{stops[2]}"/></linearGradient></defs>')
        body.append(f'<g transform="translate({x},{pad})">')
        body.append(f'<rect x="-14" y="-14" width="{pw+28}" height="{ph*scale+28}" rx="22" fill="url(#{gid})"/>')
        p, _ = panel(n)
        body.append(f'<g filter="url(#sh2)" transform="scale({scale})">{p}</g>')
        body.append(txt(pw / 2, ph * scale + 30, THEMES[n]["label"], 15, "#ffffff", 500, anchor="middle"))
        body.append('</g>')
        x += pw + gap
    body.append('</svg>')
    return "\n".join(body)

os.chdir(os.path.dirname(os.path.abspath(__file__)))
open("hero.svg", "w").write(hero())
open("themes.svg", "w").write(themes_strip())
for name in ["hero", "themes"]:
    subprocess.run(["rsvg-convert", "-z", "2", f"{name}.svg", "-o", f"{name}.png"], check=True)
    print(name, "->", os.path.getsize(f"{name}.png"), "bytes")
