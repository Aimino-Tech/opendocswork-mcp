use regex::Regex;
use super::types::*;

pub fn validate_skill_inputs(
    skill: &SkillDefinition,
    params: &serde_json::Map<String, serde_json::Value>,
) -> anyhow::Result<ValidationResult> {
    let mut rule_results = Vec::new();
    let mut all_passed = true;

    for rule in &skill.validation.rules {
        let result = evaluate_rule(skill, &rule, params);
        if !result.passed {
            all_passed = false;
        }
        rule_results.push(result);
    }

    Ok(ValidationResult {
        skill_name: skill.name.clone(),
        passed: all_passed,
        rule_results,
    })
}

fn evaluate_rule(
    skill: &SkillDefinition,
    rule: &ValidationRule,
    params: &serde_json::Map<String, serde_json::Value>,
) -> RuleResult {
    let value = params.get(&rule.field);

    match rule.rule_type {
        ValidationRuleType::Assert => {
            let present = value.is_some();
            let non_empty = match value {
                Some(serde_json::Value::Array(a)) => !a.is_empty(),
                Some(serde_json::Value::String(s)) => !s.is_empty(),
                Some(serde_json::Value::Object(_)) => true,
                Some(serde_json::Value::Number(_)) => true,
                Some(serde_json::Value::Bool(_)) => true,
                Some(serde_json::Value::Null) => false,
                None => false,
            };
            let passed = present && non_empty;
            RuleResult {
                rule_name: rule.name.clone(),
                passed,
                message: if passed { format!("{}: ok", rule.name) } else { rule.message.clone() },
            }
        }
        ValidationRuleType::Regex => {
            let passed = match value {
                Some(serde_json::Value::String(s)) => {
                    match &rule.pattern {
                        Some(pattern) => {
                            match Regex::new(pattern) {
                                Ok(re) => re.is_match(s),
                                Err(e) => {
                                    return RuleResult {
                                        rule_name: rule.name.clone(),
                                        passed: false,
                                        message: format!("Invalid regex pattern '{}': {}", pattern, e),
                                    };
                                }
                            }
                        }
                        None => false,
                    }
                }
                _ => false,
            };
            RuleResult {
                rule_name: rule.name.clone(),
                passed,
                message: if passed { format!("{}: ok", rule.name) } else { rule.message.clone() },
            }
        }
        ValidationRuleType::Enum => {
            let passed = match value {
                Some(serde_json::Value::String(s)) => {
                    match &rule.values {
                        Some(valid) => valid.contains(s),
                        None => false,
                    }
                }
                _ => false,
            };
            RuleResult {
                rule_name: rule.name.clone(),
                passed,
                message: if passed {
                    format!("{}: ok", rule.name)
                } else {
                    let valid = rule.values.clone().unwrap_or_default().join(", ");
                    format!("{}: {}. Valid values: [{}]", rule.message, value.map(|v| v.to_string()).unwrap_or_default(), valid)
                },
            }
        }
        ValidationRuleType::Custom => {
            let passed = evaluate_custom_rule(skill, rule, params);
            RuleResult {
                rule_name: rule.name.clone(),
                passed,
                message: if passed { format!("{}: ok", rule.name) } else { rule.message.clone() },
            }
        }
    }
}

fn evaluate_custom_rule(
    skill: &SkillDefinition,
    rule: &ValidationRule,
    params: &serde_json::Map<String, serde_json::Value>,
) -> bool {
    match rule.name.as_str() {
        "chart_type_for_pie" => {
            match params.get("chart_type").and_then(|v| v.as_str()) {
                Some("pie") => {
                    match params.get("data").and_then(|v| v.as_array()) {
                        Some(data) => {
                            data.first()
                                .and_then(|row| row.as_array())
                                .map(|row| row.len() == 2)
                                .unwrap_or(false)
                        }
                        None => false,
                    }
                }
                _ => true,
            }
        }
        "rows_exist_in_headers" | "values_exist_in_headers" => {
            let fields: Vec<&str> = match params.get(&rule.field) {
                Some(serde_json::Value::Array(arr)) => {
                    arr.iter().filter_map(|v| v.as_str()).collect()
                }
                _ => return false,
            };
            let headers: Vec<&str> = match params.get("data") {
                Some(serde_json::Value::Array(data)) => {
                    data.first()
                        .and_then(|row| row.as_array())
                        .map(|row| row.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default()
                }
                _ => return false,
            };
            if fields.is_empty() {
                return fields.is_empty() && rule.name == "values_exist_in_headers";
            }
            fields.iter().all(|f| headers.contains(f))
        }
        "sections_have_heading" | "sections_have_content" => {
            match params.get("sections") {
                Some(serde_json::Value::Array(sections)) => {
                    for section in sections {
                        match section {
                            serde_json::Value::Object(map) => {
                                if rule.name == "sections_have_heading" {
                                    if !map.contains_key("heading") {
                                        return false;
                                    }
                                }
                                if rule.name == "sections_have_content" {
                                    if !map.contains_key("content") {
                                        return false;
                                    }
                                }
                            }
                            _ => return false,
                        }
                    }
                    true
                }
                _ => false,
            }
        }
        "slides_have_title" | "slides_have_type" => {
            match params.get("slides") {
                Some(serde_json::Value::Array(slides)) => {
                    if slides.is_empty() {
                        return false;
                    }
                    for slide in slides {
                        match slide {
                            serde_json::Value::Object(map) => {
                                if rule.name == "slides_have_title" && !map.contains_key("title") {
                                    return false;
                                }
                                if rule.name == "slides_have_type" && !map.contains_key("type") {
                                    return false;
                                }
                            }
                            _ => return false,
                        }
                    }
                    true
                }
                _ => false,
            }
        }
        "slide_types_valid" => {
            let valid_types = ["title", "content", "section", "thank_you"];
            match params.get("slides") {
                Some(serde_json::Value::Array(slides)) => {
                    slides.iter().all(|slide| {
                        slide.get("type")
                            .and_then(|v| v.as_str())
                            .map(|t| valid_types.contains(&t))
                            .unwrap_or(false)
                    })
                }
                _ => false,
            }
        }
        "line_items_valid" => {
            match params.get("line_items") {
                Some(serde_json::Value::Array(items)) => {
                    if items.is_empty() {
                        return false;
                    }
                    items.iter().all(|item| {
                        item.is_object()
                            && item.get("description").and_then(|v| v.as_str()).is_some()
                            && item.get("quantity").and_then(|v| v.as_f64()).is_some()
                            && item.get("unit_price").and_then(|v| v.as_f64()).is_some()
                    })
                }
                _ => false,
            }
        }
        "line_items_positive" => {
            match params.get("line_items") {
                Some(serde_json::Value::Array(items)) => {
                    items.iter().all(|item| {
                        let qty = item.get("quantity").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let price = item.get("unit_price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        qty > 0.0 && price > 0.0
                    })
                }
                _ => false,
            }
        }
        "tax_rate_range" => {
            match params.get("tax_rate") {
                Some(serde_json::Value::Number(n)) => {
                    n.as_f64().map(|v| (0.0..=1.0).contains(&v)).unwrap_or(false)
                }
                Some(serde_json::Value::Null) => true,
                None => true,
                _ => false,
            }
        }
        "markdown_has_content" => {
            match params.get("markdown_text") {
                Some(serde_json::Value::String(s)) => {
                    s.contains(|c: char| c == '#')
                }
                _ => false,
            }
        }
        "data_consistent_cols" => {
            match params.get("data") {
                Some(serde_json::Value::Array(data)) => {
                    if data.is_empty() { return true; }
                    let first_len = data.first()
                        .and_then(|row| row.as_array())
                        .map(|r| r.len())
                        .unwrap_or(0);
                    if first_len == 0 { return false; }
                    data.iter().all(|row| {
                        row.as_array().map(|r| r.len() == first_len).unwrap_or(false)
                    })
                }
                _ => false,
            }
        }
        "data_min_rows" => {
            let min = match skill.name.as_str() {
                "excel.chart" => 2,
                "excel.pivot" => 2,
                _ => 2,
            };
            match params.get("data") {
                Some(serde_json::Value::Array(data)) => data.len() >= min,
                _ => false,
            }
        }
        "data_min_cols" => {
            match params.get("data") {
                Some(serde_json::Value::Array(data)) => {
                    data.first()
                        .and_then(|row| row.as_array())
                        .map(|r| r.len() >= 2)
                        .unwrap_or(false)
                }
                _ => false,
            }
        }
        _ => true,
    }
}
