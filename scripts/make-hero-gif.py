#!/usr/bin/env python3
"""Generate animated hero GIF for opendocswork-mcp README."""

from PIL import Image, ImageDraw, ImageFont
import os, textwrap

W, H = 900, 500
FPS_MS = 600
OUTPUT = os.path.join(os.path.dirname(__file__), "..", "showcase", "hero.gif")

# ── Color palette ──
BG1 = (13, 17, 23)       # #0d1117
BG2 = (22, 27, 34)       # #1b1b23
ACCENT_BLUE = (88, 166, 255)  # #58a6ff
ACCENT_AMBER = (245, 158, 11) # #f59e0b
ACCENT_TEAL = (20, 184, 166)  # #14b8a6
ACCENT_PURPLE = (168, 130, 255)
ACCENT_GREEN = (63, 185, 80)
TEXT = (226, 232, 240)   # #e2e8f0
MUTED = (148, 163, 184)  # #94a3b8
DARK_LINE = (30, 41, 59) # #1e293b

FONT_DIR = "/usr/share/fonts/truetype"

def get_font(size, bold=False):
    name = "DejaVuSans-Bold" if bold else "DejaVuSans"
    return ImageFont.truetype(f"{FONT_DIR}/{name}.ttf", size)

def get_mono_font(size):
    return ImageFont.truetype(f"{FONT_DIR}/DejaVuSansMono.ttf", size)

# ── Slides data ──
tools = [
    ("get_document_info", "Read", "Inspect metadata of DOCX, XLSX, PPTX files", ACCENT_BLUE,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "get_document_info",\n    "arguments": {\n      "file_path": "report.xlsx"\n    }\n  }\n}'),
    ("office_read", "Read", "Read any Office document → JSON, Markdown, or Chunks", ACCENT_TEAL,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "office_read",\n    "arguments": {\n      "file_path": "report.xlsx",\n      "output_format": "markdown"\n    }\n  }\n}'),
    ("skill_run · invoice", "Skill", "Generate professional invoices from {company, amount, items}", ACCENT_AMBER,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "skill_run",\n    "arguments": {\n      "skill": "word.invoice",\n      "company": "Acme Corp",\n      "amount": 12490\n    }\n  }\n}'),
    ("skill_run · excel", "Skill", "Create styled Excel reports, KPIs, budgets, forecasts", ACCENT_GREEN,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "skill_run",\n    "arguments": {\n      "skill": "excel.basic",\n      "title": "Q4 KPI Dashboard"\n    }\n  }\n}'),
    ("skill_run · ppt", "Skill", "Build pitch decks, QBRs, strategy presentations", ACCENT_PURPLE,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "skill_run",\n    "arguments": {\n      "skill": "ppt.deck",\n      "title": "Strategic Review 2026"\n    }\n  }\n}'),
    ("propagate_edit", "Coherence", "Edit an entity value and cascade to all dependents", ACCENT_TEAL,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "propagate_edit",\n    "arguments": {\n      "entity_id": "company_name",\n      "new_value": "Acme Corp"\n    }\n  }\n}'),
    ("validate", "Validate", "Deep structural validation of DOCX, XLSX, PPTX files", ACCENT_BLUE,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "validate",\n    "arguments": {\n      "file_path": "invoice.docx",\n      "checks": ["structure", "formatting"]\n    }\n  }\n}'),
    ("list_formats", "Read", "All 6 supported Office formats + capabilities", ACCENT_GREEN,
     '{\n  "method": "tools/call",\n  "params": {\n    "name": "list_formats"\n  }\n}'),
]

use_cases = [
    ("Profit & Loss Statement", "XLSX", ACCENT_BLUE),
    ("Executive KPI Dashboard", "XLSX", ACCENT_GREEN),
    ("Budget vs Actual Variance", "XLSX", ACCENT_AMBER),
    ("Balance Sheet with Ratios", "XLSX", ACCENT_TEAL),
    ("Invoice", "DOCX", ACCENT_AMBER),
    ("Financial Report Export", "PDF", ACCENT_PURPLE),
    ("Revenue Forecast", "XLSX", ACCENT_BLUE),
    ("Cost Analysis", "XLSX", ACCENT_GREEN),
    ("Annual Business Report", "DOCX", ACCENT_TEAL),
    ("Digital Strategy Report", "DOCX", ACCENT_PURPLE),
    ("IT Service Agreement", "DOCX", ACCENT_BLUE),
    ("Strategy Consulting Pitch Deck", "PPTX", ACCENT_AMBER),
    ("CFO Quarterly Business Review", "PPTX", ACCENT_GREEN),
    ("Product Launch Strategy Deck", "PPTX", ACCENT_TEAL),
    ("M&A Target Analysis Deck", "PPTX", ACCENT_PURPLE),
    ("Digital Transformation Roadmap", "PPTX", ACCENT_BLUE),
]

def draw_rounded_rect(draw, xy, radius, fill):
    x1, y1, x2, y2 = xy
    draw.rounded_rectangle(xy, radius, fill=fill)

def draw_code_block(draw, text, x, y, w, font):
    lines = text.split('\n')
    code_bg = (22, 27, 34)
    padding = 14
    line_h = font.getbbox("Ay")[3] + 6
    block_h = len(lines) * line_h + padding * 2
    draw.rounded_rectangle([x, y, x + w, y + block_h], 6, fill=code_bg)
    for i, line in enumerate(lines):
        ly = y + padding + i * line_h
        is_key = line.strip().startswith(('"method"', '"name"', '"arguments"', '"params"', '"skills"'))
        color = (198, 120, 221) if is_key else (152, 195, 121)
        draw.text((x + padding, ly), line, fill=color, font=font)

def render_frame(title, category, desc, color, code=None, is_use_case=False):
    img = Image.new("RGBA", (W, H))
    draw = ImageDraw.Draw(img)

    # Background with subtle gradient
    for y in range(H):
        ratio = y / H
        r = int(BG1[0] * (1 - ratio) + BG2[0] * ratio)
        g = int(BG1[1] * (1 - ratio) + BG2[1] * ratio)
        b = int(BG1[2] * (1 - ratio) + BG2[2] * ratio)
        draw.line([(0, y), (W, y)], fill=(r, g, b))

    # Accent glow top-right
    for r in range(200, 0, -1):
        alpha = max(0, int(15 - r * 15 / 200))
        if alpha <= 0: continue
        draw.ellipse([W - r - 100, -r + 50, W - r + 300, -r + 250], fill=(*color[:3], alpha))

    # Accent bar top
    draw.rounded_rectangle([40, 40, W - 40, 44], 2, fill=color)

    # Logo / brand top-left
    brand_font = get_font(18, bold=True)
    draw.text((50, 60), "opendocswork-mcp", fill=MUTED, font=brand_font)

    # Category badge
    if category:
        tag_font = get_font(14, bold=True)
        tw = tag_font.getbbox(category)[2] + 24
        draw.rounded_rectangle([W - 50 - tw, 58, W - 50, 58 + 28], 14, fill=(*color[:3], 40))
        draw.text((W - 50 - tw + 12, 63), category, fill=color, font=tag_font)

    if is_use_case:
        # Use case card
        title_font = get_font(42, bold=True)
        sub_font = get_font(22)

        tw = draw.textbbox((0, 0), title, font=title_font)
        cx = (W - tw[2]) // 2
        cy = 190
        draw.text((cx, cy), title, fill=TEXT, font=title_font)

        # Format badge
        fmt_font = get_font(20, bold=True)
        fw = fmt_font.getbbox(category)[2] + 32
        draw.rounded_rectangle([cx, cy + 70, cx + fw, cy + 70 + 42], 21, fill=(*color[:3], 200))
        draw.text((cx + 16, cy + 75), category, fill=(0, 0, 0), font=fmt_font)

        # Doc icon
        icon_font = get_font(60)
        # Draw a document icon using shapes
        icon_x, icon_y = W // 2 - 25, cy - 130
        draw.rectangle([icon_x, icon_y, icon_x + 50, icon_y + 60], fill=None, outline=color, width=3)
        draw.line([icon_x + 15, icon_y + 20, icon_x + 35, icon_y + 20], fill=color, width=3)
        draw.line([icon_x + 15, icon_y + 32, icon_x + 35, icon_y + 32], fill=color, width=3)
        draw.line([icon_x + 15, icon_y + 44, icon_x + 30, icon_y + 44], fill=color, width=3)
    else:
        # Tool slide
        title_font = get_font(46, bold=True)
        desc_font = get_font(20)
        code_font = get_mono_font(15)

        tw = draw.textbbox((0, 0), title, font=title_font)
        cx = (W - tw[2]) // 2
        draw.text((cx, 150), title, fill=TEXT, font=title_font)

        draw.text((60, 215), desc, fill=MUTED, font=desc_font)

        # Code block
        if code:
            code_x, code_y = 60, 265
            code_w = W - 120
            draw_code_block(draw, code, code_x, code_y, code_w, code_font)

    # Footer
    foot_font = get_font(13)
    draw.text((50, H - 45), "Rust-native · 100-500x faster than Python · Open source", fill=MUTED, font=foot_font)
    draw.text((W - 50, H - 45), "github.com/Aimino-Tech/opendocswork-mcp", fill=MUTED, font=foot_font,
              anchor="rt")

    return img

# ── Build frames ──
frames = []

# Intro frame
img = Image.new("RGBA", (W, H))
draw = ImageDraw.Draw(img)
for y in range(H):
    ratio = y / H
    r = int(BG1[0] * (1 - ratio) + BG2[0] * ratio)
    g = int(BG1[1] * (1 - ratio) + BG2[1] * ratio)
    b = int(BG1[2] * (1 - ratio) + BG2[2] * ratio)
    draw.line([(0, y), (W, y)], fill=(r, g, b))
for r in range(250, 0, -1):
    alpha = max(0, int(20 - r * 20 / 250))
    if alpha <= 0: continue
    draw.ellipse([W//2 - r + 100, H//2 - r - 80, W//2 + r + 100, H//2 + r - 80], fill=(*ACCENT_BLUE[:3], alpha))

intro_font = get_font(52, bold=True)
sub_font = get_font(24)
draw.text((W//2, 180), "opendocswork-mcp", fill=TEXT, font=intro_font, anchor="mt")
draw.text((W//2, 245), "✨ AI-native Office Document Engine ✨", fill=MUTED, font=sub_font, anchor="mt")
detail_font = get_font(18)
for i, line in enumerate(["Excel · Word · PowerPoint", "Sub-millisecond · Local-first · Open Source"]):
    draw.text((W//2, 310 + i * 35), line, fill=MUTED, font=detail_font, anchor="mt")
badge_font = get_font(16, bold=True)
for i, badge in enumerate(["100-500x Faster", "50+ Tools", "6 Formats"]):
    bw = badge_font.getbbox(badge)[2] + 28
    bx = W//2 - 170 + i * 170
    draw.rounded_rectangle([bx - bw//2, 380, bx + bw//2, 380 + 34], 17, fill=(*ACCENT_TEAL[:3], 60))
    draw.text((bx, 388), badge, fill=ACCENT_TEAL, font=badge_font, anchor="mt")
frames.append(img)

# Tool frames
for title, cat, desc, color, code in tools:
    frames.append(render_frame(title, cat, desc, color, code=code))

# Use case frames
for title, fmt, color in use_cases:
    frames.append(render_frame(title, fmt, "", color, is_use_case=True))

# Outro frame
img = Image.new("RGBA", (W, H))
draw = ImageDraw.Draw(img)
for y in range(H):
    ratio = y / H
    r = int(BG1[0] * (1 - ratio) + BG2[0] * ratio)
    g = int(BG1[1] * (1 - ratio) + BG2[1] * ratio)
    b = int(BG1[2] * (1 - ratio) + BG2[2] * ratio)
    draw.line([(0, y), (W, y)], fill=(r, g, b))

draw.rounded_rectangle([40, 40, W - 40, 44], 2, fill=ACCENT_BLUE)
brand_font = get_font(18, bold=True)
draw.text((50, 60), "opendocswork-mcp", fill=MUTED, font=brand_font)

outro_font = get_font(44, bold=True)
draw.text((W//2, 180), "Ready to Build?", fill=TEXT, font=outro_font, anchor="mt")
sub_font2 = get_font(22)
draw.text((W//2, 240), "Install in 30 seconds", fill=MUTED, font=sub_font2, anchor="mt")

# Terminal-style install box
install = "$ cargo install opendocswork-mcp"
code_font = get_mono_font(18)
ib = code_font.getbbox(install)
box_x, box_y = W//2 - 200, 280
box_w, box_h = 400, ib[3] + 30
draw.rounded_rectangle([box_x, box_y, box_x + box_w, box_y + box_h], 8, fill=(22, 27, 34))
draw.text((box_x + 15, box_y + 10), install, fill=ACCENT_GREEN, font=code_font)

links = ["github.com/Aimino-Tech/opendocswork-mcp", "crates.io · crates.io/crates/opendocswork-mcp"]
link_font = get_font(16)
for i, link in enumerate(links):
    draw.text((W//2, 360 + i * 32), link, fill=ACCENT_BLUE, font=link_font, anchor="mt")

foot_font = get_font(13)
draw.text((50, H - 45), "Rust-native · 100-500x faster than Python · Open source", fill=MUTED, font=foot_font)
frames.append(img)

# ── Save as GIF ──
duration = [FPS_MS] * len(frames)
# Show intro/outro slightly longer
duration[0] = 1200
duration[-1] = 1200

frames[0].save(
    OUTPUT,
    save_all=True,
    append_images=frames[1:],
    duration=duration,
    loop=0,
    optimize=True,
    disposal=2,
)

print(f"✅ hero.gif created: {os.path.getsize(OUTPUT) / 1024:.0f} KB, {len(frames)} frames")
