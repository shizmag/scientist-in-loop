use proptest::prelude::*;
use sil_regex::{
    clean_reference_text, extract_arxiv_id, extract_doi, extract_quoted_title, extract_year,
    is_reference_entry_start,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn proptest_extract_doi_no_panic(s in any::<String>()) {
        let _ = extract_doi(&s);
    }

    #[test]
    fn proptest_extract_arxiv_id_no_panic(s in any::<String>()) {
        let _ = extract_arxiv_id(&s);
    }

    #[test]
    fn proptest_extract_year_no_panic(s in any::<String>()) {
        let _ = extract_year(&s);
    }

    #[test]
    fn proptest_extract_quoted_title_no_panic(s in any::<String>()) {
        let _ = extract_quoted_title(&s);
    }

    #[test]
    fn proptest_clean_reference_text_no_panic(s in any::<String>()) {
        let _ = clean_reference_text(&s);
    }

    #[test]
    fn proptest_is_reference_entry_start_no_panic(s in any::<String>()) {
        let _ = is_reference_entry_start(&s);
    }
}
