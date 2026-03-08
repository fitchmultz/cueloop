//! Markdown helper tests.

use super::*;

#[test]
fn extract_section_titles_finds_all_sections() {
    let content = r#"# Title

## Section One

Content one.

## Section Two

Content two.

### Subsection

More content.
"#;
    let titles = extract_section_titles(content);
    assert_eq!(titles, vec!["Section One", "Section Two"]);
}

#[test]
fn parse_markdown_sections_extracts_content() {
    let content = r#"# Title

## Section One

Content one.

More content.

## Section Two

Content two.
"#;
    let sections = parse_markdown_sections(content);
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].0, "Section One");
    assert!(sections[0].1.contains("Content one."));
    assert_eq!(sections[1].0, "Section Two");
}
