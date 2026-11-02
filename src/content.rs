//! Readable text extraction for the optional raw-content results.
//!
//! This is not a full readability implementation: it takes the largest of the
//! article/main/body containers and normalises the whitespace, which is enough
//! for an agent to read the page behind a search result.

use scraper::Html;

/// Longest text returned per page, in characters, unless the caller asks for
/// a different limit.
const MAX_CHARS: usize = 4_000;

/// The readable text of a parsed HTML document.
pub fn main_text(html: &Html) -> String {
    main_text_limited(html, MAX_CHARS)
}

/// The readable text of a parsed HTML document, capped at the given length.
pub fn main_text_limited(html: &Html, max: usize) -> String {
    const CANDIDATES: &[&str] = &["article", "main", "[role=main]", "body"];
    for selector in CANDIDATES {
        let Ok(selector) = scraper::Selector::parse(selector) else {
            continue;
        };
        if let Some(element) = html.select(&selector).next() {
            let text = normalise(&element.text().collect::<Vec<_>>().join(" "));
            if !text.is_empty() {
                return truncate(&text, max);
            }
        }
    }
    String::new()
}

/// The canonical URL a page declares for itself, if any.
///
/// Reads `<link rel="canonical">` first and falls back to
/// `<meta property="og:url">`. Relative values are resolved against
/// `base_url` (the URL the document was fetched from) and fragments are
/// dropped, because a fragment never identifies a different page.
///
/// This only *extracts* the declaration. Whether it is a self-canonical
/// (the common case, and no signal either way) or a cross-URL pointer to
/// the original page is for the caller to decide.
pub fn canonical_url(html: &Html, base_url: &str) -> Option<String> {
    let base = url::Url::parse(base_url).ok();
    let resolve = |raw: &str| -> Option<String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        let mut parsed = match &base {
            Some(base) => base.join(raw).ok()?,
            None => url::Url::parse(raw).ok()?,
        };
        if !matches!(parsed.scheme(), "http" | "https") {
            return None;
        }
        parsed.set_fragment(None);
        Some(parsed.to_string())
    };
    if let Ok(selector) = scraper::Selector::parse("link[rel~=\"canonical\"]") {
        for element in html.select(&selector) {
            if let Some(resolved) = element.value().attr("href").and_then(resolve) {
                return Some(resolved);
            }
        }
    }
    if let Ok(selector) = scraper::Selector::parse("meta[property=\"og:url\"]") {
        for element in html.select(&selector) {
            if let Some(resolved) = element.value().attr("content").and_then(resolve) {
                return Some(resolved);
            }
        }
    }
    None
}

/// Collapse whitespace and drop empty lines.
fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn article_wins_over_body() {
        let html = Html::parse_document(
            "<html><body><nav>menu</nav><article><p>Hello   world</p></article></body></html>",
        );
        assert_eq!(main_text(&html), "Hello world");
    }

    #[test]
    fn plain_body_is_the_fallback() {
        let html = Html::parse_document("<html><body><p>Just text</p></body></html>");
        assert_eq!(main_text(&html), "Just text");
    }

    fn doc(head: &str) -> Html {
        Html::parse_document(&format!(
            "<html><head>{head}</head><body><p>x</p></body></html>"
        ))
    }

    #[test]
    fn canonical_link_wins_over_og_url() {
        let html = doc(
            "<link rel=\"canonical\" href=\"https://docs.example.com/a\">\
             <meta property=\"og:url\" content=\"https://other.example.com/a\">",
        );
        assert_eq!(
            canonical_url(&html, "https://mirror.example.com/a"),
            Some("https://docs.example.com/a".to_string())
        );
    }

    #[test]
    fn canonical_relative_href_is_resolved_and_fragment_dropped() {
        let html = doc("<link rel=\"canonical\" href=\"/guide#top\">");
        assert_eq!(
            canonical_url(&html, "https://example.com/docs/guide"),
            Some("https://example.com/guide".to_string())
        );
    }

    #[test]
    fn og_url_is_the_fallback() {
        let html = doc("<meta property=\"og:url\" content=\"https://example.com/x\">");
        assert_eq!(
            canonical_url(&html, "https://example.com/y"),
            Some("https://example.com/x".to_string())
        );
    }

    #[test]
    fn self_canonical_is_returned_verbatim_for_the_caller_to_classify() {
        let html = doc("<link rel=\"canonical\" href=\"https://example.com/a\">");
        assert_eq!(
            canonical_url(&html, "https://example.com/a"),
            Some("https://example.com/a".to_string())
        );
    }

    #[test]
    fn no_canonical_declaration_returns_none() {
        let html = doc("<title>plain</title>");
        assert_eq!(canonical_url(&html, "https://example.com/a"), None);
    }

    #[test]
    fn non_http_canonical_is_rejected() {
        let html = doc("<link rel=\"canonical\" href=\"ftp://example.com/a\">");
        assert_eq!(canonical_url(&html, "https://example.com/a"), None);
    }

    #[test]
    fn long_text_is_truncated() {
        let body = "a".repeat(MAX_CHARS + 100);
        let html = Html::parse_document(&format!("<html><body><p>{body}</p></body></html>"));
        let text = main_text(&html);
        assert!(text.chars().count() <= MAX_CHARS + 1);
    }
}
