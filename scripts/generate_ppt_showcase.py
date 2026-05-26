#!/usr/bin/env python3
"""Generate McKinsey-quality PowerPoint presentations using the MCP server.
Creates showcase decks for the use-cases.html gallery."""
import json, subprocess, os, shutil, time

REPO = os.path.normpath(os.path.join(os.path.dirname(__file__), ".."))
BIN = os.path.join(REPO, "target", "debug", "office-oxide-mcp")
OUT = os.path.join(REPO, "showcase", "generated")
os.makedirs(OUT, exist_ok=True)

class MCP:
    def __init__(self):
        self.proc = subprocess.Popen([BIN, "--transport", "stdio"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, cwd=REPO)
        self.mid = 0
        self._call("initialize", {"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"ppt-gen","version":"1"}})
        self._notify("notifications/initialized", {})
    def _next(self):
        self.mid += 1
        return self.mid
    def _write(self, m):
        self.proc.stdin.write((json.dumps(m)+"\n").encode())
        self.proc.stdin.flush()
    def _read(self):
        return json.loads(self.proc.stdout.readline().decode())
    def _call(self, method, params=None):
        m={"jsonrpc":"2.0","id":self._next(),"method":method}
        if params: m["params"]=params
        self._write(m)
        while True:
            r=self._read()
            if "id" in r and r["id"]==m["id"]:
                if "error" in r: raise RuntimeError(str(r["error"]))
                return r.get("result",{})
    def _notify(self, method, params=None):
        self._write({"jsonrpc":"2.0","method":method,"params":params or {}})
    def call(self, name, args=None):
        r=self._call("tools/call",{"name":name,"arguments":args or {}})
        t="".join(c.get("text","") for c in r.get("content",[]) if c.get("type")=="text")
        try: return json.loads(t)
        except: return t

mcp = MCP()
print("[MCP] Connected")

def save_ppt(slug, result=None):
    """Copy the generated PPTX from output/ to showcase/generated/<slug>/"""
    d = os.path.join(OUT, slug)
    os.makedirs(d, exist_ok=True)
    # Use result file_paths if available, otherwise scan output/
    src_path = None
    if result and isinstance(result, dict) and result.get("file_paths"):
        for fp in result["file_paths"]:
            fp_abs = os.path.join(REPO, fp) if not os.path.isabs(fp) else fp
            if os.path.exists(fp_abs) and fp.endswith(".pptx"):
                src_path = fp_abs
                break
    if not src_path:
        out_dir = os.path.join(REPO, "output")
        if os.path.isdir(out_dir):
            files = sorted([f for f in os.listdir(out_dir) if f.endswith(".pptx")],
                          key=lambda f: os.path.getmtime(os.path.join(out_dir, f)), reverse=True)
            if files:
                src_path = os.path.join(out_dir, files[0])
    if not src_path:
        print(f"  [WARN] No pptx files found for {slug}")
        return
    clean = f"{slug}.pptx"
    shutil.copy2(src_path, os.path.join(d, clean))
    shutil.copy2(src_path, os.path.join(d, "output.pptx"))
    print(f"  [FILE] {slug}/{clean} ({os.path.getsize(os.path.join(d, clean))} bytes)")
    # Clean output dir
    out_dir = os.path.join(REPO, "output")
    if os.path.isdir(out_dir):
        for f in os.listdir(out_dir):
            if f.endswith(".pptx"):
                os.remove(os.path.join(out_dir, f))

# ═══════════════════════════════════════════════════════════════════
# DECK 1: McKinsey-Strategy Consulting Pitch
# ═══════════════════════════════════════════════════════════════════
print("\n=== DECK 1: Strategy Consulting Pitch ===")
r = mcp.call("run_skill", {"skill_name": "ppt.deck", "params": {
    "title": "Global Growth Strategy — Acme Corp",
    "subtitle": "Confidential — Board Presentation",
    "theme": "professional",
    "include_slide_numbers": True,
    "slides": [
        {"type": "title", "title": "Global Growth Strategy", "subtitle": "Acme Corp — FY 2026–2028 Strategic Plan\nPrepared for the Board of Directors\nMay 2026",
         "content": ""},
        {"type": "agenda", "title": "Agenda", "content": "• Executive Summary\n• Market Landscape & Opportunity\n• Strategic Pillars\n• Financial Projections\n• Implementation Roadmap\n• Risk Assessment\n• Appendix"},
        {"type": "content", "title": "Executive Summary", "callout": "Acme Corp delivered $39.1M revenue in FY 2025 — 28% YoY growth — and is on track to reach $100M ARR by FY 2028 through three strategic pillars.", "content": "• Market position: Global document processing market projected to reach $45B by 2028\n• Growth strategy: Geographic expansion, product verticalization, platform ecosystem\n• Investment: $4.5M strategic investment expected to yield $12-15M incremental revenue within 18 months\n• Margins: Operating margins projected to improve from 22% to 40% over the planning horizon"},
        {"type": "section", "title": "Market Landscape & Opportunity", "content": ""},
        {"type": "content", "title": "Global Market Context", "content": "The document processing market is undergoing a structural shift.\n\n• Global market projected to reach $45B by 2028 (CAGR 12.4%)\n• Key growth drivers: digital transformation, regulatory compliance, AI adoption, hybrid work\n• Legacy vendors (Aspose, Adobe) slow to innovate with cloud-native solutions\n• Open-source alternatives require significant engineering investment\n• Acme Corp's TAM: $8.5B in enterprise AI document processing"},
        {"type": "content", "title": "Competitive Positioning", "content": "• Speed advantage: 100-500x faster than Python-based solutions\n• Local-first architecture: zero dependency on cloud infrastructure\n• Open-source licensing: removes procurement barriers\n• 4MB binary vs 150MB+ for Python-based alternatives\n• Sub-50ms cold start vs 1-5s for Python VM\n• Cross-platform: Linux, macOS, Windows — single binary"},
        {"type": "section", "title": "Strategic Pillars", "content": ""},
        {"type": "content", "title": "Pillar 1: Geographic Expansion", "content": "• North America: 62% of FY 2025 revenue — maintain and deepen\n• Europe: 23% growth driven by GDPR compliance requirements\n• APAC: Highest growth trajectory at 47% YoY — prioritize investment\n• LATAM & MEA: Untapped $1.2B addressable market\n• Entry strategy: phased rollout starting with Germany and Japan by Q2 2026"},
        {"type": "content", "title": "Pillar 2: Product Verticalization", "content": "• Compliance automation suite: targeting financial services and healthcare\n• Enterprise SDK: embeddable document engine for ISVs\n• Skills marketplace: 50 community-contributed skills by Q4 2026\n• AI document intelligence: classification, extraction, generation\n• Each vertical represents $50-200M incremental TAM"},
        {"type": "content", "title": "Pillar 3: Platform Ecosystem", "content": "• Partner network: 12 regional system integrators and 3 cloud platform providers\n• Developer community: docs-first approach, contribution framework\n• MCP directory listings: 20+ directories for distribution\n• Viral loops: open-source → community → enterprise adoption\n• Marketplace: skills, templates, integrations ecosystem"},
        {"type": "section", "title": "Financial Projections", "content": ""},
        {"type": "content", "title": "Revenue Model & Projections", "callout": "Path to $100M ARR: FY 2026E $48-52M → FY 2027E $65-72M → FY 2028E $90-105M at 25-30% CAGR", "content": "• Gross margin improvement: 60% → 65% through operational efficiencies\n• Operating expenses: $14-16M including $4.5M strategic investment\n• Net income target: $12-15M in FY 2026\n• Operating margins: 26-28% in FY 2026, expanding to 40% by FY 2028\n• Cash position: Expected to remain above $10M throughout planning horizon"},
        {"type": "content", "title": "Investment Requirements", "content": "• Geographic expansion: $1.8M (sales teams, local infrastructure)\n• Product development: $1.2M (compliance suite, SDK, AI features)\n• Ecosystem & community: $0.8M (developer relations, marketplace)\n• Marketing & sales: $0.7M (demand generation, content, events)\n• Total strategic investment: $4.5M over 18 months\n• Expected ROI: $12-15M incremental revenue within 18 months"},
        {"type": "section", "title": "Implementation Roadmap", "content": ""},
        {"type": "content", "title": "Phase 1: Foundation (Q1–Q2 2026)", "content": "• Market research completion and partnership framework definition\n• Initial hires: Regional Sales Directors (3), Solutions Engineers (4)\n• German market entry preparation — local entity setup\n• Compliance module beta with 3 design partners\n• 5 system integrator partnerships secured\n• Community milestone: 50 GitHub stars, 10 skills"},
        {"type": "content", "title": "Phase 2: Acceleration (Q3–Q4 2026)", "content": "• Full compliance automation launch at industry conferences\n• Japanese market entry with local partner\n• Expand to 12 total system integrator partners\n• Skills marketplace launch with 30 community skills\n• Enterprise customer growth: 47 → 80\n• Revenue target: $13-15M in Q4 2026 alone"},
        {"type": "content", "title": "Phase 3: Scale (FY 2027–2028)", "content": "• 3 additional region entries (Brazil, UAE, Singapore)\n• AI document intelligence product launch\n• Partner-generated revenue target: $4M annually\n• Community: 200 GitHub stars, 50+ skills, 500+ developers\n• Enterprise customers: 120+ with $100K+ ACV\n• Path to profitability with 40% operating margins"},
        {"type": "section", "title": "Risk Assessment", "content": ""},
        {"type": "content", "title": "Key Risks & Mitigation", "content": "• Market entry execution: phased rollout with pilot programs and go/no-go gates\n• Talent acquisition: competitive compensation, remote-first policy, global hiring\n• Competitive response: speed-to-market advantage, open-source community moat\n• Currency and regulatory risk: local partnerships, compliance-first design\n• Technology risk: continuous R&D investment, performance benchmarking\n• Concentration risk: diversify revenue across regions and verticals"},
        {"type": "thank_you", "title": "Thank You", "subtitle": "Questions & Discussion\nstrategy@acmecorp.com", "content": ""},
    ]
}})
save_ppt("01-strategy-consulting-pitch", r)
print(f"  [OK] Strategy Consulting Pitch")

# ═══════════════════════════════════════════════════════════════════
# DECK 2: CFO Quarterly Business Review (QBR)
# ═══════════════════════════════════════════════════════════════════
print("\n=== DECK 2: CFO Quarterly Business Review ===")
r = mcp.call("run_skill", {"skill_name": "ppt.deck", "params": {
    "title": "Q4 FY 2025 Business Review",
    "subtitle": "Acme Corp — Financial Performance & Outlook",
    "theme": "professional",
    "include_slide_numbers": True,
    "slides": [
        {"type": "title", "title": "Q4 FY 2025 Business Review", "subtitle": "Acme Corp — Financial Performance & Outlook\nPrepared for the Board\nMay 2026",
         "content": ""},
        {"type": "content", "title": "Executive Summary", "content": "• Record revenue of $11.8M in Q4 2025 (+32% YoY)\n• Full-year revenue of $39.1M (+28% YoY), exceeding guidance\n• Gross margin expanded 230bps to 62.8%\n• Operating income of $3.1M (26.5% margin)\n• Free cash flow of $2.4M, up 200% YoY\n• Customer count grew to 210, with 52 enterprise clients"},
        {"type": "content", "title": "Revenue Performance", "callout": "Record $11.8M in Q4 2025 — all segments exceeded guidance.", "content": "• All segments exceeded Q4 guidance\n• New Customer ARR: $3.8M (+45% YoY)\n• Net Revenue Retention: 120% (+8pp vs prior year)", "chart": {"type": "horizontal_bars", "title": "Revenue by Segment ($M)", "items": [
            {"label": "Product Revenue", "value": 70, "color": "003A70"},
            {"label": "Service Revenue", "value": 20, "color": "5B9BD5"},
            {"label": "Licensing Revenue", "value": 10, "color": "70AD47"}
        ]}},
        {"type": "content", "title": "Profitability Analysis", "content": "• Gross Margin: 62.8% (+230bps YoY) — driven by scale efficiencies\n• Operating Margin: 26.5% (+450bps YoY)\n• Net Margin: 19.9% (+340bps YoY)\n• EBITDA Margin: 29.0% (+450bps YoY)\n• R&D investment: 10.5% of revenue (maintaining innovation pipeline)\n• S&M efficiency: Magic Number improved to 1.02x"},
        {"type": "content", "title": "Cash Flow & Balance Sheet", "content": "• Operating Cash Flow: $3.1M (+158% YoY)\n• Free Cash Flow: $2.4M (+200% YoY)\n• Cash & Equivalents: $8.2M (up from $5.8M in FY 2024)\n• Debt-free balance sheet with $8.2M cash and zero debt\n• Cash runway: 36 months at current burn rate\n• DSO improved to 38 days from 45 days"},
        {"type": "content", "title": "Customer Metrics", "content": "• Total Customers: 210 (up from 120, +75% YoY)\n• Enterprise Customers: 52 (up from 28, +86% YoY)\n• Average Contract Value: $105K (+24% YoY)\n• Customer Acquisition Cost: $10.5K (-16% YoY)\n• LTV/CAC Ratio: 27.1x (up from 18.0x)\n• Net Promoter Score: 72 (up from 58)"},
        {"type": "content", "title": "FY 2026 Guidance", "content": "• Revenue Guidance: $48-52M (25-30% growth)\n• Gross Margin: 63-65% (continued expansion)\n• Operating Margin: 26-28%\n• Strategic Investment: $4.5M in expansion initiatives\n• Enterprise Customers: target of 80-90\n• Cash position expected to remain above $10M"},
        {"type": "thank_you", "title": "Thank You", "subtitle": "CFO Office — Acme Corp\ninvestors@acmecorp.com", "content": ""},
    ]
}})
save_ppt("02-cfo-qbr-review", r)
print(f"  [OK] CFO Quarterly Business Review")

# ═══════════════════════════════════════════════════════════════════
# DECK 3: Product Launch Strategy
# ═══════════════════════════════════════════════════════════════════
print("\n=== DECK 3: Product Launch Strategy ===")
r = mcp.call("run_skill", {"skill_name": "ppt.deck", "params": {
    "title": "Compliance Automation Suite — Launch Plan",
    "subtitle": "Product Strategy & Go-to-Market",
    "theme": "professional",
    "include_slide_numbers": True,
    "slides": [
        {"type": "title", "title": "Compliance Automation Suite", "subtitle": "Product Strategy & Go-to-Market Plan\nConfidential — Internal Use Only\nMay 2026",
         "content": ""},
        {"type": "content", "title": "Market Opportunity", "content": "• Market projected to reach $42B by 2027 (CAGR 13.2%)\n• Top pain points: manual processes (67%), fragmented tools (54%), audit readiness (48%)\n• 78% of compliance officers report increasing regulatory pressure", "chart": {"type": "vertical_bars", "title": "TAM by Segment ($B)", "items": [
            {"label": "Financial\nServices", "value": 18, "color": "003A70"},
            {"label": "Healthcare", "value": 12, "color": "5B9BD5"},
            {"label": "Tech &\nIndustrial", "value": 7, "color": "70AD47"},
            {"label": "Public\nSector", "value": 5, "color": "ED7D31"}
        ]}},
        {"type": "content", "title": "Product Vision", "content": "• AI-native compliance document generation and management\n• Automated regulatory filing — reduce manual effort by 80%\n• Real-time compliance monitoring with instant alerts\n• Intelligent document classification and redaction\n• Multi-jurisdiction support with automatic regulation updates\n• Audit-ready documentation with complete version history\n• Integration with existing GRC platforms and document management systems"},
        {"type": "content", "title": "Target Segments", "content": "• Tier 1: Global banks and financial institutions (50+)\n  — Average deal size: $250-500K annually\n  — Sales cycle: 6-9 months\n• Tier 2: Regional banks and insurance companies (200+)\n  — Average deal size: $80-150K annually\n  — Sales cycle: 3-6 months\n• Tier 3: Healthcare providers and payers (500+)\n  — Average deal size: $40-80K annually\n  — Sales cycle: 2-4 months"},
        {"type": "content", "title": "Go-to-Market Strategy", "content": "• Phase 1 (Q2 2026): Beta with 3 design partners in financial services\n• Phase 2 (Q3 2026): General availability — focus on Tier 2 financial\n• Phase 3 (Q4 2026): Healthcare vertical launch with HIPAA module\n• Channel: Direct sales + 5 system integrator partnerships\n• Pricing: Usage-based + annual subscription model\n• Marketing: Industry conferences, compliance officer communities, content marketing"},
        {"type": "content", "title": "Revenue Projections", "content": "• Year 1: $1.5-2.0M ARR (20-25 initial customers)\n• Year 2: $5-7M ARR (60-80 customers, expansion revenue)\n• Year 3: $15-20M ARR (200+ customers, cross-sell)\n• Average ACV: $85K in Year 1, growing to $100K+\n• Gross margin: 78-82% (SaaS platform economics)\n• Total investment required: $1.2M over 18 months"},
        {"type": "thank_you", "title": "Thank You", "subtitle": "Product Team — Acme Corp\nproduct@acmecorp.com", "content": ""},
    ]
}})
save_ppt("03-product-launch-strategy", r)
print(f"  [OK] Product Launch Strategy")

# ═══════════════════════════════════════════════════════════════════
# DECK 4: M&A Target Analysis
# ═══════════════════════════════════════════════════════════════════
print("\n=== DECK 4: M&A Target Analysis ===")
r = mcp.call("run_skill", {"skill_name": "ppt.deck", "params": {
    "title": "M&A Target Assessment — DocuTech AI",
    "subtitle": "Strategic Acquisition Analysis for the Board",
    "theme": "professional",
    "include_slide_numbers": True,
    "slides": [
        {"type": "title", "title": "M&A Target Assessment", "subtitle": "DocuTech AI — Strategic Acquisition Analysis\nConfidential — Board Materials\nMay 2026",
         "content": ""},
        {"type": "content", "title": "Executive Summary", "content": "• Target: DocuTech AI — AI-powered document intelligence platform\n• Ask Price: $45-55M (all-cash or cash + equity)\n• Strategic rationale: Accelerate AI capabilities, acquire enterprise customers\n• Revenue: $6.2M ARR, 140% YoY growth, 85% gross margin\n• Customer base: 85 enterprise clients, 92% net retention\n• Team: 42 employees (28 engineering, 8 G&A, 6 sales & marketing)\n• Recommendation: Proceed to due diligence with $48M indicative offer"},
        {"type": "content", "title": "Strategic Rationale", "content": "• DocuTech AI's NLP engine complements our OOXML processing capabilities\n• Combined entity offers end-to-end document intelligence: create, process, analyze\n• Enterprise customer overlap: <15% — significant cross-sell opportunity\n• Technology moat: 12 patents pending on AI document classification\n• Accelerates our product verticalization roadmap by 12-18 months\n• Talent acquisition: world-class NLP research team of 8 PhDs"},
        {"type": "content", "title": "Financial Analysis", "content": "• Revenue: $6.2M ARR with 140% YoY growth trajectory\n• Gross Margin: 85% (SaaS model with 92% net revenue retention)\n• EBITDA: -$1.2M (heavy R&D investment phase)\n• Customer ACV: $73K average, $250K top-tier\n• Sales efficiency: Magic Number of 0.95x\n• Projected breakeven: Q3 2027 without acquisition synergies\n• Synergy potential: $3-5M cost savings, $8-12M revenue synergies by Year 3"},
        {"type": "content", "title": "Valuation & Deal Structure", "content": "• Implied EV/ARR multiple: 7.3-8.9x\n• Comps: 5-12x range for AI/SaaS companies at similar scale\n• 40% premium to comp median justified by strategic fit\n• Structure: $35M cash + $13M equity (vesting over 3 years)\n• Earn-out: $5M additional based on FY 2027 revenue target\n• Funding: existing cash ($8.2M) + debt facility ($15M) + equity\n• Accretion to Acme Corp EPS expected by Year 2"},
        {"type": "content", "title": "Integration Plan", "content": "• Day 1-30: Leadership alignment, cultural integration, retention packages\n• Day 31-90: Technology stack integration, API unification\n• Day 91-180: Sales team integration, cross-training, bundled offerings\n• Day 181-365: Unified product roadmap, combined R&D sprints\n• Key risk: Talent retention — 18-month equity vesting for key personnel\n• Integration team: Dedicated PMO with 5 workstream leads"},
        {"type": "thank_you", "title": "Thank You", "subtitle": "Corporate Development — Acme Corp\nstrategy@acmecorp.com", "content": ""},
    ]
}})
save_ppt("04-ma-target-analysis", r)
print(f"  [OK] M&A Target Analysis")

# ═══════════════════════════════════════════════════════════════════
# DECK 5: Digital Transformation Roadmap
# ═══════════════════════════════════════════════════════════════════
print("\n=== DECK 5: Digital Transformation Roadmap ===")
r = mcp.call("run_skill", {"skill_name": "ppt.deck", "params": {
    "title": "Digital Transformation Roadmap 2026–2028",
    "subtitle": "Enterprise Technology Strategy",
    "theme": "professional",
    "include_slide_numbers": True,
    "slides": [
        {"type": "title", "title": "Digital Transformation Roadmap", "subtitle": "Enterprise Technology Strategy 2026–2028\nConfidential\nMay 2026",
         "content": ""},
        {"type": "content", "title": "Vision & Strategic Objectives", "content": "• Vision: Become the AI-native document infrastructure for the enterprise\n• Objective 1: Achieve 10x developer productivity through SDK-first architecture\n• Objective 2: Enable zero-touch document workflows across all formats\n• Objective 3: Build the largest open-source Office document community\n• Objective 4: Achieve 99.99% reliability with sub-millisecond latency\n• Each objective has clear KPIs and quarterly milestones"},
        {"type": "content", "title": "Current State Assessment", "content": "• Strengths: Speed (100-500x vs Python), 4MB binary, zero deps, local-first\n• Weaknesses: Limited write capabilities, no cloud-native deployment option\n• Opportunities: Enterprise standardization on MCP protocol, AI agent adoption\n• Threats: Microsoft Graph API improvements, Google Docs API expansion\n• Gap analysis: 40% of planned tools delivered (36 of 90)\n• Priority: Close the write gap in EPIC-1 and EPIC-2"},
        {"type": "content", "title": "Technology Architecture Evolution", "content": "• Phase 1 (Current): Monolithic Rust binary with MCP stdio transport\n• Phase 2 (Q3 2026): Plugin architecture for custom format handlers\n• Phase 3 (Q1 2027): Streaming HTTP transport, horizontal scaling\n• Phase 4 (Q3 2027): Distributed processing for large-scale enterprise\n• Phase 5 (2028): Edge-native deployment, WASM compilation target\n• Each phase maintains backward compatibility"},
        {"type": "content", "title": "Platform Maturity Model", "content": "• Level 1 (Current): Core read + limited write for 6 Office formats\n• Level 2 (Q3 2026): Full read/write/edit for all formats, 90+ tools\n• Level 3 (Q1 2027): Skills System with 50+ community-contributed skills\n• Level 4 (Q3 2027): Coherence Engine, batch operations, templates\n• Level 5 (2028): AI-native document intelligence and autonomous workflows\n• Each level unlocks new market segments and use cases"},
        {"type": "thank_you", "title": "Thank You", "subtitle": "Technology Team — Acme Corp\neng@acmecorp.com", "content": ""},
    ]
}})
save_ppt("05-digital-transformation", r)
print(f"  [OK] Digital Transformation Roadmap")

mcp.proc.terminate()
print("\n✅ PPT GENERATION COMPLETE")
print(f"Output: {OUT}")
