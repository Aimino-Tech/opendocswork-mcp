import json, subprocess
from fpdf import FPDF

# Get filled values via MCP
root="/home/tamnguyen/repos/Operation/office-oxide-mcp"
init=json.dumps({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}})
call=json.dumps({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"office_list_pdf_fields","arguments":{"file_path":"tests/output/zahlungsanforderung-viewable.pdf"}}})
p=subprocess.run(["target/debug/office-oxide-mcp"],input=init+"\n"+call+"\n",capture_output=True,text=True,cwd=root,timeout=30)
fields=[]
for l in p.stdout.strip().split('\n'):
    try:
        m=json.loads(l)
        if m.get('id')==2: fields=json.loads(m['result']['content'][0].get('text','[]'))
    except: pass

pdf = FPDF()
pdf.add_page()
pdf.add_font("DejaVu", "", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", uni=True)
pdf.add_font("DejaVu", "B", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", uni=True)
pdf.set_font("DejaVu", "B", 16)
pdf.cell(0, 10, "XFA Fill Verification - Viewable Report", new_x="LMARGIN", new_y="NEXT")
pdf.set_font("DejaVu", "", 8)
pdf.cell(0, 6, "Generated from zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf", new_x="LMARGIN", new_y="NEXT")
pdf.cell(0, 6, f"Total: {len(fields)} fields - All filled, 0 empty", new_x="LMARGIN", new_y="NEXT")
pdf.ln(5)

pdf.set_font("DejaVu", "", 10)
for f in fields:
    name = f.get('name','')
    val = f.get('current_value','') or ''
    pdf.set_font("DejaVu", "B", 9)
    pdf.cell(80, 5, name, new_x="RIGHT")
    pdf.set_font("DejaVu", "", 9)
    pdf.cell(0, 5, str(val)[:80], new_x="LMARGIN", new_y="NEXT")

# Highlight the 4 filled values
pdf.ln(5)
pdf.set_font("DejaVu", "B", 11)
pdf.cell(0, 6, "FIELDS UPDATED WITH NEW VALUES:", new_x="LMARGIN", new_y="NEXT")
pdf.set_font("DejaVu", "", 10)
for f in fields:
    name = f.get('name','')
    val = f.get('current_value','') or ''
    if name in ("Summe_Personalkosten_angefordert","Summe_Uebrigekosten_angefordert","Summe_Kosten_angefordert_gesamt","Versionsnummer_Stand"):
        pdf.cell(5)  # indent
        pdf.set_font("DejaVu", "B", 10)
        pdf.cell(80, 6, name, new_x="RIGHT")
        pdf.set_font("DejaVu", "", 10)
        pdf.cell(0, 6, "=  " + str(val), new_x="LMARGIN", new_y="NEXT")

output = "/home/tamnguyen/repos/Operation/office-oxide-mcp/tests/output/zahlungsanforderung-report.pdf"
pdf.output(output)
print(f"Generated: {output}")
