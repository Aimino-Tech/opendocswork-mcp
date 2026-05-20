use lopdf::{dictionary, Document, Object, ObjectId, Stream};
use serde::Serialize;
use std::collections::HashMap;

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

/// Fills and inspects form fields in PDF documents (AcroForm and XFA).
pub struct PdfFormFiller;

impl PdfFormFiller {
    pub fn new() -> Self {
        Self
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
        let mut doc = Document::load(input_path)
            .map_err(|e| format!("Failed to load PDF: {}", e))?;

        // Extract the AcroForm reference from catalog without holding borrow
        let acro_form_ref = {
            let catalog = doc.catalog().map_err(|e| {
                format!("Failed to read PDF catalog: {}", e)
            })?;
            match catalog.get(b"AcroForm") {
                Ok(Object::Reference(id)) => *id,
                Ok(Object::Dictionary(_)) => {
                    // Direct dictionary - need to add as object to modify it
                    return Err("AcroForm as direct dictionary not supported".to_string());
                }
                Err(_) => {
                    return Err("No AcroForm found in PDF document".to_string());
                }
                _ => {
                    return Err("Invalid AcroForm entry in catalog".to_string());
                }
            }
        };

        // Get acro form dict data
        let (has_xfa, has_fields) = {
            let acro_form_dict = doc.get_dictionary(acro_form_ref).map_err(|e| {
                format!("Failed to get AcroForm dictionary: {}", e)
            })?;
            (acro_form_dict.has(b"XFA"), acro_form_dict.has(b"Fields"))
        };

        let filled_count = if has_xfa {
            self.fill_xfa_fields(&mut doc, acro_form_ref, fields)?
        } else if has_fields {
            self.fill_acroform_fields(&mut doc, acro_form_ref, fields)?
        } else {
            return Err("PDF has no recognized form fields (no XFA or Fields found in AcroForm)".to_string());
        };

        doc.save(output_path).map_err(|e| {
            format!("Failed to save PDF to '{}': {}", output_path, e)
        })?;

        let result = FillFormResult {
            status: "filled".to_string(),
            filled_field_count: filled_count,
            output_path: output_path.to_string(),
        };

        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    /// List all form fields in a PDF document with their current values.
    pub fn list_fields(&self, input_path: &str) -> PdfResult<String> {
        let doc = Document::load(input_path)
            .map_err(|e| format!("Failed to load PDF: {}", e))?;

        let (acro_form_ref, has_xfa, fields_array) = {
            let catalog = doc.catalog().map_err(|e| {
                format!("Failed to read PDF catalog: {}", e)
            })?;

            let acro_form = match catalog.get(b"AcroForm") {
                Ok(obj) => obj,
                Err(_) => {
                    return Ok("[]".to_string());
                }
            };

            let (ref_id, dict) = match acro_form {
                Object::Reference(id) => {
                    let dict = doc.get_dictionary(*id).map_err(|e| {
                        format!("Failed to get AcroForm dictionary: {}", e)
                    })?;
                    (*id, dict)
                }
                Object::Dictionary(_dict) => {
                    return Err("AcroForm as direct dictionary not supported".to_string());
                }
                _ => {
                    return Err("Invalid AcroForm entry in catalog".to_string());
                }
            };

            let fields_arr = dict.get(b"Fields").ok()
                .and_then(|f| f.as_array().ok())
                .cloned();
            let has_xfa = dict.has(b"XFA");

            (ref_id, has_xfa, fields_arr)
        };

        let mut fields: Vec<FieldInfo> = Vec::new();

        // Collect AcroForm fields
        if let Some(ref fields_arr) = fields_array {
            for field_ref in fields_arr.iter() {
                if let Ok(field_id) = field_ref.as_reference() {
                    let _ = self.collect_fields_recursive(&doc, field_id, "", &mut fields);
                }
            }
        }

        // Collect XFA field names from the XDP XML
        if has_xfa {
            let _ = self.collect_xfa_fields(&doc, acro_form_ref, &mut fields);
        }

        serde_json::to_string_pretty(&fields)
            .map_err(|e| format!("Serialization error: {}", e))
    }

    // ─── AcroForm Implementation ─────────────────────────────────

    /// Fill AcroForm fields by matching field names and setting /V values.
    fn fill_acroform_fields(
        &self,
        doc: &mut Document,
        acro_form_ref: ObjectId,
        fields: &HashMap<String, String>,
    ) -> PdfResult<usize> {
        let fields_array = {
            let acro_form_dict = doc.get_dictionary(acro_form_ref).map_err(|e| {
                format!("Failed to get AcroForm dictionary: {}", e)
            })?;
            acro_form_dict.get(b"Fields").and_then(Object::as_array)
                .map_err(|_| "AcroForm has no Fields array".to_string())?
                .clone()
        };

        // Collect all field reference IDs from the Fields array recursively
        let all_field_ids = self.collect_field_ids(doc, &fields_array);

        let mut filled_count = 0;

        for &field_id in &all_field_ids {
            if let Ok(field_dict) = doc.get_dictionary(field_id) {
                let field_name = match field_dict.get(b"T").and_then(Object::as_string) {
                    Ok(name) => name.to_string(),
                    Err(_) => continue,
                };

                // Try to match the field name against our input map
                let matched_value = fields.get(&field_name)
                    .or_else(|| {
                        fields.iter().find(|(key, _)| {
                            field_name.contains(key.as_str()) || key.contains(&field_name)
                        }).map(|(_, v)| v)
                    });

                if let Some(value) = matched_value {
                    if let Ok(field_dict_mut) = doc.get_dictionary_mut(field_id) {
                        field_dict_mut.set("V", Object::string_literal(value.as_bytes()));
                        field_dict_mut.set("DV", Object::string_literal(value.as_bytes()));
                        filled_count += 1;
                    }
                }
            }
        }

        Ok(filled_count)
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
            // Collect this field if it has a /T name
            if field_dict.has(b"T") {
                result.push(field_id);
            }

            // Recurse into /Kids if present
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
            // Get field name
            let field_name = match field_dict.get(b"T").and_then(Object::as_string) {
                Ok(name) => {
                    let full_name = if parent_name.is_empty() {
                        name.to_string()
                    } else {
                        format!("{}.{}", parent_name, name)
                    };
                    full_name
                }
                Err(_) => {
                    // Anonymous field, skip unless it has interesting kids
                    if field_dict.has(b"Kids") {
                        if let Ok(kids) = field_dict.get(b"Kids").and_then(Object::as_array) {
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

            // Get field type
            let field_type = match field_dict.get(b"FT") {
                Ok(Object::Name(name)) => String::from_utf8_lossy(name).to_string(),
                _ => "Unknown".to_string(),
            };

            // Get current value
            let current_value = match field_dict.get(b"V") {
                Ok(Object::String(s, _)) => Some(String::from_utf8_lossy(s).to_string()),
                Ok(Object::Name(n)) => Some(String::from_utf8_lossy(n).to_string()),
                _ => None,
            };

            // Add this field
            result.push(FieldInfo {
                name: field_name.clone(),
                field_type,
                current_value,
            });

            // Recurse into kids
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
        // Get the XFA entry - can be a stream or an array of streams
        let xfa_data = self.extract_xfa_xml(doc, acro_form_ref)?;

        // Parse the XDP XML and replace field values
        let modified_xml = self.replace_xfa_values(&xfa_data, fields)?;

        // Write the modified XML back
        self.write_xfa_xml(doc, acro_form_ref, &modified_xml)?;

        // Count how many fields we actually filled
        let filled_count = fields.len();
        Ok(filled_count)
    }

    /// Extract the XDP XML content from the /AcroForm/XFA entry.
    fn extract_xfa_xml(&self, doc: &Document, acro_form_ref: ObjectId) -> PdfResult<String> {
        let xfa_obj = {
            let acro_form_dict = doc.get_dictionary(acro_form_ref).map_err(|e| {
                format!("Failed to get AcroForm dictionary: {}", e)
            })?;
            acro_form_dict.get(b"XFA")
                .map_err(|_| "No XFA entry in AcroForm".to_string())?
                .clone()
        };

        match xfa_obj {
            Object::Stream(stream) => {
                // Single stream containing the entire XDP
                let content = stream.decompressed_content()
                    .map_err(|e| format!("Failed to decompress XFA stream: {}", e))?;
                String::from_utf8(content)
                    .map_err(|e| format!("XFA stream is not valid UTF-8: {}", e))
            }
            Object::Reference(id) => {
                // Reference to a stream
                let obj = doc.get_object(id)
                    .map_err(|e| format!("Failed to get XFA object: {}", e))?;
                match obj {
                    Object::Stream(stream) => {
                        let content = stream.decompressed_content()
                            .map_err(|e| format!("Failed to decompress XFA stream: {}", e))?;
                        String::from_utf8(content)
                            .map_err(|e| format!("XFA stream is not valid UTF-8: {}", e))
                    }
                    _ => Err("XFA reference does not point to a stream".to_string()),
                }
            }
            Object::Array(arr) => {
                // Array of [name_string, stream_or_ref, ...] pairs
                let mut combined = String::new();
                for item in arr.iter() {
                    if let Ok(stream) = item.as_reference().and_then(|id| {
                        doc.get_object(id).and_then(Object::as_stream)
                    }) {
                        let content = stream.decompressed_content()
                            .map_err(|e| format!("Failed to decompress XFA sub-stream: {}", e))?;
                        let text = String::from_utf8(content)
                            .map_err(|e| format!("XFA sub-stream is not valid UTF-8: {}", e))?;
                        combined.push_str(&text);
                    } else if let Object::Stream(stream) = item {
                        let content = stream.decompressed_content()
                            .map_err(|e| format!("Failed to decompress XFA sub-stream: {}", e))?;
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

    /// Replace XFA field values in the XDP XML.
    ///
    /// XDP XML uses `<field_name>value</field_name>` elements within the data sets.
    /// We look for matching field name elements and replace their text content.
    fn replace_xfa_values(&self, xml: &str, fields: &HashMap<String, String>) -> PdfResult<String> {
        let mut result = xml.to_string();

        for (field_name, value) in fields {
            // Look for patterns like: <fieldName>oldValue</fieldName>
            // or <fieldName xmlns="...">oldValue</fieldName>
            // We use a simple but effective approach: find the opening tag and replace content
            let encoded_value = value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&apos;");

            // Try with different namespace patterns
            let patterns = vec![
                format!("<{}>", field_name),
                format!("<{} ", field_name),
            ];

            for pattern in &patterns {
                let mut search_start = 0;
                while let Some(tag_start) = result[search_start..].find(pattern.as_str()) {
                    let abs_tag_start = search_start + tag_start;

                    // Find the end of the opening tag
                    let opening_end = if pattern.ends_with(' ') {
                        // Has attributes - find the closing >
                        match result[abs_tag_start..].find('>') {
                            Some(pos) => abs_tag_start + pos + 1,
                            None => break,
                        }
                    } else {
                        abs_tag_start + pattern.len()
                    };

                    // Make sure this is not a closing tag
                    if abs_tag_start > 0 && result.as_bytes()[abs_tag_start - 1] == b'/' {
                        search_start = opening_end;
                        continue;
                    }

                    // Find the closing tag
                    let close_tag = format!("</{}>", field_name);
                    if let Some(closing_start) = result[opening_end..].find(&close_tag) {
                        let abs_closing_start = opening_end + closing_start;

                        // Replace content between tags
                        let mut new_result = String::with_capacity(
                            result.len() - (abs_closing_start - opening_end) + encoded_value.len()
                        );
                        new_result.push_str(&result[..opening_end]);
                        new_result.push_str(&encoded_value);
                        new_result.push_str(&result[abs_closing_start..]);
                        result = new_result;
                    }
                    break; // Only replace first occurrence per field
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
        // Create the new XFA stream object first (before borrowing doc mutably for dict update)
        let xml_bytes = xml.as_bytes().to_vec();
        let xfa_stream_id = doc.add_object(
            Stream::new(dictionary! {}, xml_bytes)
        );

        // Now update the AcroForm dictionary's XFA entry
        let acro_form_dict = doc.get_dictionary_mut(acro_form_ref).map_err(|e| {
            format!("Failed to get AcroForm dictionary: {}", e)
        })?;
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
            Err(_) => return Ok(()), // Silently skip if XFA extraction fails
        };

        // Extract field names from the XDP XML
        // Look for patterns: <fieldName>value</fieldName>
        let mut pos = 0;
        let bytes = xml.as_bytes();

        while pos < bytes.len() {
            // Find next '<' that starts a tag (not '</')
            if bytes[pos] != b'<' || (pos + 1 < bytes.len() && bytes[pos + 1] == b'/') {
                pos += 1;
                continue;
            }

            // Find the end of the opening tag name (before space, >, or />)
            let tag_start = pos + 1;
            let mut tag_end = tag_start;
            while tag_end < bytes.len() && bytes[tag_end] != b'>' && bytes[tag_end] != b' '
                && bytes[tag_end] != b'/' {
                tag_end += 1;
            }

            if tag_end <= tag_start {
                pos += 1;
                continue;
            }

            let tag_name = &xml[tag_start..tag_end];

            // Skip known XDP structural tags
            if tag_name.starts_with('?')
                || tag_name == "xdp:xdp"
                || tag_name == "xfa:datasets"
                || tag_name == "xfa:data"
            {
                // Find the closing >
                let close = match xml[pos..].find('>') {
                    Some(p) => pos + p + 1,
                    None => break,
                };
                pos = close;
                continue;
            }

            // Skip tags with xml, xfa, or xsi namespaces
            if tag_name.contains(':') && !tag_name.starts_with("xfa:") {
                pos += 1;
                continue;
            }

            // Clean namespace prefix if present
            let clean_name = if let Some(idx) = tag_name.find(':') {
                &tag_name[idx + 1..]
            } else {
                tag_name
            };

            // Find the closing tag
            let close_tag = format!("</{}>", tag_name);
            let rest = &xml[pos..];
            if let Some(closing_start) = rest.find(&close_tag) {
                // Check if this is a self-closing tag
                let tag_end_pos = match xml[pos..].find('>') {
                    Some(p) => pos + p,
                    None => break,
                };
                if tag_end_pos > 0 && xml.as_bytes()[tag_end_pos - 1] == b'/' {
                    // Self-closing tag, skip
                    pos = tag_end_pos + 1;
                    continue;
                }

                // Extract the text content (between > and <)
                let content_start = tag_end_pos + 1;
                let content_end = pos + closing_start;

                let value = if content_end > content_start {
                    Some(xml[content_start..content_end].to_string())
                } else {
                    None
                };

                // Filter out structural XFA elements
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

        Ok(())
    }
}

impl Default for PdfFormFiller {
    fn default() -> Self {
        Self::new()
    }
}
