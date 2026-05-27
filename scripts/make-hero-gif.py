#!/usr/bin/env python3
"""Generate animated hero GIF for opendocswork-mcp README using rendered output previews."""

from PIL import Image, ImageDraw, ImageFont
import os

W, H = 900, 500
FPS_MS = 600
BASE = os.path.dirname(os.path.dirname(__file__))
OUTPUT = os.path.join(BASE, "showcase", "hero.gif")
PREVIEWS = os.path.join(BASE, "showcase", "previews")

# Color palette
BG1 = (13, 17, 23)
BG2 = (22, 27, 34)
TEXT = (226, 232, 240)
MUTED = (148, 163, 184)
DARK_LINE = (30, 41, 59)
ACCENTS = {
    "XLSX": (63, 185, 80),
    "DOCX": (88, 166, 255),
    "PPTX": (245, 158, 11),
    "PDF": (168, 130, 255),
}
DEFAULT_ACCENT = (88, 166, 255)

FONT_DIR = "/usr/share/fonts/truetype/dejavu"

def font(size, bold=False):
    return ImageFont.truetype(f"{FONT_DIR}/DejaVuSans-Bold.ttf" if bold else f"{FONT_DIR}/DejaVuSans.ttf", size)

def mono_font(size):
    return ImageFont.truetype(f"{FONT_DIR}/DejaVuSansMono.ttf", size)

def gradient_bg(draw):
    for y in range(H):
        ratio = y / H
        r = int(BG1[0] * (1 - ratio) + BG2[0] * ratio)
        g = int(BG1[1] * (1 - ratio) + BG2[1] * ratio)
        b = int(BG1[2] * (1 - ratio) + BG2[2] * ratio)
        draw.line([(0, y), (W, y)], fill=(r, g, b))

def draw_accent_glow(draw, color, cx=W-150, cy=80):
    for r in range(200, 0, -1):
        alpha = max(0, int(15 - r * 15 / 200))
        if alpha <= 0: continue
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(*color[:3], alpha))

def render_preview_frame(title, fmt, preview_path):
    color = ACCENTS.get(fmt, DEFAULT_ACCENT)
    img = Image.new("RGBA", (W, H))
    draw = ImageDraw.Draw(img)
    gradient_bg(draw)
    draw_accent_glow(draw, color)

    # Thin accent bar at top
    draw.rounded_rectangle([40, 40, W - 40, 44], 2, fill=color)

    # Brand top-left
    draw.text((50, 60), "opendocswork-mcp   ·   showcase", fill=MUTED, font=font(16, bold=True))

    # Format badge top-right
    tag_w = font(13, bold=True).getbbox(fmt)[2] + 24
    draw.rounded_rectangle([W - 50 - tag_w, 58, W - 50, 58 + 28], 14, fill=(*color[:3], 40))
    draw.text((W - 50 - tag_w + 12, 63), fmt, fill=color, font=font(13, bold=True))

    # Load preview image
    try:
        preview = Image.open(preview_path).convert("RGBA")
    except Exception:
        preview = None

    if preview:
        # Fit preview into the frame - center, max ~500x320
        pw, ph = preview.size
        max_pw, max_ph = 580, 340
        scale = min(max_pw / pw, max_ph / ph, 1.0)
        new_w, new_h = int(pw * scale), int(ph * scale)
        preview = preview.resize((new_w, new_h), Image.LANCZOS)
        px = (W - new_w) // 2
        py = (H - new_h) // 2 + 20
        # Soft shadow behind preview
        for i in range(5, 0, -1):
            alpha = 8 - i
            shadow = Image.new("RGBA", (new_w + i*8, new_h + i*8), (0, 0, 0, alpha))
            img.paste(shadow, (px - i*4, py - i*4), shadow)
        img.paste(preview, (px, py), preview)

    # Title overlay at bottom
    title_bg_h = 70
    for y in range(title_bg_h):
        alpha = int(180 * (1 - y / title_bg_h))
        draw.line([(0, H - y - 50), (W, H - y - 50)], fill=(0, 0, 0, alpha))
    title_font = font(28, bold=True)
    tw = draw.textbbox((0, 0), title, font=title_font)
    draw.text(((W - tw[2]) // 2, H - 50 - (title_bg_h - tw[3]) // 2 - 8), title, fill=TEXT, font=title_font)

    # Footer
    foot_font = font(12)
    draw.text((50, H - 22), "Rust-native MCP · OOXML · 6 formats · 50+ tools", fill=MUTED, font=foot_font)
    draw.text((W - 50, H - 22), "github.com/Aimino-Tech/opendocswork-mcp", fill=MUTED, font=foot_font, anchor="rt")

    return img

# Slides: (title, format, preview_filename)
slides = [
    ("Profit & Loss Statement",          "XLSX", "01-pnl.png"),
    ("Executive KPI Dashboard",          "XLSX", "02-kpi.png"),
    ("Budget vs Actual Variance",        "XLSX", "03-budget.png"),
    ("Balance Sheet with Ratios",        "XLSX", "04-balance.png"),
    ("Invoice Generator",                "DOCX", "05-invoice.png"),
    ("Financial Report Export",          "PDF",  "06-pdfexport.png"),
    ("Revenue Forecast Model",           "XLSX", "10-forecast.png"),
    ("Cost Analysis Dashboard",          "XLSX", "11-cost.png"),
    ("Annual Business Report",           "DOCX", "15-report.png"),
    ("Digital Strategy Report",          "DOCX", "16-strategy.png"),
    ("IT Service Agreement",             "DOCX", "17-contract.png"),
    ("Strategy Consulting Pitch Deck",   "PPTX", "01-strategy-consulting-pitch.png"),
    ("CFO Quarterly Business Review",    "PPTX", "02-cfo-qbr-review.png"),
    ("Product Launch Strategy Deck",     "PPTX", "03-product-launch-strategy.png"),
    ("M&A Target Analysis Deck",         "PPTX", "04-ma-target-analysis.png"),
    ("Digital Transformation Roadmap",   "PPTX", "05-digital-transformation.png"),
]

frames = []

# Intro frame
intro = Image.new("RGBA", (W, H))
intro_draw = ImageDraw.Draw(intro)
gradient_bg(intro_draw)
for r in range(250, 0, -1):
    alpha = max(0, int(20 - r * 20 / 250))
    if alpha <= 0: continue
    intro_draw.ellipse([W//2 - r + 100, H//2 - r - 80, W//2 + r + 100, H//2 + r - 80],
                       fill=(*DEFAULT_ACCENT[:3], alpha))

intro_draw.text((W//2, 160), "opendocswork-mcp", fill=TEXT, font=font(52, bold=True), anchor="mt")
intro_draw.text((W//2, 225), "AI-native Office Document Engine", fill=MUTED, font=font(24), anchor="mt")
intro_draw.text((W//2, 265), "Excel  ·  Word  ·  PowerPoint  ·  PDF", fill=MUTED, font=font(18), anchor="mt")
badges = ["100-500x Faster", "50+ Tools", "6 Formats", "Open Source"]
for i, badge in enumerate(badges):
    bw = font(16, bold=True).getbbox(badge)[2] + 28
    bx = W//2 - 190 + i * 127
    intro_draw.rounded_rectangle([bx - bw//2, 310, bx + bw//2, 310 + 34], 17, fill=(*DEFAULT_ACCENT[:3], 60))
    intro_draw.text((bx, 318), badge, fill=DEFAULT_ACCENT, font=font(16, bold=True), anchor="mt")
cargo_text = "$ cargo install opendocswork-mcp"
cf = mono_font(16)
cw = cf.getbbox(cargo_text)[2] + 32
intro_draw.rounded_rectangle([W//2 - cw//2, 370, W//2 + cw//2, 370 + 38], 8, fill=(22, 27, 34))
intro_draw.text((W//2, 382), cargo_text, fill=(63, 185, 80), font=cf, anchor="mt")
frames.append(intro)

# Slide frames with preview images
for title, fmt, preview_file in slides:
    path = os.path.join(PREVIEWS, preview_file)
    frames.append(render_preview_frame(title, fmt, path))

# Outro frame
outro = Image.new("RGBA", (W, H))
outro_draw = ImageDraw.Draw(outro)
gradient_bg(outro_draw)
draw_accent_glow(outro_draw, DEFAULT_ACCENT, cx=W//2, cy=H//2)

outro_draw.rounded_rectangle([40, 40, W - 40, 44], 2, fill=DEFAULT_ACCENT)
outro_draw.text((50, 60), "opendocswork-mcp", fill=MUTED, font=font(18, bold=True))

outro_draw.text((W//2, 180), "Ready to Build?", fill=TEXT, font=font(44, bold=True), anchor="mt")
outro_draw.text((W//2, 235), "Install in 30 seconds", fill=MUTED, font=font(22), anchor="mt")

install = "$ cargo install opendocswork-mcp"
cf = mono_font(18)
inst_w = cf.getbbox(install)[2] + 32
outro_draw.rounded_rectangle([W//2 - inst_w//2, 275, W//2 + inst_w//2, 275 + 42], 8, fill=(22, 27, 34))
outro_draw.text((W//2, 289), install, fill=(63, 185, 80), font=cf, anchor="mt")

links = ["github.com/Aimino-Tech/opendocswork-mcp", "crates.io/crates/opendocswork-mcp"]
for i, link in enumerate(links):
    outro_draw.text((W//2, 350 + i * 32), link, fill=DEFAULT_ACCENT, font=font(16), anchor="mt")

outro_draw.text((50, H - 45), "Rust-native MCP · OOXML · 6 formats · 50+ tools", fill=MUTED, font=font(13))
outro_draw.text((W - 50, H - 45), "github.com/Aimino-Tech/opendocswork-mcp", fill=MUTED, font=font(13), anchor="rt")
frames.append(outro)

# Save GIF
durations = [FPS_MS] * len(frames)
durations[0] = 1200
durations[-1] = 1200

frames[0].save(
    OUTPUT,
    save_all=True,
    append_images=frames[1:],
    duration=durations,
    loop=0,
    optimize=True,
    disposal=2,
)

print(f"hero.gif created: {os.path.getsize(OUTPUT) / 1024:.0f} KB, {len(frames)} frames")
for i, (title, fmt, _) in enumerate(slides):
    print(f"  {i+1:2d}. [{fmt:4s}] {title}")
