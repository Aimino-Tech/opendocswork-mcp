use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// Result type for form operations.
type PdfResult<T> = Result<T, String>;

/// Information about a single form field.
#[derive(Debug, Clone, Serialize)]
pub struct FieldInfo {
    pub name: String,
    pub field_type: String,
    pub current_value: Option<String>,
}

/// Structured result for fill_form operations.
#[derive(Debug, Clone, Serialize)]
pub struct FillFormResult {
    pub status: String,
    pub filled_field_count: usize,
    pub output_path: String,
}

/// Decode a PDF string or name value, handling UTF-16BE/LE encoding with BOM.
/// Falls back to standard lossy UTF-8 decoding.
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.starts_with(b"\xFE\xFF") && bytes.len() >= 4 {
        let chars: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&chars).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
    } else if bytes.starts_with(b"\xFF\xFE") && bytes.len() >= 4 {
        let chars: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|c| c.len() == 2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16(&chars).unwrap_or_else(|_| String::from_utf8_lossy(bytes).to_string())
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// Fills and inspects form fields in PDF documents (AcroForm and XFA).
pub struct PdfFormFiller;

impl PdfFormFiller {
    pub fn new() -> Self {
        Self
    }

    fn try_decrypt(doc: &mut Document) {
        if doc.is_encrypted() {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if doc.decrypt("").is_err() {
                    let _ = doc.decrypt("owner");
                }
            }));
            if result.is_err() {
                eprintln!("Warning: PDF decryption failed (lopdf limitation for AES)");
            }
        }
    }

    /// lopdf sometimes fails to resolve objects from ObjStm streams in PDFs
    /// with hybrid xref (xref stream + table) or linearized PDFs.
    fn resolve_object_streams(doc: &mut Document) {
        let obj_ids: Vec<_> = doc.objects.keys().cloned().collect();
        let mut new_objects: BTreeMap<(u32, u16), Object> = BTreeMap::new();
        let mut objstm_count = 0;
        let mut total_objects = 0;

        for id in &obj_ids {
            if let Some(stream) = doc.objects.get(id).and_then(|o| o.as_stream().ok()) {
                if stream.dict.has_type(b"ObjStm") {
                    objstm_count += 1;
                    let mut s = stream.clone();
                    let content_len_before = s.content.len();
                    match lopdf::ObjectStream::new(&mut s) {
                        Ok(os) => {
                            let c = os.objects.len();
                            total_objects += c;
                            if c > 0 {
                                eprintln!("  ObjStm {}: extracted {} objects", id.0, c);
                            }
                            for (oid, obj) in os.objects {
                                new_objects.entry(oid).or_insert(obj);
                            }
                        }
                        Err(_e) => {
                            let mut s2 = stream.clone();
                            if s2.decompress().is_ok() {
                                if let Ok(os) = lopdf::ObjectStream::new(&mut s2) {
                                    let c = os.objects.len();
                                    total_objects += c;
                                    eprintln!("  ObjStm {}: extracted {} objects (after manual decompress)", id.0, c);
                                    for (oid, obj) in os.objects {
                                        new_objects.entry(oid).or_insert(obj);
                                    }
                                } else {
                                    eprintln!("  ObjStm {}: ObjectStream::new failed even after decompress: content_len={}", id.0, content_len_before);
                                }
                            } else {
                                eprintln!(
                                    "  ObjStm {}: decompress failed, content_len={}",
                                    id.0, content_len_before
                                );
                            }
                        }
                    }
                }
            }
        }
        eprintln!(
            "  resolve_object_streams: {} ObjStm, {} total objects extracted",
            objstm_count, total_objects
        );

        for (id, obj) in new_objects {
            doc.objects.entry(id).or_insert(obj);
        }
    }

    /// Analyze a PDF's page layout: extract text positions from content streams,
    /// detect field labels, and suggest overlay coordinates.
    /// Returns a JSON string with page layouts and field suggestions.
    pub fn analyze_layout(input_path: &str) -> PdfResult<String> {
        let mut doc =
            Document::load(input_path).map_err(|e| format!("Failed to load PDF: {}", e))?;
        Self::try_decrypt(&mut doc);
        Self::resolve_object_streams(&mut doc);

        let mut pages_data: Vec<serde_json::Value> = Vec::new();
        let page_ids = doc.get_pages();
        let total_pages: u32 = page_ids.len() as u32;

        for (&page_num, &page_id) in &page_ids {
            let mut text_items: Vec<serde_json::Value> = Vec::new();

            match doc.get_and_decode_page_content(page_id) {
                Ok(content) => {
                    let mut i = 0;
                    while i < content.operations.len() {
                        let op = &content.operations[i];
                        if op.operator == "BT" {
                            let mut x = 0.0f32;
                            let mut y = 0.0f32;
                            let mut texts = Vec::new();
                            let mut font = String::new();
                            for j in i + 1..content.operations.len() {
                                let op2 = &content.operations[j];
                                match op2.operator.as_str() {
                                    "ET" => {
                                        if !texts.is_empty() {
                                            text_items.push(serde_json::json!({
                                                "x": x,
                                                "y": y,
                                                "text": texts.join(" "),
                                                "font": font,
                                            }));
                                        }
                                        i = j;
                                        break;
                                    }
                                    "Tf" => {
                                        if op2.operands.len() >= 2 {
                                            font = op2.operands[0]
                                                .as_name()
                                                .map(|n| String::from_utf8_lossy(n).to_string())
                                                .unwrap_or_default();
                                        }
                                    }
                                    "Td" => {
                                        // Td is relative — accumulate from previous position
                                        if op2.operands.len() >= 2 {
                                            x += op2.operands[0].as_f32().unwrap_or(0.0);
                                            y += op2.operands[1].as_f32().unwrap_or(0.0);
                                        }
                                    }
                                    "Tm" => {
                                        if op2.operands.len() >= 6 {
                                            y = op2.operands[5].as_f32().unwrap_or(y);
                                        }
                                    }
                                    "Tj" => {
                                        if let Ok(s) = op2.operands[0].as_str() {
                                            texts.push(String::from_utf8_lossy(s).to_string());
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        i += 1;
                    }
                }
                Err(_) => {
                    // Content stream decode failed — fallback: scan raw streams
                    if let Ok(page_dict) = doc.get_dictionary(page_id) {
                        if let Ok(contents) = page_dict.get(b"Contents") {
                            let stream_ids: Vec<ObjectId> = match contents {
                                Object::Reference(id) => vec![*id],
                                Object::Array(arr) => {
                                    arr.iter().filter_map(|o| o.as_reference().ok()).collect()
                                }
                                _ => vec![],
                            };
                            for &sid in &stream_ids {
                                if let Ok(obj) = doc.get_object(sid) {
                                    if let Ok(stream) = obj.as_stream() {
                                        if let Ok(raw) = stream.decompressed_content() {
                                            let s = String::from_utf8_lossy(&raw);
                                            for line in s.lines() {
                                                let t = line.trim();
                                                if t.contains("Tj") || t.contains("TJ") {
                                                    text_items.push(serde_json::json!({
                                                        "raw": t,
                                                    }));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            pages_data.push(serde_json::json!({
                "page": page_num,
                "text_count": text_items.len(),
                "texts": text_items,
            }));
        }

        let result = serde_json::json!({
            "file": input_path,
            "pages": total_pages,
            "page_layouts": pages_data,
        });

        serde_json::to_string_pretty(&result).map_err(|e| format!("Serialization error: {}", e))
    }

    /// Fill form fields in a PDF document.
    ///
    /// Opens the PDF, detects form type (AcroForm or XFA), fills matching
    /// fields, and saves to the output path.
    pub fn fill_form(
        &self,
        input_path: &str,
        output_path: &str,
        fields: &HashMap<String, String>,
    ) -> PdfResult<String> {
        let mut doc =
            Document::load(input_path).map_err(|e| format!("Failed to load PDF: {}", e))?;

        Self::try_decrypt(&mut doc);

        Self::resolve_object_streams(&mut doc);

        let acro_form_ref = match doc.catalog().and_then(|c| c.get(b"AcroForm")) {
            Ok(Object::Reference(id)) => *id,
            Ok(Object::Dictionary(_)) => {
                return Err("AcroForm as direct dictionary not supported".to_string());
            }
            Err(_) => {
                return Err("No AcroForm found in PDF document".to_string());
            }
            _ => {
                return Err("Invalid AcroForm entry in catalog".to_string());
            }
        };

        let (has_xfa, has_fields) = {
            let acro_form_dict = match doc.get_dictionary(acro_form_ref) {
                Ok(d) => d,
                Err(_) => {
                    Self::resolve_object_streams(&mut doc);
                    doc.get_dictionary(acro_form_ref)
                        .map_err(|e| format!("Failed to get AcroForm dictionary: {}", e))?
                }
            };
            (acro_form_dict.has(b"XFA"), acro_form_dict.has(b"Fields"))
        };

        // If the PDF has both XFA and AcroForm Fields, fill both
        // If it has only XFA, fill XFA only
        // If it has only Fields, fill AcroForm only
        let mut filled_count = 0;

        if has_xfa {
            match self.fill_xfa_fields(&mut doc, acro_form_ref, fields) {
                Ok(count) => filled_count += count,
                Err(_) => { /* XFA matching failed — try AcroForm below */ }
            }
        }

        if has_fields {
            filled_count += self.fill_acroform_fields(&mut doc, acro_form_ref, fields)?;
        }

        if filled_count == 0 && !has_xfa && !has_fields {
            return Err(
                "PDF has no recognized form fields (no XFA or Fields found in AcroForm)"
                    .to_string(),
            );
        }

        if filled_count == 0 && !has_fields {
            if !has_xfa {
                return Err(
                    "PDF has no recognized form fields (no XFA or Fields found in AcroForm)"
                        .to_string(),
                );
            }
        } else if filled_count > 0 || has_fields {
            if let Ok(acro_form_dict) = doc.get_dictionary_mut(acro_form_ref) {
                acro_form_dict.set("NeedAppearances", true);
            }
        }

        doc.save(output_path)
            .map_err(|e| format!("Failed to save PDF to '{}': {}", output_path, e))?;

        let result = FillFormResult {
            status: "filled".to_string(),
            filled_field_count: filled_count,
            output_path: output_path.to_string(),
        };

        serde_json::to_string_pretty(&result).map_err(|e| format!("Serialization error: {}", e))
    }

    /// List all form fields in a PDF document with their current values.
    pub fn list_fields(&self, input_path: &str) -> PdfResult<String> {
        let mut doc =
            Document::load(input_path).map_err(|e| format!("Failed to load PDF: {}", e))?;

        Self::try_decrypt(&mut doc);
        Self::resolve_object_streams(&mut doc);

        let (acro_form_ref, has_xfa, fields_array) = {
            let catalog = doc
                .catalog()
                .map_err(|e| format!("Failed to read PDF catalog: {}", e))?;

            let acro_form = match catalog.get(b"AcroForm") {
                Ok(obj) => obj,
                Err(_) => {
                    return Ok("[]".to_string());
                }
            };

            let (ref_id, dict) = match acro_form {
                Object::Reference(id) => {
                    let dict = doc
                        .get_dictionary(*id)
                        .map_err(|e| format!("Failed to get AcroForm dictionary: {}", e))?;
                    (*id, dict)
                }
                Object::Dictionary(_dict) => {
                    return Err("AcroForm as direct dictionary not supported".to_string());
                }
                _ => {
                    return Err("Invalid AcroForm entry in catalog".to_string());
                }
            };

            let fields_arr = dict
                .get(b"Fields")
                .ok()
                .and_then(|f| f.as_array().ok())
                .cloned();
            let has_xfa = dict.has(b"XFA");

            (ref_id, has_xfa, fields_arr)
        };

        let mut fields: Vec<FieldInfo> = Vec::new();

        // Collect AcroForm fields (if Fields array exists)
        if let Some(ref fields_arr) = fields_array {
            for field_ref in fields_arr.iter() {
                if let Ok(field_id) = field_ref.as_reference() {
                    self.collect_fields_recursive(&doc, field_id, "", &mut fields);
                }
            }
        }

        // Collect XFA field names from the XDP XML
        if has_xfa {
            let _ = self.collect_xfa_fields(&doc, acro_form_ref, &mut fields);
        }

        serde_json::to_string_pretty(&fields).map_err(|e| format!("Serialization error: {}", e))
    }

    // ─── AcroForm Implementation ─────────────────────────────────

    /// Fill AcroForm fields by matching field names and setting /V values.
    /// Handles hierarchical field names and checkbox types.
    fn fill_acroform_fields(
        &self,
        doc: &mut Document,
        acro_form_ref: ObjectId,
        fields: &HashMap<String, String>,
    ) -> PdfResult<usize> {
        let fields_array = {
            let acro_form_dict = doc
                .get_dictionary(acro_form_ref)
                .map_err(|e| format!("Failed to get AcroForm dictionary: {}", e))?;
            acro_form_dict
                .get(b"Fields")
                .ok()
                .and_then(|f| f.as_array().ok())
                .cloned()
                .ok_or_else(|| "AcroForm has no Fields array".to_string())?
        };

        // Collect all field reference IDs from the Fields array recursively
        let all_field_ids = self.collect_field_ids(doc, &fields_array);

        let mut filled_count = 0;

        for &field_id in &all_field_ids {
            if let Ok(field_dict) = doc.get_dictionary(field_id) {
                // Get the field name (both top-level and full hierarchical path)
                let field_name = match field_dict.get(b"T").and_then(Object::as_str) {
                    Ok(name) => decode_pdf_string(name),
                    Err(_) => continue,
                };

                // Try exact match first, then partial name match
                let matched_value = fields.get(&field_name).or_else(|| {
                    // Try partial/fuzzy match (field_name contains key or vice versa)
                    fields
                        .iter()
                        .find(|(key, _)| {
                            field_name.contains(key.as_str()) || key.contains(&field_name)
                        })
                        .map(|(_, v)| v)
                });

                if let Some(value) = matched_value {
                    if let Ok(field_dict_mut) = doc.get_dictionary_mut(field_id) {
                        // Detect field type: Button (Btn), Text (Tx), Choice (Ch), etc.
                        let field_type = field_dict_mut
                            .get(b"FT")
                            .ok()
                            .and_then(|o| o.as_name().ok())
                            .map(|n| String::from_utf8_lossy(n).to_string());

                        match field_type.as_deref() {
                            Some("Btn") => {
                                let name_value = value.as_bytes().to_vec();
                                let ff = field_dict_mut
                                    .get(b"Ff")
                                    .ok()
                                    .and_then(|o| o.as_i64().ok())
                                    .unwrap_or(0);
                                let has_kids = field_dict_mut.has(b"Kids");
                                let parent_id = field_dict_mut
                                    .get(b"Parent")
                                    .ok()
                                    .and_then(|o| o.as_reference().ok());

                                // Helper: resolve the actual checkbox state name from user's value
                                // by checking AP/N for non-Off keys. Maps "Yes" → "On" etc.
                                let get_checked_name = |d: &Document,
                                                        obj_id: ObjectId,
                                                        fallback: Vec<u8>|
                                 -> Vec<u8> {
                                    let non_off_keys: Vec<Vec<u8>> = d
                                        .get_dictionary(obj_id)
                                        .ok()
                                        .and_then(|dict| {
                                            dict.get(b"AP").ok().and_then(|o| o.as_dict().ok())
                                        })
                                        .and_then(|ap| {
                                            ap.get(b"N").ok().and_then(|o| o.as_dict().ok())
                                        })
                                        .map(|n| {
                                            n.iter()
                                                .filter(|(k, _)| k.as_slice() != b"Off")
                                                .map(|(k, _)| k.clone())
                                                .collect()
                                        })
                                        .unwrap_or_default();
                                    if non_off_keys.is_empty() {
                                        return fallback;
                                    }
                                    // Exact match
                                    if non_off_keys.iter().any(|k| k == &fallback) {
                                        return fallback;
                                    }
                                    // Prefix match (e.g. "Yes" matches "Yes_6")
                                    if let Some(m) = non_off_keys.iter().find(|k| {
                                        let kstr = String::from_utf8_lossy(k);
                                        let vstr = String::from_utf8_lossy(&fallback);
                                        kstr.starts_with(vstr.as_ref())
                                    }) {
                                        return m.clone();
                                    }
                                    // User sent a truthy value, use first checked key
                                    let vstr = String::from_utf8_lossy(&fallback).to_lowercase();
                                    if matches!(
                                        vstr.as_str(),
                                        "yes" | "true" | "1" | "x" | "on" | "checked"
                                    ) {
                                        return non_off_keys.into_iter().next().unwrap_or(fallback);
                                    }
                                    fallback
                                };

                                // Radio group parent: has Kids + Radio flag (Ff bit 15 = 0x8000)
                                if has_kids && (ff & 0x8000) != 0 {
                                    let kid_ids: Vec<ObjectId> = doc
                                        .get_dictionary(field_id)
                                        .ok()
                                        .and_then(|d| {
                                            d.get(b"Kids")
                                                .ok()
                                                .and_then(|k| k.as_array().ok().cloned())
                                        })
                                        .unwrap_or_default()
                                        .iter()
                                        .filter_map(|o| o.as_reference().ok())
                                        .collect();
                                    let checked = get_checked_name(
                                        doc,
                                        kid_ids.first().copied().unwrap_or(field_id),
                                        name_value,
                                    );
                                    if let Ok(p_mut) = doc.get_dictionary_mut(field_id) {
                                        p_mut.set("V", Object::Name(checked.clone()));
                                        p_mut.set("DV", Object::Name(checked.clone()));
                                    }
                                    for &k in &kid_ids {
                                        let is_selected = doc
                                            .get_dictionary(k)
                                            .ok()
                                            .and_then(|d| {
                                                d.get(b"AP").ok().and_then(|o| o.as_dict().ok())
                                            })
                                            .and_then(|ap| {
                                                ap.get(b"N").ok().and_then(|o| o.as_dict().ok())
                                            })
                                            .map(|n| n.has(&checked))
                                            .unwrap_or(false);
                                        let state = if is_selected {
                                            checked.clone()
                                        } else {
                                            b"Off".to_vec()
                                        };
                                        if let Ok(k_mut) = doc.get_dictionary_mut(k) {
                                            k_mut.set("AS", Object::Name(state));
                                        }
                                    }
                                    filled_count += 1;
                                    continue;
                                }

                                // Radio group child: has Parent pointing to radio group
                                let is_radio_child = parent_id.is_some_and(|p| {
                                    doc.get_dictionary(p).ok().is_some_and(|d| {
                                        d.get(b"FT")
                                            .ok()
                                            .and_then(|o| o.as_name().ok())
                                            .map(|n| n == b"Btn")
                                            .unwrap_or(false)
                                    })
                                });
                                if is_radio_child {
                                    let p_ref = parent_id.unwrap();
                                    let kid_ids: Vec<ObjectId> = doc
                                        .get_dictionary(p_ref)
                                        .ok()
                                        .and_then(|d| {
                                            d.get(b"Kids")
                                                .ok()
                                                .and_then(|k| k.as_array().ok().cloned())
                                        })
                                        .unwrap_or_default()
                                        .iter()
                                        .filter_map(|o| o.as_reference().ok())
                                        .collect();
                                    let checked = get_checked_name(doc, field_id, name_value);
                                    if let Ok(p_mut) = doc.get_dictionary_mut(p_ref) {
                                        p_mut.set("V", Object::Name(checked.clone()));
                                        p_mut.set("DV", Object::Name(checked));
                                    }
                                    for &k in &kid_ids {
                                        let state = get_checked_name(doc, k, b"Off".to_vec());
                                        if let Ok(k_mut) = doc.get_dictionary_mut(k) {
                                            k_mut.set("AS", Object::Name(state));
                                        }
                                    }
                                    filled_count += 1;
                                    continue;
                                }

                                // Standalone checkbox: set V + AS on the field itself
                                let checked = get_checked_name(doc, field_id, name_value);
                                if let Ok(fd_mut) = doc.get_dictionary_mut(field_id) {
                                    fd_mut.set("AS", Object::Name(checked.clone()));
                                    fd_mut.set("V", Object::Name(checked.clone()));
                                    fd_mut.set("DV", Object::Name(checked));
                                }
                                filled_count += 1;
                            }
                            Some("Ch") => {
                                // Choice field: /V is typically a string (index or value)
                                // Some choice fields expect Name, some expect string
                                // Try string first (most common)
                                field_dict_mut.set("V", Object::string_literal(value.as_bytes()));
                                field_dict_mut.set("DV", Object::string_literal(value.as_bytes()));
                                filled_count += 1;
                            }
                            Some("Sig") => {
                                // Signature field: skip silently
                                continue;
                            }
                            _ => {
                                // Text (Tx) and other types: /V as string
                                field_dict_mut.set("V", Object::string_literal(value.as_bytes()));
                                field_dict_mut.set("DV", Object::string_literal(value.as_bytes()));
                                filled_count += 1;
                                // Generate appearance stream for the modified field
                                if let Ok(obj) = field_dict_mut.get(b"DA") {
                                    if let Ok(da_bytes) = obj.as_str() {
                                        let da_str = String::from_utf8_lossy(da_bytes).to_string();
                                        let _ = Self::generate_field_appearance(
                                            doc,
                                            field_id,
                                            &field_name,
                                            value,
                                            &da_str,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(filled_count)
    }

    /// Generate a minimal appearance stream for a filled text field widget.
    /// Parses the /DA (default appearance) string to extract font & color,
    /// builds a Form XObject with the field value rendered inside the widget /Rect,
    /// and sets it as /AP → /N on the widget annotation.
    fn generate_field_appearance(
        doc: &mut Document,
        field_id: ObjectId,
        _field_name: &str,
        value: &str,
        da: &str,
    ) -> PdfResult<()> {
        // Don't modify if we can't get the field dict
        let (rect, ft) = match doc.get_dictionary(field_id) {
            Ok(dict) => {
                let ft = dict
                    .get(b"FT")
                    .ok()
                    .and_then(|o| o.as_name().ok())
                    .map(|n| String::from_utf8_lossy(n).to_string())
                    .unwrap_or_default();
                let rect = dict
                    .get(b"Rect")
                    .ok()
                    .and_then(|o| o.as_array().ok())
                    .cloned()
                    .unwrap_or_default();
                (rect, ft)
            }
            Err(_) => return Ok(()),
        };

        // Only generate appearances for Tx fields with valid rects
        if ft != "Tx" || rect.len() < 4 {
            return Ok(());
        }

        let mut font_name = "Helv".to_string();
        let mut font_size = 10.0f32;
        if let Some(tf_pos) = da.find("Tf") {
            let before = da[..tf_pos].trim();
            let tokens: Vec<&str> = before.split_whitespace().collect();
            if let Some(last) = tokens.last() {
                font_size = last.parse::<f32>().unwrap_or(10.0);
            }
            for tok in tokens.iter().rev().skip(1) {
                if let Some(stripped) = tok.strip_prefix('/') {
                    font_name = stripped.to_string();
                    break;
                }
            }
        }

        let x1 = rect[0].as_f32().unwrap_or(0.0);
        let y1 = rect[1].as_f32().unwrap_or(0.0);
        let x2 = rect[2].as_f32().unwrap_or(x1 + 100.0);
        let y2 = rect[3].as_f32().unwrap_or(y1 + 20.0);
        let w = x2 - x1;
        let h = y2 - y1;

        // Build appearance content stream
        let encoded = format!(
            "BT\n/{} {} Tf\n{} {} Td\n({}) Tj\nET",
            font_name,
            font_size,
            2.0,
            h - font_size - 2.0,
            value
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)"),
        );

        let base_font = map_font_name(&font_name);
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => Object::Name(base_font.as_bytes().to_vec()),
        });

        let ap_stream = Stream::new(
            dictionary! {
                "Type" => "XObject",
                "Subtype" => "Form",
                "BBox" => vec![0.into(), 0.into(), w.into(), h.into()],
                "FormType" => 1,
                "Resources" => dictionary! {
                        "Font" => dictionary! {
                        font_name.as_str() => Object::Reference(font_id),
                    },
                },
            },
            encoded.as_bytes().to_vec(),
        );
        let ap_id = doc.add_object(ap_stream);

        // Set /AP << /N <ref> >> on the field
        if let Ok(field_mut) = doc.get_dictionary_mut(field_id) {
            field_mut.set(
                "AP",
                dictionary! {
                    "N" => Object::Reference(ap_id),
                },
            );
        }

        Ok(())
    }

    /// Recursively collect field object IDs from a Fields array.
    /// Any object that has a /T (field name) in the subtree is collected.
    fn collect_field_ids(&self, doc: &Document, fields_array: &[Object]) -> Vec<ObjectId> {
        let mut result = Vec::new();
        for field_ref in fields_array.iter() {
            if let Ok(field_id) = field_ref.as_reference() {
                self.collect_field_ids_recursive(doc, field_id, &mut result);
            }
        }
        result
    }

    /// Recursively walk field hierarchy, collecting all IDs with /T.
    fn collect_field_ids_recursive(
        &self,
        doc: &Document,
        field_id: ObjectId,
        result: &mut Vec<ObjectId>,
    ) {
        if let Ok(field_dict) = doc.get_dictionary(field_id) {
            if field_dict.has(b"T") {
                result.push(field_id);
            }

            if let Ok(kids) = field_dict.get(b"Kids").and_then(Object::as_array) {
                for kid_ref in kids.iter() {
                    if let Ok(kid_id) = kid_ref.as_reference() {
                        self.collect_field_ids_recursive(doc, kid_id, result);
                    }
                }
            }
        }
    }

    /// Recursively collect field information from AcroForm fields.
    fn collect_fields_recursive(
        &self,
        doc: &Document,
        field_id: ObjectId,
        parent_name: &str,
        result: &mut Vec<FieldInfo>,
    ) {
        if let Ok(field_dict) = doc.get_dictionary(field_id) {
            let field_name = match field_dict.get(b"T").and_then(Object::as_str) {
                Ok(name) => {
                    let name_str = decode_pdf_string(name);
                    if parent_name.is_empty() {
                        name_str.to_string()
                    } else {
                        format!("{}.{}", parent_name, name_str)
                    }
                }
                Err(_) => {
                    // Anonymous field, skip unless it has interesting kids
                    if field_dict.has(b"Kids") {
                        if let Ok(kids) = field_dict.get(b"Kids").and_then(|k| k.as_array()) {
                            for kid_ref in kids.iter() {
                                if let Ok(kid_id) = kid_ref.as_reference() {
                                    self.collect_fields_recursive(doc, kid_id, parent_name, result);
                                }
                            }
                        }
                    }
                    return;
                }
            };

            let field_type = match field_dict.get(b"FT") {
                Ok(Object::Name(name)) => String::from_utf8_lossy(name).to_string(),
                _ => "Unknown".to_string(),
            };

            let current_value = match field_dict.get(b"V") {
                Ok(Object::String(s, _)) => Some(String::from_utf8_lossy(s).to_string()),
                Ok(Object::Name(n)) => Some(String::from_utf8_lossy(n).to_string()),
                _ => None,
            };

            result.push(FieldInfo {
                name: field_name.clone(),
                field_type,
                current_value,
            });

            if let Ok(kids) = field_dict.get(b"Kids").and_then(Object::as_array) {
                for kid_ref in kids.iter() {
                    if let Ok(kid_id) = kid_ref.as_reference() {
                        self.collect_fields_recursive(doc, kid_id, &field_name, result);
                    }
                }
            }
        }
    }

    // ─── XFA Implementation ──────────────────────────────────────

    /// Fill XFA form fields by parsing the XDP XML, replacing values, and writing back.
    fn fill_xfa_fields(
        &self,
        doc: &mut Document,
        acro_form_ref: ObjectId,
        fields: &HashMap<String, String>,
    ) -> PdfResult<usize> {
        let xfa_data = self.extract_xfa_xml(doc, acro_form_ref)?;

        // Track which fields were actually found/modified
        let mut filled_count = 0;

        // Use the original approach: try quick-xml first, fall back to text replacement
        let modified_xml = match self.replace_xfa_values_xml(&xfa_data, fields, &mut filled_count) {
            Ok(xml) => {
                if filled_count > 0 {
                    xml
                } else {
                    // quick-xml didn't find any matching fields, try text-based approach
                    self.replace_xfa_values(&xfa_data, fields, &mut filled_count)?
                }
            }
            Err(_e) => {
                // Fall back to text-based replacement
                self.replace_xfa_values(&xfa_data, fields, &mut filled_count)?
            }
        };

        if filled_count == 0 {
            return Err("No matching XFA fields found".to_string());
        }

        self.write_xfa_xml(doc, acro_form_ref, &modified_xml)?;

        Ok(filled_count)
    }

    /// Extract the XDP XML content from the /AcroForm/XFA entry.
    fn extract_xfa_xml(&self, doc: &Document, acro_form_ref: ObjectId) -> PdfResult<String> {
        let xfa_obj = {
            let acro_form_dict = doc
                .get_dictionary(acro_form_ref)
                .map_err(|e| format!("Failed to get AcroForm dictionary: {}", e))?;
            acro_form_dict
                .get(b"XFA")
                .map_err(|_| "No XFA entry in AcroForm".to_string())?
                .clone()
        };

        match xfa_obj {
            Object::Stream(stream) => {
                let content = stream
                    .get_plain_content()
                    .map_err(|e| format!("Failed to read XFA stream: {}", e))?;
                String::from_utf8(content)
                    .map_err(|e| format!("XFA stream is not valid UTF-8: {}", e))
            }
            Object::Reference(id) => {
                let obj = doc
                    .get_object(id)
                    .map_err(|e| format!("Failed to get XFA object: {}", e))?;
                match obj {
                    Object::Stream(stream) => {
                        let content = stream
                            .get_plain_content()
                            .map_err(|e| format!("Failed to read XFA stream: {}", e))?;
                        String::from_utf8(content)
                            .map_err(|e| format!("XFA stream is not valid UTF-8: {}", e))
                    }
                    _ => Err("XFA reference does not point to a stream".to_string()),
                }
            }
            Object::Array(arr) => {
                let mut combined = String::new();
                for item in arr.iter() {
                    if let Ok(stream) = item
                        .as_reference()
                        .and_then(|id| doc.get_object(id).and_then(Object::as_stream))
                    {
                        let content = stream
                            .get_plain_content()
                            .map_err(|e| format!("Failed to read XFA sub-stream: {}", e))?;
                        let text = String::from_utf8(content)
                            .map_err(|e| format!("XFA sub-stream is not valid UTF-8: {}", e))?;
                        combined.push_str(&text);
                    } else if let Object::Stream(stream) = item {
                        let content = stream
                            .get_plain_content()
                            .map_err(|e| format!("Failed to read XFA sub-stream: {}", e))?;
                        let text = String::from_utf8(content)
                            .map_err(|e| format!("XFA sub-stream is not valid UTF-8: {}", e))?;
                        combined.push_str(&text);
                    }
                }
                Ok(combined)
            }
            _ => Err("Unexpected XFA entry type".to_string()),
        }
    }

    /// Replace XFA field values using quick-xml for proper XML-aware parsing.
    /// Only modifies text content within the <xfa:data> section.
    fn replace_xfa_values_xml(
        &self,
        xml: &str,
        fields: &HashMap<String, String>,
        filled_count: &mut usize,
    ) -> PdfResult<String> {
        use quick_xml::events::{BytesText, Event};
        use quick_xml::Reader;
        use quick_xml::Writer;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut writer = Writer::new(Vec::new());

        #[derive(PartialEq)]
        enum State {
            Outside,
            InDatasets,
            InData,
        }

        let mut state = State::Outside;
        let mut data_depth: u32 = 0;
        let mut current_field_name: Option<String> = None;
        let mut replaced_in_current_tag = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let local_name = e.local_name().as_ref().to_vec();
                    let local_str = String::from_utf8_lossy(&local_name).to_string();

                    // Detect datasets container (e.g., <xfa:datasets> or <datasets>)
                    if local_str == "datasets" {
                        state = State::InDatasets;
                    }

                    // Detect <xfa:data> / <data> within datasets
                    if state == State::InDatasets && local_str == "data" {
                        state = State::InData;
                        data_depth = 1;
                    } else if state == State::InData {
                        data_depth += 1;
                        // Check if this element's local name matches a field key
                        if fields.contains_key(&local_str) {
                            current_field_name = Some(local_str.clone());
                        }
                        // Also check if a dotted path from parent matches
                        // (e.g., "Parent.Child" in fields map)
                    }

                    writer.write_event(Event::Start(e.into_owned())).unwrap();
                    replaced_in_current_tag = false;
                }

                Ok(Event::Text(e)) => {
                    if let Some(ref field) = current_field_name {
                        if !replaced_in_current_tag {
                            if let Some(value) = fields.get(field) {
                                let encoded = value
                                    .replace('&', "&amp;")
                                    .replace('<', "&lt;")
                                    .replace('>', "&gt;")
                                    .replace('"', "&quot;")
                                    .replace('\'', "&apos;");
                                writer
                                    .write_event(Event::Text(BytesText::new(&encoded)))
                                    .unwrap();
                                *filled_count += 1;
                                replaced_in_current_tag = true;
                                continue;
                            }
                        }
                    }
                    writer.write_event(Event::Text(e)).unwrap();
                }

                Ok(Event::CData(e)) => {
                    if let Some(ref field) = current_field_name {
                        if !replaced_in_current_tag {
                            if let Some(value) = fields.get(field) {
                                writer
                                    .write_event(Event::Text(BytesText::new(value)))
                                    .unwrap();
                                *filled_count += 1;
                                replaced_in_current_tag = true;
                                continue;
                            }
                        }
                    }
                    writer.write_event(Event::CData(e)).unwrap();
                }

                Ok(Event::End(e)) => {
                    let local_name = e.local_name().as_ref().to_vec();
                    let local_str = String::from_utf8_lossy(&local_name).to_string();

                    if state == State::InData {
                        data_depth = data_depth.saturating_sub(1);
                        if data_depth == 0 {
                            // Exiting <data> back into datasets
                            state = State::InDatasets;
                        }
                    } else if state == State::InDatasets && local_str == "datasets" {
                        state = State::Outside;
                    }

                    current_field_name = None;
                    writer.write_event(Event::End(e)).unwrap();
                }

                Ok(Event::Eof) => break,

                Ok(e) => {
                    writer.write_event(e).unwrap();
                }

                Err(e) => {
                    return Err(format!("XML parse error: {}", e));
                }
            }
        }

        String::from_utf8(writer.into_inner())
            .map_err(|e| format!("XML serialization error: {}", e))
    }

    /// Replace XFA field values in the XDP XML using text-based pattern matching.
    /// Only used as fallback when quick-xml approach fails to find matching fields.
    fn replace_xfa_values(
        &self,
        xml: &str,
        fields: &HashMap<String, String>,
        filled_count: &mut usize,
    ) -> PdfResult<String> {
        let mut result = xml.to_string();

        for (field_name, value) in fields {
            let encoded_value = value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;");

            let patterns = vec![
                format!("<{}>", field_name),
                format!("<{} ", field_name),
                format!("<xfa:{}>", field_name),
                format!("<xfa:{} ", field_name),
            ];

            for pattern in &patterns {
                let mut search_start = 0;
                let mut found = false;
                while let Some(tag_start) = result[search_start..].find(pattern.as_str()) {
                    let abs_tag_start = search_start + tag_start;

                    let opening_end = if pattern.ends_with(' ') {
                        match result[abs_tag_start..].find('>') {
                            Some(pos) => abs_tag_start + pos + 1,
                            None => break,
                        }
                    } else {
                        abs_tag_start + pattern.len()
                    };

                    // Skip self-closing tags
                    if abs_tag_start > 0 && result.as_bytes()[abs_tag_start - 1] == b'/' {
                        search_start = opening_end;
                        continue;
                    }

                    // Extract element name from opening tag for closing tag match
                    let elem_name_start = pattern
                        .trim_end_matches(' ')
                        .trim_end_matches('>')
                        .trim_start_matches('<');
                    let close_tag = format!("</{}>", elem_name_start);

                    if let Some(closing_start) = result[opening_end..].find(&close_tag) {
                        let abs_closing_start = opening_end + closing_start;

                        let mut new_result = String::with_capacity(
                            result.len() - (abs_closing_start - opening_end) + encoded_value.len(),
                        );
                        new_result.push_str(&result[..opening_end]);
                        new_result.push_str(&encoded_value);
                        new_result.push_str(&result[abs_closing_start..]);
                        result = new_result;
                        found = true;
                    }
                    break;
                }
                if found {
                    *filled_count += 1;
                    break;
                }
            }
        }

        Ok(result)
    }

    /// Write modified XDP XML back to the document's AcroForm XFA entry.
    fn write_xfa_xml(
        &self,
        doc: &mut Document,
        acro_form_ref: ObjectId,
        xml: &str,
    ) -> PdfResult<()> {
        let xml_bytes = xml.as_bytes().to_vec();
        let xfa_stream_id = doc.add_object(Stream::new(dictionary! {}, xml_bytes));

        let acro_form_dict = doc
            .get_dictionary_mut(acro_form_ref)
            .map_err(|e| format!("Failed to get AcroForm dictionary: {}", e))?;
        acro_form_dict.set("XFA", Object::Reference(xfa_stream_id));

        Ok(())
    }

    /// Collect field names from XFA XDP XML.
    fn collect_xfa_fields(
        &self,
        doc: &Document,
        acro_form_ref: ObjectId,
        result: &mut Vec<FieldInfo>,
    ) -> PdfResult<()> {
        let xml = match self.extract_xfa_xml(doc, acro_form_ref) {
            Ok(xml) => xml,
            Err(_) => return Ok(()),
        };

        // Try quick-xml based extraction first
        if self.collect_xfa_fields_xml(&xml, result) {
            return Ok(());
        }

        // Fall back to text-based extraction
        self.collect_xfa_fields_text(&xml, result);
        Ok(())
    }

    /// Collect XFA fields using quick-xml parsing.
    /// Returns true if successful, false if parsing failed.
    fn collect_xfa_fields_xml(&self, xml: &str, result: &mut Vec<FieldInfo>) -> bool {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        #[derive(PartialEq)]
        enum State {
            Outside,
            InDatasets,
            InData,
        }

        let mut state = State::Outside;
        let mut data_depth: u32 = 0;
        let mut field_name: Option<String> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                    if local_name == "datasets" {
                        state = State::InDatasets;
                    }
                    if state == State::InDatasets && local_name == "data" {
                        state = State::InData;
                        data_depth = 1;
                    } else if state == State::InData {
                        data_depth += 1;
                        // Name is set by the subsequent Text event
                        field_name = Some(local_name);
                    }
                }

                Ok(Event::Text(e)) => {
                    if state == State::InData && data_depth > 1 {
                        if let Some(ref name) = field_name {
                            let text = e.unescape().unwrap_or_default().to_string();
                            if !name.starts_with("xfa:") && !text.trim().is_empty() {
                                result.push(FieldInfo {
                                    name: name.clone(),
                                    field_type: "XFA".to_string(),
                                    current_value: Some(text),
                                });
                            }
                        }
                    }
                    field_name = None;
                }

                Ok(Event::End(e)) => {
                    let local_name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();

                    if state == State::InData {
                        data_depth = data_depth.saturating_sub(1);
                        if data_depth <= 1 {
                            if local_name == "data" {
                                state = State::InDatasets;
                            } else {
                                // Back to data container level
                                data_depth = 1;
                            }
                        }
                    } else if state == State::InDatasets && local_name == "datasets" {
                        state = State::Outside;
                    }
                    field_name = None;
                }

                Ok(Event::Eof) => break,
                Err(_) => return false,
                _ => {}
            }
        }

        true
    }

    /// Text-based fallback for XFA field extraction.
    fn collect_xfa_fields_text(&self, xml: &str, result: &mut Vec<FieldInfo>) {
        let mut pos = 0;
        let bytes = xml.as_bytes();

        while pos < bytes.len() {
            if bytes[pos] != b'<' || (pos + 1 < bytes.len() && bytes[pos + 1] == b'/') {
                pos += 1;
                continue;
            }

            let tag_start = pos + 1;
            let mut tag_end = tag_start;
            while tag_end < bytes.len()
                && bytes[tag_end] != b'>'
                && bytes[tag_end] != b' '
                && bytes[tag_end] != b'/'
            {
                tag_end += 1;
            }

            if tag_end <= tag_start {
                pos += 1;
                continue;
            }

            let tag_name = &xml[tag_start..tag_end];

            if tag_name.starts_with('?')
                || tag_name == "xdp:xdp"
                || tag_name == "xfa:datasets"
                || tag_name == "xfa:data"
            {
                let close = match xml[pos..].find('>') {
                    Some(p) => pos + p + 1,
                    None => break,
                };
                pos = close;
                continue;
            }

            if tag_name.contains(':') && !tag_name.starts_with("xfa:") {
                pos += 1;
                continue;
            }

            let clean_name = if let Some(idx) = tag_name.find(':') {
                &tag_name[idx + 1..]
            } else {
                tag_name
            };

            let close_tag = format!("</{}>", tag_name);
            let rest = &xml[pos..];
            if let Some(closing_start) = rest.find(&close_tag) {
                let tag_end_pos = match xml[pos..].find('>') {
                    Some(p) => pos + p,
                    None => break,
                };
                if tag_end_pos > 0 && xml.as_bytes()[tag_end_pos - 1] == b'/' {
                    pos = tag_end_pos + 1;
                    continue;
                }

                let content_start = tag_end_pos + 1;
                let content_end = pos + closing_start;

                let value = if content_end > content_start {
                    Some(xml[content_start..content_end].to_string())
                } else {
                    None
                };

                if !clean_name.is_empty()
                    && clean_name != "topSubform"
                    && clean_name != "topmostSubform"
                    && !clean_name.starts_with("Page")
                    && tag_name != "xfa:data"
                    && tag_name != "xfa:datasets"
                {
                    let display_value = value.as_ref().map(|v| {
                        v.replace("&amp;", "&")
                            .replace("&lt;", "<")
                            .replace("&gt;", ">")
                            .replace("&quot;", "\"")
                            .replace("&apos;", "'")
                    });
                    result.push(FieldInfo {
                        name: clean_name.to_string(),
                        field_type: "XFA".to_string(),
                        current_value: display_value,
                    });
                }

                pos = content_end + close_tag.len();
            } else {
                pos += 1;
            }
        }
    }
}

// ─── Flat PDF (Non-Form) Text Overlay ────────────────────────────────

/// A single text field to overlay on a flat PDF page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFieldOverlay {
    pub page: u32,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    pub text: String,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default = "default_font_name")]
    pub font_name: String,
}

fn default_font_size() -> f64 {
    11.0
}

fn default_font_name() -> String {
    "Helvetica".to_string()
}

/// Structured result for flat PDF fill operations.
#[derive(Debug, Clone, Serialize)]
pub struct FlatFillResult {
    pub status: String,
    pub inserted_text_count: usize,
    pub output_path: String,
    pub pages_modified: Vec<u32>,
}

/// Fills text into PDFs that don't have form fields (flat PDFs).
///
/// Uses content stream appending (`add_to_page_content`) to overlay
/// text at specified (x, y) coordinates without modifying existing content.
/// Supports standard PDF fonts (Helvetica, Times-Roman, Courier) that
/// require no font embedding.
pub struct FlatPdfFiller;

impl FlatPdfFiller {
    pub fn new() -> Self {
        Self
    }

    /// Fill text into a flat PDF at specified positions.
    pub fn fill_flat_pdf(
        &self,
        input_path: &str,
        output_path: &str,
        fields: &[TextFieldOverlay],
    ) -> PdfResult<String> {
        let mut doc =
            Document::load(input_path).map_err(|e| format!("Failed to load PDF: {}", e))?;

        PdfFormFiller::try_decrypt(&mut doc);
        PdfFormFiller::resolve_object_streams(&mut doc);

        let mut inserted_count = 0;
        let mut pages_modified: Vec<u32> = Vec::new();

        // Group fields by page for efficient processing
        let mut fields_by_page: std::collections::BTreeMap<u32, Vec<&TextFieldOverlay>> =
            std::collections::BTreeMap::new();
        for field in fields {
            fields_by_page.entry(field.page).or_default().push(field);
        }

        for (&page_num, page_fields) in &fields_by_page {
            for field in page_fields {
                self.insert_text(
                    &mut doc,
                    page_num,
                    &field.text,
                    field.x as f32,
                    field.y as f32,
                    field.font_size as f32,
                    &field.font_name,
                )?;
                inserted_count += 1;
            }
            pages_modified.push(page_num);
        }

        if inserted_count == 0 {
            return Err("No fields provided".to_string());
        }

        doc.save(output_path)
            .map_err(|e| format!("Failed to save PDF: {}", e))?;

        let result = FlatFillResult {
            status: "filled".to_string(),
            inserted_text_count: inserted_count,
            output_path: output_path.to_string(),
            pages_modified,
        };

        serde_json::to_string_pretty(&result).map_err(|e| format!("Serialization error: {}", e))
    }

    /// Insert a single text string at (x, y) on a specific page.
    /// Uses content stream append to overlay text on existing page content.
    #[allow(clippy::too_many_arguments)]
    fn insert_text(
        &self,
        doc: &mut Document,
        page_num: u32,
        text: &str,
        x: f32,
        y: f32,
        font_size: f32,
        font_name: &str,
    ) -> PdfResult<()> {
        let page_id = self.resolve_page_id(doc, page_num)?;

        let font_key = self.ensure_font_in_page_resources(doc, page_id, font_name)?;

        use lopdf::content::{Content, Operation};

        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new("BT", vec![]),
                Operation::new(
                    "Tf",
                    vec![Object::Name(font_key.as_bytes().to_vec()), font_size.into()],
                ),
                Operation::new("Td", vec![x.into(), y.into()]),
                Operation::new("Tj", vec![Object::string_literal(text.as_bytes())]),
                Operation::new("ET", vec![]),
                Operation::new("Q", vec![]),
            ],
        };

        doc.add_to_page_content(page_id, content)
            .map_err(|e| format!("Failed to add content to page {}: {}", page_num, e))?;

        Ok(())
    }

    /// Ensure a font is registered in the page's /Resources/Font dictionary.
    /// Returns the font key (e.g., "F1") to use in Tf operator.
    fn ensure_font_in_page_resources(
        &self,
        doc: &mut Document,
        page_id: ObjectId,
        font_name: &str,
    ) -> PdfResult<String> {
        let resources_id = self.get_or_create_page_resources(doc, page_id)?;

        let font_dict_id = {
            let res_dict = doc
                .get_dictionary(resources_id)
                .map_err(|e| format!("Failed to get Resources dict: {}", e))?;

            match res_dict.get(b"Font") {
                Ok(Object::Reference(id)) => *id,
                Ok(Object::Dictionary(dict)) => {
                    let new_id = doc.add_object(Object::Dictionary(dict.clone()));
                    let res_mut = doc
                        .get_dictionary_mut(resources_id)
                        .map_err(|e| format!("Failed to get Resources dict: {}", e))?;
                    res_mut.set("Font", Object::Reference(new_id));
                    new_id
                }
                _ => {
                    let new_id = doc.add_object(dictionary! {});
                    let res_mut = doc
                        .get_dictionary_mut(resources_id)
                        .map_err(|e| format!("Failed to get Resources dict: {}", e))?;
                    res_mut.set("Font", Object::Reference(new_id));
                    new_id
                }
            }
        };

        let font_dict = doc
            .get_dictionary(font_dict_id)
            .map_err(|e| format!("Failed to get Font dict: {}", e))?;

        for (key, value) in font_dict.iter() {
            if let Ok(font_obj_id) = value.as_reference() {
                if let Ok(font_obj) = doc.get_dictionary(font_obj_id) {
                    if let Ok(base) = font_obj.get(b"BaseFont").and_then(Object::as_name) {
                        if base == font_name.as_bytes() {
                            return Ok(String::from_utf8_lossy(key).to_string());
                        }
                    }
                }
            }
        }

        let next_key = {
            let current_font_dict = doc
                .get_dictionary(font_dict_id)
                .map_err(|e| format!("Failed to get Font dict: {}", e))?;
            let mut max_n = 0u32;
            for (key, _) in current_font_dict.iter() {
                let key_str = String::from_utf8_lossy(key).to_string();
                if let Some(rest) = key_str.strip_prefix('F') {
                    if let Ok(n) = rest.parse::<u32>() {
                        if n > max_n {
                            max_n = n;
                        }
                    }
                }
            }
            format!("F{}", max_n + 1)
        };

        let new_font_id = doc.add_object(dictionary! {
            "Type" => "Font",
            "Subtype" => "Type1",
            "BaseFont" => Object::Name(font_name.as_bytes().to_vec()),
        });

        let font_dict_mut = doc
            .get_dictionary_mut(font_dict_id)
            .map_err(|e| format!("Failed to get Font dict: {}", e))?;
        font_dict_mut.set(next_key.as_bytes(), Object::Reference(new_font_id));

        Ok(next_key)
    }

    /// Get or create the page's /Resources dictionary as a reference.
    fn get_or_create_page_resources(
        &self,
        doc: &mut Document,
        page_id: ObjectId,
    ) -> PdfResult<ObjectId> {
        let page_dict = doc
            .get_dictionary(page_id)
            .map_err(|e| format!("Failed to get page dictionary: {}", e))?;

        match page_dict.get(b"Resources") {
            Ok(Object::Reference(id)) => Ok(*id),
            Ok(Object::Dictionary(dict)) => {
                let new_id = doc.add_object(Object::Dictionary(dict.clone()));
                let page_mut = doc
                    .get_dictionary_mut(page_id)
                    .map_err(|e| format!("Failed to get page dict: {}", e))?;
                page_mut.set("Resources", Object::Reference(new_id));
                Ok(new_id)
            }
            _ => {
                let new_id = doc.add_object(dictionary! {
                    "Font" => dictionary! {},
                });
                let page_mut = doc
                    .get_dictionary_mut(page_id)
                    .map_err(|e| format!("Failed to get page dict: {}", e))?;
                page_mut.set("Resources", Object::Reference(new_id));
                Ok(new_id)
            }
        }
    }

    /// Resolve a 1-indexed page number to a lopdf ObjectId.
    /// Tries `get_pages()` first, then brute-force scans all objects for /Type /Page
    /// (needed for some encrypted PDFs where lopdf's page tree walk fails).
    fn resolve_page_id(&self, doc: &Document, page_num: u32) -> PdfResult<ObjectId> {
        let pages = doc.get_pages();
        if let Some(&id) = pages.get(&page_num) {
            return Ok(id);
        }

        let page_ids: Vec<ObjectId> = doc
            .objects
            .iter()
            .filter(|(_, obj)| {
                obj.as_dict()
                    .ok()
                    .and_then(|d| d.get(b"Type").ok())
                    .and_then(|o| o.as_name().ok())
                    .map(|n| n == b"Page")
                    .unwrap_or(false)
            })
            .map(|(&id, _)| id)
            .collect();

        if !page_ids.is_empty() {
            if page_num <= page_ids.len() as u32 {
                let mut sorted = page_ids;
                sorted.sort_by_key(|(obj_num, _)| *obj_num);
                let idx = if page_num == 0 { 0usize } else { (page_num - 1) as usize };
                return Ok(sorted[idx]);
            }
            return Err(format!(
                "Page {} not found (document has {} pages)",
                page_num,
                page_ids.len()
            ));
        }

        Err(format!(
            "Page {} not found in document (no pages detected)",
            page_num
        ))
    }
}

impl Default for PdfFormFiller {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for FlatPdfFiller {
    fn default() -> Self {
        Self::new()
    }
}

fn map_font_name(da_name: &str) -> String {
    match da_name {
        "Helv" | "Helvetica" | "HeBo" => "Helvetica".to_string(),
        "TiRo" | "TimesNewRoman" | "Times-Roman" => "Times-Roman".to_string(),
        "Cour" | "Courier" => "Courier".to_string(),
        "ZaDb" => "ZapfDingbats".to_string(),
        "Symb" => "Symbol".to_string(),
        other => other.to_string(),
    }
}

// ─── PDF Text Extraction ──────────────────────────────────────────

/// Extract plain text content from a PDF document.
/// Uses lopdf's built-in text extraction across all pages.
pub fn read_pdf_text(input_path: &str) -> Result<String, String> {
    let mut doc = Document::load(input_path).map_err(|e| format!("Failed to load PDF: {}", e))?;

    PdfFormFiller::try_decrypt(&mut doc);
    PdfFormFiller::resolve_object_streams(&mut doc);

    let page_count = doc.get_pages().len() as u32;
    if page_count == 0 {
        return Ok(String::new());
    }

    let page_numbers: Vec<u32> = (1..=page_count).collect();
    doc.extract_text(&page_numbers)
        .map_err(|e| format!("Failed to extract PDF text: {}", e))
}

/// Extract text from a PDF as structured chunks (one per page).
pub fn read_pdf_chunks(input_path: &str) -> Result<Vec<serde_json::Value>, String> {
    let mut doc = Document::load(input_path).map_err(|e| format!("Failed to load PDF: {}", e))?;

    PdfFormFiller::try_decrypt(&mut doc);
    PdfFormFiller::resolve_object_streams(&mut doc);

    let page_count = doc.get_pages().len() as u32;
    if page_count == 0 {
        return Ok(Vec::new());
    }

    let page_numbers: Vec<u32> = (1..=page_count).collect();
    let chunks = doc.extract_text_chunks(&page_numbers);

    let mut result = Vec::new();
    let mut page_num = 0u32;
    let mut current_page_text = String::new();

    for chunk in chunks {
        match chunk {
            Ok(text) => {
                if !current_page_text.is_empty() && text.contains('\n') {
                    // New page separator detected
                    page_num += 1;
                    result.push(serde_json::json!({
                        "page": page_num,
                        "text": current_page_text.trim(),
                    }));
                    current_page_text = String::new();
                }
                current_page_text.push_str(&text);
            }
            Err(_) => continue,
        }
    }

    if !current_page_text.is_empty() {
        page_num += 1;
        result.push(serde_json::json!({
            "page": page_num,
            "text": current_page_text.trim(),
        }));
    }

    // If we got no page separation, fall back to single page
    if result.is_empty() && !current_page_text.is_empty() {
        result.push(serde_json::json!({
            "page": 1,
            "text": current_page_text.trim(),
        }));
    }

    Ok(result)
}

/// Extract PDF text as markdown (simple formatting — one page per section).
pub fn read_pdf_to_md(input_path: &str) -> Result<String, String> {
    let text = read_pdf_text(input_path)?;
    Ok(text)
}

/// Extract PDF content as structured JSON: form fields + per-page text.
pub fn read_pdf_json(input_path: &str) -> Result<String, String> {
    let mut doc = Document::load(input_path).map_err(|e| format!("Failed to load PDF: {}", e))?;

    PdfFormFiller::try_decrypt(&mut doc);
    PdfFormFiller::resolve_object_streams(&mut doc);

    let page_count = doc.get_pages().len() as u32;

    let filler = PdfFormFiller::new();
    let fields_json: Vec<serde_json::Value> = {
        let mut fields = Vec::new();
        if let Ok(catalog) = doc.catalog() {
            if let Ok(acro_form) = catalog.get(b"AcroForm") {
                let (ref_id, has_xfa, fields_arr) = match acro_form {
                    Object::Reference(id) => {
                        let dict = doc.get_dictionary(*id).ok();
                        let has_xfa = dict.map(|d| d.has(b"XFA")).unwrap_or(false);
                        let fields_arr = dict
                            .and_then(|d| d.get(b"Fields").ok())
                            .and_then(|f| f.as_array().ok().cloned());
                        (*id, has_xfa, fields_arr)
                    }
                    Object::Dictionary(dict) => {
                        let has_xfa = dict.has(b"XFA");
                        let fields_arr = dict
                            .get(b"Fields")
                            .ok()
                            .and_then(|f| f.as_array().ok().cloned());
                        ((0, 0), has_xfa, fields_arr)
                    }
                    _ => ((0, 0), false, None),
                };

                if let Some(ref fields_arr) = fields_arr {
                    for field_ref in fields_arr.iter() {
                        if let Ok(field_id) = field_ref.as_reference() {
                            collect_field_info_recursive(&doc, field_id, "", &mut fields);
                        }
                    }
                }

                if has_xfa && ref_id != (0, 0) {
                    if let Ok(xml_str) = filler.extract_xfa_xml(&doc, ref_id) {
                        if !filler.collect_xfa_fields_xml(&xml_str, &mut fields) {
                            filler.collect_xfa_fields_text(&xml_str, &mut fields);
                        }
                    }
                }
            }
        }

        fields
            .into_iter()
            .map(|f: FieldInfo| {
                serde_json::json!({
                    "name": f.name,
                    "type": f.field_type,
                    "value": f.current_value,
                })
            })
            .collect()
    };

    let page_numbers: Vec<u32> = if page_count > 0 {
        (1..=page_count).collect()
    } else {
        Vec::new()
    };

    let content: Vec<serde_json::Value> = if !page_numbers.is_empty() {
        let chunks = doc.extract_text_chunks(&page_numbers);
        let mut pages = Vec::new();
        let mut current_text = String::new();
        let mut current_page = 1u32;

        for chunk in chunks {
            match chunk {
                Ok(text) => {
                    if !current_text.is_empty() && text.contains('\n') {
                        pages.push(serde_json::json!({
                            "page": current_page,
                            "text": current_text.trim(),
                        }));
                        current_page += 1;
                        current_text = String::new();
                    }
                    current_text.push_str(&text);
                }
                Err(_) => continue,
            }
        }
        if !current_text.is_empty() {
            pages.push(serde_json::json!({
                "page": current_page,
                "text": current_text.trim(),
            }));
        }
        if pages.is_empty() && !current_text.is_empty() {
            pages.push(serde_json::json!({
                "page": 1,
                "text": current_text.trim(),
            }));
        }
        pages
    } else {
        Vec::new()
    };

    let result = serde_json::json!({
        "format": "pdf",
        "page_count": page_count,
        "form_fields": fields_json,
        "content": content,
    });

    serde_json::to_string_pretty(&result).map_err(|e| format!("Serialization error: {}", e))
}

fn collect_field_info_recursive(
    doc: &Document,
    field_id: ObjectId,
    parent_name: &str,
    result: &mut Vec<FieldInfo>,
) {
    if let Ok(field_dict) = doc.get_dictionary(field_id) {
        let field_name = match field_dict.get(b"T").and_then(Object::as_str) {
            Ok(name) => {
                let name_str = decode_pdf_string(name);
                if parent_name.is_empty() {
                    name_str.to_string()
                } else {
                    format!("{}.{}", parent_name, name_str)
                }
            }
            Err(_) => {
                if field_dict.has(b"Kids") {
                    if let Ok(kids) = field_dict.get(b"Kids").and_then(|k| k.as_array()) {
                        for kid_ref in kids.iter() {
                            if let Ok(kid_id) = kid_ref.as_reference() {
                                collect_field_info_recursive(doc, kid_id, parent_name, result);
                            }
                        }
                    }
                }
                return;
            }
        };

        let field_type = match field_dict.get(b"FT") {
            Ok(Object::Name(name)) => String::from_utf8_lossy(name).to_string(),
            _ => "Unknown".to_string(),
        };

        let current_value = match field_dict.get(b"V") {
            Ok(Object::String(s, _)) => Some(String::from_utf8_lossy(s).to_string()),
            Ok(Object::Name(n)) => Some(String::from_utf8_lossy(n).to_string()),
            _ => None,
        };

        result.push(FieldInfo {
            name: field_name,
            field_type,
            current_value,
        });

        if let Ok(kids) = field_dict.get(b"Kids").and_then(Object::as_array) {
            for kid_ref in kids.iter() {
                if let Ok(kid_id) = kid_ref.as_reference() {
                    collect_field_info_recursive(doc, kid_id, parent_name, result);
                }
            }
        }
    }
}
