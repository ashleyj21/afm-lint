use afm_lint::lint;

fn rules(source: &str) -> Vec<&'static str> {
    lint::run(source).iter().map(|d| d.rule).collect()
}

#[test]
fn valid_file_has_no_diagnostics() {
    let source = include_str!("fixtures/valid.afm");
    let found = rules(source);
    assert!(found.is_empty(), "expected no diagnostics, got {:?}", found);
}

#[test]
fn missing_header_key_is_reported() {
    let source = include_str!("fixtures/missing_header.afm");
    assert!(rules(source).contains(&"missing-header-key"));
}

#[test]
fn font_bbox_wrong_number_count_is_reported() {
    let source = include_str!("fixtures/bad_bbox_count.afm");
    assert!(rules(source).contains(&"malformed-font-bbox"));
}

#[test]
fn font_bbox_inverted_corners_is_reported() {
    let source = include_str!("fixtures/bad_bbox_inverted.afm");
    assert!(rules(source).contains(&"inverted-font-bbox"));
}

#[test]
fn ascender_and_descender_sign_are_reported() {
    let source = include_str!("fixtures/sign_errors.afm");
    let found = rules(source);
    assert!(found.contains(&"ascender-sign"));
    assert!(found.contains(&"descender-sign"));
}

#[test]
fn negative_width_is_reported() {
    let source = include_str!("fixtures/negative_width.afm");
    assert!(rules(source).contains(&"negative-width"));
}

#[test]
fn duplicate_character_code_is_reported() {
    let source = include_str!("fixtures/duplicate_code.afm");
    assert!(rules(source).contains(&"duplicate-code"));
}

#[test]
fn char_metrics_count_mismatch_is_reported() {
    let source = include_str!("fixtures/char_count_mismatch.afm");
    assert!(rules(source).contains(&"char-count-mismatch"));
}

#[test]
fn malformed_lines_are_reported_in_every_table() {
    let source = include_str!("fixtures/malformed.afm");
    let found = rules(source);
    assert!(found.contains(&"malformed-char-line"));
    assert!(found.contains(&"malformed-kern-pair"));
    assert!(found.contains(&"malformed-kern-value"));
    assert!(found.contains(&"malformed-composite-line"));
}

#[test]
fn duplicate_kern_pair_and_undefined_glyph_are_reported() {
    let source = include_str!("fixtures/kern_pairs.afm");
    let found = rules(source);
    assert!(found.contains(&"duplicate-kern-pair"));
    assert!(found.contains(&"undefined-kern-glyph"));
}

#[test]
fn kern_pairs_count_mismatch_is_reported() {
    let source = include_str!("fixtures/kern_count_mismatch.afm");
    assert!(rules(source).contains(&"kern-count-mismatch"));
}

#[test]
fn duplicate_composite_and_undefined_glyph_are_reported() {
    let source = include_str!("fixtures/composites.afm");
    let found = rules(source);
    assert!(found.contains(&"duplicate-composite"));
    assert!(found.contains(&"undefined-composite-glyph"));
}

#[test]
fn composite_count_and_part_mismatches_are_reported() {
    let source = include_str!("fixtures/composite_mismatch.afm");
    let found = rules(source);
    assert!(found.contains(&"composite-count-mismatch"));
    assert!(found.contains(&"composite-part-count-mismatch"));
    assert!(found.contains(&"malformed-composite-part"));
}
