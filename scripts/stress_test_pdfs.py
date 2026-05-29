#!/usr/bin/env python3
"""Stress test all PDF MCP tools against real-world PDFs."""
import subprocess
import json
import sys
import os
import time

BIN = "target/release/office-oxide-mcp"

def call_tool(name, args, timeout=30):
    proc = subprocess.Popen(
        [BIN], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    init = {"jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "capabilities": {},
                       "clientInfo": {"name": "stress-test", "version": "0.1.0"}}}
    call = {"jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": name, "arguments": args}}
    payload = json.dumps(init) + "\n" + json.dumps(call) + "\n"
    try:
        stdout, stderr = proc.communicate(input=payload.encode(), timeout=timeout)
    except subprocess.TimeoutExpired:
        proc.kill()
        stdout, stderr = proc.communicate()
        return {"error": {"message": f"TIMEOUT after {timeout}s"}}
    for line in stdout.decode().splitlines():
        line = line.strip()
        if not line: continue
        try:
            obj = json.loads(line)
            if obj.get("id") == 2: return obj
        except json.JSONDecodeError: continue
    stderr_text = stderr.decode().strip()
    return {"error": {"message": f"No tool response. stderr={stderr_text}"}}

def get_result_text(response):
    try:
        content = response["result"]["content"]
        for item in content:
            if "text" in item: return item["text"]
        return str(content)
    except (KeyError, TypeError):
        return str(response)

def check_tool(tool, args, assertion):
    r = call_tool(tool, args)
    try:
        if assertion(r): return True, None
        else: return False, get_result_text(r)[:300]
    except Exception as e:
        return False, str(e)

passed, failed = [], []

def test(name, fn):
    try:
        fn()
        passed.append(name)
        print(f"  ✅ {name}")
    except Exception as e:
        failed.append(name)
        print(f"  ❌ {name}: {e}")

outdir = "/tmp/pdf_stress_test"
os.makedirs(outdir, exist_ok=True)

print("=" * 60)
print(" PDF MCP Tools — Stress Test Suite")
print("=" * 60)
print(f"Time: {time.strftime('%Y-%m-%d %H:%M:%S')}")
print()

if not os.path.exists(BIN):
    print(f"❌ Binary not found: {BIN}")
    sys.exit(1)
print(f"✅ Binary: {BIN} ({os.path.getsize(BIN)} bytes)\n")

# 1. list_formats
print("─" * 50)
print("📋 list_formats — PDF presence in format list")
print("─" * 50)
def t1():
    ok, err = check_tool("list_formats", {}, lambda r: "pdf" in get_result_text(r).lower())
    assert ok, f"PDF not in formats: {err}"
test("list_formats includes PDF", t1)

# 2. office_list_pdf_fields
print("\n" + "─" * 50)
print("📋 office_list_pdf_fields — field enumeration")
print("─" * 50)

def t2a():
    ok, err = check_tool("office_list_pdf_fields",
        {"file_path": "test/test_file/eforms-Simple-Job-Application.pdf"},
        lambda r: len(json.loads(get_result_text(r))) >= 10)
    assert ok, err
    fields = json.loads(get_result_text(call_tool("office_list_pdf_fields",
        {"file_path": "test/test_file/eforms-Simple-Job-Application.pdf"})))
    print(f"    Field names: {[f['name'] for f in fields[:5]]}...")
test("AcroForm: eforms (10+ fields)", t2a)

def t2b():
    ok, err = check_tool("office_list_pdf_fields",
        {"file_path": "test/test_file/HVB_Selbstauskunft.pdf"},
        lambda r: len(json.loads(get_result_text(r))) >= 5)
    assert ok, err
test("AcroForm: HVB (5+ fields)", t2b)

def t2c():
    ok, err = check_tool("office_list_pdf_fields",
        {"file_path": "test/test_file/irs-fw2.pdf"},
        lambda r: len(json.loads(get_result_text(r))) >= 10)
    assert ok, err
test("AcroForm: IRS W-2 (10+ fields)", t2c)

def t2d():
    ok, err = check_tool("office_list_pdf_fields",
        {"file_path": "test/test_file/shc-adult-patient-questionnaire.pdf"},
        lambda r: len(json.loads(get_result_text(r))) == 0)
    assert ok, err
test("Flat PDF: no form fields", t2d)

def t2e():
    ok, err = check_tool("office_list_pdf_fields",
        {"file_path": "test/test_file/zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf"},
        lambda r: len(json.loads(get_result_text(r))) > 0)
    assert ok, err
test("XFA: fields detected", t2e)

def t2f():
    ok, err = check_tool("office_list_pdf_fields",
        {"file_path": "/nonexistent/foo.pdf"}, lambda r: "error" in r)
    assert ok, err
test("Missing file → error", t2f)

# 3. office_fill_pdf_form
print("\n" + "─" * 50)
print("📋 office_fill_pdf_form — form field filling")
print("─" * 50)

def t3a():
    ok, err = check_tool("office_fill_pdf_form", {
        "file_path": "test/test_file/eforms-Simple-Job-Application.pdf",
        "output_path": f"{outdir}/eforms-filled.pdf",
        "fields": {"Name last first middle": "John Michael Doe",
                   "Address number street": "123 Main St", "City": "New York"}},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
    s = os.path.getsize(f"{outdir}/eforms-filled.pdf")
    assert s > 1000, f"Output too small: {s}b"
    print(f"    Output: {outdir}/eforms-filled.pdf ({s} bytes)")
test("AcroForm: eforms (3 fields)", t3a)

def t3b():
    ok, err = check_tool("office_fill_pdf_form", {
        "file_path": "test/test_file/HVB_Selbstauskunft.pdf",
        "output_path": f"{outdir}/hvb-filled.pdf",
        "fields": {"Text39": "John", "Text40": "Doe"}},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
    print(f"    Output: {outdir}/hvb-filled.pdf ({os.path.getsize(f'{outdir}/hvb-filled.pdf')}b)")
test("AcroForm: HVB (2 fields)", t3b)

def t3c():
    ok, err = check_tool("office_fill_pdf_form", {
        "file_path": "test/test_file/irs-fw2.pdf",
        "output_path": f"{outdir}/irs-filled.pdf",
        "fields": {"f1_01": "123-45-6789", "f1_02": "ACME Corp"}},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
    print(f"    Output: {outdir}/irs-filled.pdf ({os.path.getsize(f'{outdir}/irs-filled.pdf')}b)")
test("AcroForm: IRS W-2 (2 fields)", t3c)

def t3d():
    ok, err = check_tool("office_fill_pdf_form", {
        "file_path": "test/test_file/zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf",
        "output_path": f"{outdir}/xfa-filled.pdf",
        "fields": {"Summe_Personalkosten_angefordert": "50000",
                   "Summe_Sachkosten_angefordert": "15000"}},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
    print(f"    Output: {outdir}/xfa-filled.pdf ({os.path.getsize(f'{outdir}/xfa-filled.pdf')}b)")
test("XFA: Zahlungsanforderung (2 fields)", t3d)

def t3e():
    ok, err = check_tool("office_fill_pdf_form", {
        "file_path": "test/test_file/shc-adult-patient-questionnaire.pdf",
        "output_path": f"{outdir}/flat-failed.pdf", "fields": {"name": "John"}},
        lambda r: "error" in r)
    assert ok, err
test("Flat PDF: graceful failure", t3e)

def t3f():
    ok, err = check_tool("office_fill_pdf_form", {
        "file_path": "test/test_file/eforms-Simple-Job-Application.pdf",
        "output_path": f"{outdir}/empty.pdf", "fields": {}},
        lambda r: "error" in r)
    assert ok, err
test("Empty fields → error", t3f)

def t3g():
    ok, err = check_tool("office_fill_pdf_form", {
        "file_path": "/nonexistent/test.pdf", "output_path": f"{outdir}/no.pdf",
        "fields": {"name": "test"}}, lambda r: "error" in r)
    assert ok, err
test("Missing file → error", t3g)

# 4. office_overlay_pdf_text
print("\n" + "─" * 50)
print("📋 office_overlay_pdf_text — text overlay")
print("─" * 50)

def t4a():
    ok, err = check_tool("office_overlay_pdf_text", {
        "file_path": "test/test_file/shc-adult-patient-questionnaire.pdf",
        "output_path": f"{outdir}/stanford-filled.pdf",
        "fields": [
            {"page": 1, "x": 180.0, "y": 710.0, "text": "Jane Smith", "font_size": 11.0},
            {"page": 1, "x": 180.0, "y": 690.0, "text": "01/15/1985", "font_size": 11.0},
            {"page": 1, "x": 180.0, "y": 670.0, "text": "Annual checkup", "font_size": 10.0}]},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
    s = os.path.getsize(f"{outdir}/stanford-filled.pdf")
    assert s > 1000, f"Output too small: {s}b"
    print(f"    Output: {outdir}/stanford-filled.pdf ({s} bytes)")
test("Flat PDF overlay (3 fields)", t4a)

def t4b():
    ok, err = check_tool("office_overlay_pdf_text", {
        "file_path": "test/test_file/shc-adult-patient-questionnaire.pdf",
        "output_path": f"{outdir}/multi-font.pdf",
        "fields": [
            {"page": 1, "x": 100.0, "y": 600.0, "text": "Helvetica", "font_name": "Helvetica"},
            {"page": 1, "x": 100.0, "y": 570.0, "text": "Times Roman", "font_name": "Times-Roman", "font_size": 14.0},
            {"page": 1, "x": 100.0, "y": 540.0, "text": "Courier", "font_name": "Courier"}]},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
test("Multi-font (Helvetica/Times/Courier)", t4b)

def t4c():
    ok, err = check_tool("office_overlay_pdf_text", {
        "file_path": "test/test_file/zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf",
        "output_path": f"{outdir}/xfa-overlay.pdf",
        "fields": [
            {"page": 1, "x": 100.0, "y": 700.0, "text": "Overlay line 1"},
            {"page": 1, "x": 100.0, "y": 680.0, "text": "Overlay line 2"}]},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
test("Overlay on XFA (same page, multiple fields)", t4c)

def t4d_mp():
    ok, err = check_tool("office_overlay_pdf_text", {
        "file_path": "test/test_file/shc-adult-patient-questionnaire.pdf",
        "output_path": f"{outdir}/multi-page.pdf",
        "fields": [
            {"page": 1, "x": 180.0, "y": 710.0, "text": "Page 1 name"},
            {"page": 2, "x": 180.0, "y": 700.0, "text": "Page 2 meds"},
            {"page": 3, "x": 180.0, "y": 700.0, "text": "Page 3 signature"}]},
        lambda r: json.loads(get_result_text(r)).get("status") == "filled")
    assert ok, err
    print(f"    Output: {outdir}/multi-page.pdf ({os.path.getsize(f'{outdir}/multi-page.pdf')}b)")
test("Multi-page overlay (3 pages)", t4d_mp)

def t4d():
    ok, err = check_tool("office_overlay_pdf_text", {
        "file_path": "test/test_file/shc-adult-patient-questionnaire.pdf",
        "output_path": f"{outdir}/empty-overlay.pdf", "fields": []},
        lambda r: "error" in r)
    assert ok, err
test("Empty fields → error", t4d)

def t4e():
    ok, err = check_tool("office_overlay_pdf_text", {
        "file_path": "/nonexistent/test.pdf", "output_path": f"{outdir}/no.pdf",
        "fields": [{"page": 1, "text": "test"}]}, lambda r: "error" in r)
    assert ok, err
test("Missing file → error", t4e)

def t4f():
    ok, err = check_tool("office_overlay_pdf_text", {
        "file_path": "/tmp/test.txt", "output_path": f"{outdir}/out.pdf",
        "fields": [{"page": 1, "text": "test"}]}, lambda r: "error" in r)
    assert ok, err
test("Bad extension → error", t4f)

# 5. office_read (PDF)
print("\n" + "─" * 50)
print("📋 office_read — PDF text extraction")
print("─" * 50)

def t5a():
    ok, err = check_tool("office_read",
        {"file_path": "test/test_file/eforms-Simple-Job-Application.pdf", "output_format": "markdown"},
        lambda r: len(get_result_text(r)) > 50)
    assert ok, err
test("AcroForm → markdown", t5a)
def t5b():
    ok, err = check_tool("office_read",
        {"file_path": "test/test_file/eforms-Simple-Job-Application.pdf", "output_format": "text"},
        lambda r: len(get_result_text(r)) > 50)
    assert ok, err
test("AcroForm → text", t5b)
def t5c():
    ok, err = check_tool("office_read",
        {"file_path": "test/test_file/eforms-Simple-Job-Application.pdf", "output_format": "json"},
        lambda r: len(json.loads(get_result_text(r)).get("content", [])) > 0)
    assert ok, err
test("AcroForm → json", t5c)
def t5d():
    ok, err = check_tool("office_read",
        {"file_path": "test/test_file/eforms-Simple-Job-Application.pdf", "output_format": "chunks"},
        lambda r: len(json.loads(get_result_text(r))) > 0)
    assert ok, err
test("AcroForm → chunks", t5d)
def t5e():
    ok, err = check_tool("office_read",
        {"file_path": "test/test_file/shc-adult-patient-questionnaire.pdf", "output_format": "text"},
        lambda r: len(get_result_text(r)) > 50)
    assert ok, err
test("Flat PDF → text", t5e)
def t5f():
    ok, err = check_tool("office_read",
        {"file_path": "test/test_file/zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf",
         "output_format": "text"},
        lambda r: len(get_result_text(r)) > 50)
    assert ok, err
test("XFA → text", t5f)
def t5g():
    ok, err = check_tool("office_read",
        {"file_path": "test/test_file/zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf",
         "output_format": "json"},
        lambda r: len(json.loads(get_result_text(r)).get("form_fields", [])) > 0)
    assert ok, err
    data = json.loads(get_result_text(call_tool("office_read",
        {"file_path": "test/test_file/zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf",
         "output_format": "json"})))
    print(f"    XFA JSON: {len(data.get('form_fields',[]))} fields, {data.get('page_count',0)} pages")
test("XFA → json (form_fields + page_count)", t5g)
def t5h():
    ok, err = check_tool("office_read",
        {"file_path": "Cargo.toml", "output_format": "text"},
        lambda r: "error" in r)
    assert ok, err
test("Non-PDF → error", t5h)

# 6. office_analyze_pdf_layout
print("\n" + "─" * 50)
print("📋 office_analyze_pdf_layout — layout analysis")
print("─" * 50)

def t6a():
    ok, err = check_tool("office_analyze_pdf_layout",
        {"file_path": "test/test_file/eforms-Simple-Job-Application.pdf"},
        lambda r: len(get_result_text(r)) > 50)
    assert ok, err
test("AcroForm layout", t6a)
def t6b():
    ok, err = check_tool("office_analyze_pdf_layout",
        {"file_path": "test/test_file/shc-adult-patient-questionnaire.pdf"},
        lambda r: len(get_result_text(r)) > 50)
    assert ok, err
test("Flat PDF layout", t6b)
def t6c():
    ok, err = check_tool("office_analyze_pdf_layout",
        {"file_path": "test/test_file/zahlungsanforderung-innovationsnetzwerke-fue-einzelprojekt.pdf"},
        lambda r: len(get_result_text(r)) > 50)
    assert ok, err
test("XFA layout", t6c)
def t6d():
    ok, err = check_tool("office_analyze_pdf_layout",
        {"file_path": "/nonexistent/foo.pdf"}, lambda r: "error" in r)
    assert ok, err
test("Missing file → error", t6d)

# Results
print(f"\n{'=' * 60}")
print(f" RESULTS: {len(passed)} passed, {len(failed)} failed, "
      f"{len(passed) + len(failed)} total")
print(f"{'=' * 60}")
if failed:
    for name in failed: print(f"  ❌ {name}")
    sys.exit(1)
else:
    print("\n🎉 ALL TESTS PASSED!")
    sys.exit(0)
