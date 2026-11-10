use std::{fs, path::Path};

#[test]
fn readme_invoice_and_pdf_links_exist_and_match_their_formats() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(root.join("README.md")).unwrap();

    for (label, format) in [("Invoice", "DOCX"), ("Financial Report Export", "PDF")] {
        let row = readme
            .lines()
            .find(|line| line.contains(&format!("[{label}](")))
            .unwrap_or_else(|| panic!("README is missing the {label} link"));
        let start = row.find("](").expect("README link target") + 2;
        let end = start + row[start..].find(')').expect("README link end");
        let target = &row[start..end];
        let path = root.join(target);

        assert_eq!(
            path.extension().and_then(|ext| ext.to_str()).unwrap().to_ascii_uppercase(),
            format,
            "{label} must link to a {format} file"
        );
        assert!(path.is_file(), "{label} link target does not exist: {target}");
    }
}
