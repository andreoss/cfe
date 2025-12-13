use crate::theme::Theme;
use client::Section;

pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for character in raw.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

pub fn page(theme: Theme, title: &str, body: &str) -> String {
    format!(
        concat!(
            "<!DOCTYPE html>\n",
            "<html lang=\"en\" class=\"theme-{theme}\">\n",
            "<head>\n",
            "<meta charset=\"utf-8\">\n",
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
            "<title>{title} &#183; Forum</title>\n",
            "</head>\n",
            "<body class=\"theme-{theme}\">\n",
            "<p class=\"skip\"><a href=\"#main\">Skip to the content</a></p>\n",
            "<header class=\"masthead\">\n",
            "<h1><a href=\"/\">Forum</a></h1>\n",
            "</header>\n",
            "<main id=\"main\">\n",
            "{body}\n",
            "</main>\n",
            "<footer class=\"foot\"><p>Server-rendered pages. Nothing here needs scripts.</p></footer>\n",
            "</body>\n",
            "</html>\n"
        ),
        theme = theme.name(),
        title = escape(title),
        body = body,
    )
}

pub fn section_list(sections: &[Section]) -> String {
    if sections.is_empty() {
        return "<p>No sections yet.</p>\n".to_owned();
    }
    let mut out = String::from("<table class=\"sections\">\n<caption>Sections</caption>\n<thead>\n<tr><th scope=\"col\">Section</th><th scope=\"col\">Address</th><th scope=\"col\">May post</th></tr>\n</thead>\n<tbody>\n");
    for section in sections {
        out.push_str(&format!(
            "<tr><th scope=\"row\"><a href=\"/sections/{slug}\">{title}</a></th><td><code>{slug}</code></td><td>{may_post}</td></tr>\n",
            slug = escape(&section.slug),
            title = escape(&section.title),
            may_post = if section.may_post { "yes" } else { "no" },
        ));
    }
    out.push_str("</tbody>\n</table>\n");
    out
}

pub fn message(theme: Theme, heading: &str, detail: &str) -> String {
    page(
        theme,
        heading,
        &format!(
            "<h2>{heading}</h2>\n<p>{detail}</p>\n<p><a href=\"/\">Back to the sections</a></p>",
            heading = escape(heading),
            detail = escape(detail),
        ),
    )
}

pub fn scripting_free(html: &str) -> bool {
    let lower = html.to_ascii_lowercase();
    if lower.contains("<script")
        || lower.contains("<noscript")
        || lower.contains("javascript:")
        || lower.contains("<iframe")
    {
        return false;
    }
    let bytes: Vec<char> = lower.chars().collect();
    for (index, character) in bytes.iter().enumerate() {
        if *character != ' ' {
            continue;
        }
        let rest: String = bytes[index + 1..].iter().collect();
        if !rest.starts_with("on") {
            continue;
        }
        let tail: String = rest.chars().skip(2).take_while(|c| c.is_alphanumeric()).collect();
        let after: String = rest.chars().skip(2 + tail.chars().count()).take(2).collect();
        if !tail.is_empty() && after.starts_with('=') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(slug: &str, title: &str, may_post: bool) -> Section {
        Section {
            slug: slug.to_owned(),
            title: title.to_owned(),
            topics_score: "anyone".to_owned(),
            may_post,
        }
    }

    #[test]
    fn escaping_takes_the_markup_out_of_text() {
        assert_eq!(escape("a<b>&c\"d'e"), "a&lt;b&gt;&amp;c&quot;d&#39;e");
    }

    #[test]
    fn a_page_is_a_whole_document_with_a_title() {
        let html = page(Theme::Light, "Sections", "<p>body</p>");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\" class=\"theme-light\">"));
        assert!(html.contains("<title>Sections &#183; Forum</title>"));
        assert!(html.contains("<p>body</p>"));
        assert!(html.ends_with("</html>\n"));
    }

    #[test]
    fn a_page_shows_the_theme_it_was_rendered_with() {
        assert!(page(Theme::Dark, "x", "").contains("class=\"theme-dark\""));
    }

    #[test]
    fn a_section_list_offers_every_section_by_name() {
        let html = section_list(&[section("general", "General Talk", true)]);
        assert!(html.contains("href=\"/sections/general\""));
        assert!(html.contains("General Talk"));
        assert!(html.contains("<caption>Sections</caption>"));
        assert!(html.contains("<th scope=\"col\">May post</th>"));
    }

    #[test]
    fn a_section_list_with_nothing_in_it_says_so() {
        assert_eq!(section_list(&[]), "<p>No sections yet.</p>\n");
    }

    #[test]
    fn text_from_the_api_cannot_become_markup() {
        let html = section_list(&[section("x", "<img src=1 onerror=alert(1)>", false)]);
        assert!(!html.contains("<img"));
        assert!(html.contains("&lt;img"));
    }

    #[test]
    fn a_message_page_names_its_heading_and_its_detail() {
        let html = message(Theme::Light, "Unavailable", "The board is not answering");
        assert!(html.contains("<h2>Unavailable</h2>"));
        assert!(html.contains("The board is not answering"));
    }

    #[test]
    fn rendered_pages_carry_no_scripting() {
        let pages = [
            page(
                Theme::Light,
                "Sections",
                &section_list(&[section("general", "General", true)]),
            ),
            message(Theme::Dark, "Unavailable", "try again"),
        ];
        for html in pages {
            assert!(scripting_free(&html), "not scripting free: {html}");
        }
    }

    #[test]
    fn scripting_is_recognised_wherever_it_hides() {
        assert!(!scripting_free("<p onclick=\"go()\">x</p>"));
        assert!(!scripting_free("<script src=\"/a.js\"></script>"));
        assert!(!scripting_free("<a href=\"javascript:go()\">x</a>"));
        assert!(!scripting_free("<noscript>turn it on</noscript>"));
        assert!(!scripting_free("<iframe src=\"/a\"></iframe>"));
        assert!(scripting_free("<p>on click = nothing happens</p>"));
        assert!(scripting_free("<p class=\"online\">online</p>"));
    }
}
