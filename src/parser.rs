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
                    if let Ok(joined) = base_url.join(href) {
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
    document.root_element().text().fold(0, |acc, text| {
        acc + text.split_whitespace().count()
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