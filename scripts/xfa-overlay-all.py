import json, subprocess

root="/home/tamnguyen/repos/Operation/office-oxide-mcp"
bin="target/debug/office-oxide-mcp"
init=json.dumps({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}})

def mcp(name,args):
    call=json.dumps({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":name,"arguments":args}})
    p=subprocess.run([bin],input=init+"\n"+call+"\n",capture_output=True,text=True,cwd=root,timeout=30)
    for l in p.stdout.strip().split('\n'):
        try:
            m=json.loads(l)
            if m.get('id')==2: return m['result']['content'][0].get('text','')
        except: pass
    return ""

# Get filled values
r=mcp("office_list_pdf_fields",{"file_path":"tests/output/zahlungsanforderung-viewable.pdf"})
fields=json.loads(r)

# Build overlay fields for ALL 19 fields on page 0
overlay_fields = []
y = 720
overlay_fields.append({"page":0,"x":50,"y":y,"text":"XFA FORM - ALL FILLED VALUES:","font_size":13})
y -= 18
for f in fields:
    name=f.get('name','')
    val=f.get('current_value','') or ''
    if len(str(val))>70: val=str(val)[:70]+"..."
    overlay_fields.append({"page":0,"x":55,"y":y,"text":f"{name[:35]}: {val}","font_size":8})
    y -= 11
    if y < 30: break  # don't overflow page

# Overlay on the filled XFA
result=mcp("office_overlay_pdf_text",{"file_path":"tests/output/zahlungsanforderung-viewable.pdf","output_path":"tests/output/zahlungsanforderung-overlay-all.pdf","fields":overlay_fields})
print(f"OVERLAY: {result}")

# Read back to verify
r=mcp("office_read",{"file_path":"tests/output/zahlungsanforderung-overlay-all.pdf","output_format":"text"})
if 'XFA FILL' in r or 'Summe_Personalkosten' in r:
    lines=[l for l in r.split('\n') if l.strip()]
    print(f"\nREADBACK: {len(lines)} lines")
    print('\n'.join(lines[-15:]))
    print("\n✅ VIEWABLE PROOF GENERATED")
else:
    print(f"\nRAW TEXT:\n{r[:500]}")
