use reqwest::Url;
use scraper::{Html, Selector};


pub fn is_under_base_path(base: &Url, target: &Url) -> bool {
    if base.domain() != target.domain() {
        return false;
    }
    target.path().starts_with(base.path())
}

pub fn extract_links(base_url_str: &str, html: &str) -> Vec<String> {
    let mut links = Vec::new();
    if let Ok(base_url) = Url::parse(base_url_str) {
        let document = Html::parse_document(html);
        if let Ok(selector) = Selector::parse("a[href]") {
            for element in document.select(&selector) {
                if let Some(href) = element.value().attr("href") {
                    if let Ok(mut joined) = base_url.join(href) {
                        joined.set_fragment(None);
                        links.push(joined.to_string());
                    }
                }
            }
        }
    }
    links
}
pub fn count_words(html: &str) -> usize {
    let document = Html::parse_document(html);
    document
        .root_element()
        .text()
        .fold(0, |acc, text| {
            let valid_words = text
                .split_whitespace()
                .filter(|word| {
                    let lower = word.to_lowercase();
                    lower.starts_with(|c: char| c.is_ascii_lowercase())
                })
                .count();
            acc + valid_words
        })
}

pub fn get_extension(url_str: &str) -> String {
    if let Ok(parsed) = Url::parse(url_str) {
        let path = parsed.path();
        if path.ends_with('/') || path.is_empty() {
            return "html".to_string();
        }
        if let Some(pos) = path.rfind('.') {
            let ext = &path[pos + 1..];
            if !ext.contains('/') && !ext.is_empty() {
                return ext.to_lowercase();
            }
        }
    }
    "html".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Url;

    #[test]
    fn test_is_under_base_path_valid_subpaths() {
        let base = Url::parse("https://books.toscrape.com/catalogue/").unwrap();

        let subpage = Url::parse("https://books.toscrape.com/catalogue/page-2.html").unwrap();
        let subcategory = Url::parse("https://books.toscrape.com/catalogue/category/books_1/index.html").unwrap();

        assert!(is_under_base_path(&base, &subpage));
        assert!(is_under_base_path(&base, &subcategory));
    }

    #[test]
    fn test_is_under_base_path_rejects_external_and_parent_paths() {
        let base = Url::parse("https://books.toscrape.com/catalogue/").unwrap();

        let external_domain = Url::parse("https://example.com/catalogue/page-1.html").unwrap();
        let parent_path = Url::parse("https://books.toscrape.com/media/cover.jpg").unwrap();
        let root_path = Url::parse("https://books.toscrape.com/index.html").unwrap();

        assert!(!is_under_base_path(&base, &external_domain));
        assert!(!is_under_base_path(&base, &parent_path));
        assert!(!is_under_base_path(&base, &root_path));
    }

    #[test]
    fn test_extract_links_strips_fragments() {
        let base_url = "https://books.toscrape.com/catalogue/index.html";
        let html = r#"
            <a href="book.html#overview">Overview</a>
            <a href="book.html#reviews">Reviews</a>
            <a href="book.html">Direct Link</a>
        "#;

        let links = extract_links(base_url, html);

        assert_eq!(links.len(), 3);
        assert_eq!(links[0], "https://books.toscrape.com/catalogue/book.html");
        assert_eq!(links[1], "https://books.toscrape.com/catalogue/book.html");
        assert_eq!(links[2], "https://books.toscrape.com/catalogue/book.html");
    }

    #[test]
    fn test_extract_links_resolves_relative_paths() {
        let base_url = "https://books.toscrape.com/catalogue/category/books_1/index.html";
        let html = r#"
            <a href="../../a-light-in-the-attic_1000/index.html">Book Detail</a>
            <a href="../books_2/page-2.html">Category Page 2</a>
            <a href="/index.html">Root Index</a>
        "#;

        let links = extract_links(base_url, html);

        assert_eq!(links.len(), 3);
        assert_eq!(links[0], "https://books.toscrape.com/catalogue/a-light-in-the-attic_1000/index.html");
        assert_eq!(links[1], "https://books.toscrape.com/catalogue/category/books_2/page-2.html");
        assert_eq!(links[2], "https://books.toscrape.com/index.html");
    }

    #[test]
    fn test_extract_links_ignores_non_anchor_or_missing_href() {
        let base_url = "https://books.toscrape.com/index.html";
        let html = r#"
            <a name="top">Anchor without href</a>
            <div>Not a link</div>
            <a href="valid.html">Valid Link</a>
        "#;

        let links = extract_links(base_url, html);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "https://books.toscrape.com/valid.html");
    }

    #[test]
    fn test_count_words_filters_alphabetic_strings() {
        let html = r#"
            <html>
                <body>
                    <h1>Welcome to the Store!</h1>
                    <p>Price: $19.99 (In stock: 50 items) - #1 Best-Seller!</p>
                </body>
            </html>
        "#;

        let count = count_words(html);
        assert!(count > 0);
    }

    #[test]
    fn test_count_words_ignores_html_tags_and_attributes() {
        let html = r#"
            <div class="product_main" id="12345" style="display: block;">
                <!-- This is an HTML comment that should be ignored -->
                <h2>The attic is upstairs</h2>
            </div>
        "#;

        let count = count_words(html);
        assert_eq!(count, 4);
    }

    #[test]
    fn test_get_extension_normalizes_case_and_directories() {
        assert_eq!(get_extension("https://books.toscrape.com/"), "html");
        assert_eq!(get_extension("https://books.toscrape.com/catalogue/category/"), "html");

        assert_eq!(get_extension("https://books.toscrape.com/media/cover.JPG"), "jpg");
        assert_eq!(get_extension("https://books.toscrape.com/docs/manual.PDF"), "pdf");
        assert_eq!(get_extension("https://books.toscrape.com/assets/style.CSS"), "css");
    }

    #[test]
    fn test_get_extension_various_formats() {
        assert_eq!(get_extension("https://books.toscrape.com/archive.zip"), "zip");
        assert_eq!(get_extension("https://books.toscrape.com/script.js"), "js");
        assert_eq!(get_extension("https://books.toscrape.com/image.png"), "png");
        assert_eq!(get_extension("https://books.toscrape.com/data.tar.gz"), "gz");
    }
}