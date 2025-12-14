use crate::html::escape;

#[derive(Clone, Copy, PartialEq, Eq)]
enum List {
    Bullet,
    Ordered,
}

pub fn render(source: &str) -> String {
    let mut out = String::new();
    let mut text: Vec<&str> = Vec::new();
    let mut quote: Vec<&str> = Vec::new();
    let mut list: Option<List> = None;
    let mut fenced = false;
    for line in source.lines() {
        if fenced {
            if line.trim_start().starts_with("```") {
                out.push_str("</code></pre>\n");
                fenced = false;
            } else {
                out.push_str(&escape(line));
                out.push('\n');
            }
            continue;
        }
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with("```") {
            flush(&mut out, &mut text, &mut quote, &mut list);
            out.push_str("<pre><code>");
            fenced = true;
            continue;
        }
        if trimmed.trim().is_empty() {
            flush(&mut out, &mut text, &mut quote, &mut list);
            continue;
        }
        if let Some(rest) = quoted(trimmed) {
            flush_text(&mut out, &mut text);
            flush_list(&mut out, &mut list);
            quote.push(rest);
            continue;
        }
        if let Some(rest) = item(trimmed, List::Bullet) {
            flush_text(&mut out, &mut text);
            flush_quote(&mut out, &mut quote);
            if list != Some(List::Bullet) {
                flush_list(&mut out, &mut list);
                out.push_str("<ul>\n");
                list = Some(List::Bullet);
            }
            out.push_str(&format!("<li>{}</li>\n", inline(rest)));
            continue;
        }
        if let Some(rest) = item(trimmed, List::Ordered) {
            flush_text(&mut out, &mut text);
            flush_quote(&mut out, &mut quote);
            if list != Some(List::Ordered) {
                flush_list(&mut out, &mut list);
                out.push_str("<ol>\n");
                list = Some(List::Ordered);
            }
            out.push_str(&format!("<li>{}</li>\n", inline(rest)));
            continue;
        }
        if let Some((level, rest)) = heading(trimmed) {
            flush(&mut out, &mut text, &mut quote, &mut list);
            out.push_str(&format!("<h{level}>{}</h{level}>\n", inline(rest)));
            continue;
        }
        flush_quote(&mut out, &mut quote);
        flush_list(&mut out, &mut list);
        text.push(trimmed);
    }
    if fenced {
        out.push_str("</code></pre>\n");
    }
    flush(&mut out, &mut text, &mut quote, &mut list);
    out
}

fn flush(out: &mut String, text: &mut Vec<&str>, quote: &mut Vec<&str>, list: &mut Option<List>) {
    flush_text(out, text);
    flush_quote(out, quote);
    flush_list(out, list);
}

fn flush_text(out: &mut String, text: &mut Vec<&str>) {
    if text.is_empty() {
        return;
    }
    let body: Vec<String> = text.drain(..).map(inline).collect();
    out.push_str(&format!("<p>{}</p>\n", body.join("<br>\n")));
}

fn flush_quote(out: &mut String, quote: &mut Vec<&str>) {
    if quote.is_empty() {
        return;
    }
    let body: Vec<&str> = std::mem::take(quote);
    out.push_str(&format!(
        "<blockquote>{}</blockquote>\n",
        render(&body.join("\n"))
    ));
}

fn flush_list(out: &mut String, list: &mut Option<List>) {
    match list.take() {
        None => {}
        Some(List::Bullet) => out.push_str("</ul>\n"),
        Some(List::Ordered) => out.push_str("</ol>\n"),
    }
}

fn quoted(line: &str) -> Option<&str> {
    let rest = line.trim_start();
    let rest = rest.strip_prefix('>')?;
    Some(rest.strip_prefix(' ').unwrap_or(rest))
}

fn item(line: &str, kind: List) -> Option<&str> {
    let rest = line.trim_start();
    match kind {
        List::Bullet => {
            let rest = rest
                .strip_prefix("- ")
                .or_else(|| rest.strip_prefix("* "))?;
            Some(rest.trim_end())
        }
        List::Ordered => {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if digits.is_empty() {
                return None;
            }
            let rest = rest[digits.len()..].strip_prefix(". ")?;
            Some(rest.trim_end())
        }
    }
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let marks = line.chars().take_while(|c| *c == '#').count();
    if !(1..=3).contains(&marks) {
        return None;
    }
    let rest = line[marks..].trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((marks + 3, rest))
}

fn inline(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '`'
            && let Some(end) = at(&chars, '`', index + 1)
        {
            out.push_str("<code>");
            out.push_str(&escape(&chars[index + 1..end].iter().collect::<String>()));
            out.push_str("</code>");
            index = end + 1;
            continue;
        }
        if chars[index] == '*'
            && chars.get(index + 1) == Some(&'*')
            && let Some(end) = pair(&chars, '*', index + 2).filter(|end| *end > index + 2)
        {
            out.push_str("<strong>");
            out.push_str(&inline(&chars[index + 2..end].iter().collect::<String>()));
            out.push_str("</strong>");
            index = end + 2;
            continue;
        }
        if (chars[index] == '*' || chars[index] == '_')
            && let Some(end) = at(&chars, chars[index], index + 1).filter(|end| *end > index + 1)
        {
            out.push_str("<em>");
            out.push_str(&inline(&chars[index + 1..end].iter().collect::<String>()));
            out.push_str("</em>");
            index = end + 1;
            continue;
        }
        if chars[index] == '['
            && let Some((text, url, end)) = link(&chars, index)
        {
            match address(&url) {
                Some(href) => {
                    out.push_str(&format!(
                        "<a href=\"{href}\">{}</a>",
                        inline(&text),
                        href = escape(&href)
                    ));
                }
                None => out.push_str(&escape(&format!("[{text}]({url})"))),
            }
            index = end;
            continue;
        }
        out.push_str(&escape(&chars[index].to_string()));
        index += 1;
    }
    out
}

fn at(chars: &[char], wanted: char, from: usize) -> Option<usize> {
    (from..chars.len()).find(|index| chars[*index] == wanted)
}

fn pair(chars: &[char], wanted: char, from: usize) -> Option<usize> {
    (from..chars.len().saturating_sub(1))
        .find(|index| chars[*index] == wanted && chars[*index + 1] == wanted)
}

fn link(chars: &[char], from: usize) -> Option<(String, String, usize)> {
    let text_end = at(chars, ']', from + 1)?;
    if chars.get(text_end + 1) != Some(&'(') {
        return None;
    }
    let url_end = at(chars, ')', text_end + 2)?;
    let text: String = chars[from + 1..text_end].iter().collect();
    let url: String = chars[text_end + 2..url_end].iter().collect();
    if text.is_empty() || url.is_empty() {
        return None;
    }
    Some((text, url, url_end + 1))
}

fn address(raw: &str) -> Option<String> {
    let href = raw.trim();
    if href.is_empty() || href.chars().any(char::is_control) || href.contains('"') {
        return None;
    }
    let allowed = href.starts_with('/')
        || href.starts_with('#')
        || href.starts_with("http://")
        || href.starts_with("https://")
        || href.starts_with("mailto:");
    if allowed { Some(href.to_owned()) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::scripting_free;

    #[test]
    fn plain_lines_become_paragraphs() {
        assert_eq!(render("Hello."), "<p>Hello.</p>\n");
        assert_eq!(render("One.\n\nTwo."), "<p>One.</p>\n<p>Two.</p>\n");
    }

    #[test]
    fn a_line_break_inside_a_paragraph_is_kept() {
        assert_eq!(render("One.\nTwo."), "<p>One.<br>\nTwo.</p>\n");
    }

    #[test]
    fn emphasis_and_strong_and_code_are_rendered() {
        assert_eq!(render("*soft*"), "<p><em>soft</em></p>\n");
        assert_eq!(render("**loud**"), "<p><strong>loud</strong></p>\n");
        assert_eq!(render("`code`"), "<p><code>code</code></p>\n");
    }

    #[test]
    fn a_fenced_block_is_kept_as_it_was_written() {
        assert_eq!(
            render("```\nlet x = 1;\n```"),
            "<pre><code>let x = 1;\n</code></pre>\n"
        );
    }

    #[test]
    fn a_fence_that_is_never_closed_is_still_closed() {
        assert_eq!(render("```\nopen"), "<pre><code>open\n</code></pre>\n");
    }

    #[test]
    fn a_quote_and_a_list_are_blocks_of_their_own() {
        assert_eq!(
            render("> spoken"),
            "<blockquote><p>spoken</p>\n</blockquote>\n"
        );
        assert_eq!(
            render("- one\n- two"),
            "<ul>\n<li>one</li>\n<li>two</li>\n</ul>\n"
        );
        assert_eq!(render("1. one"), "<ol>\n<li>one</li>\n</ol>\n");
    }

    #[test]
    fn a_heading_becomes_a_heading_below_the_page_ones() {
        assert_eq!(render("# Title"), "<h4>Title</h4>\n");
        assert_eq!(render("### Part"), "<h6>Part</h6>\n");
    }

    #[test]
    fn markup_in_the_source_is_shown_as_text() {
        assert_eq!(
            render("<img src=1 onerror=alert(1)>"),
            "<p>&lt;img src=1 onerror=alert(1)&gt;</p>\n"
        );
        assert_eq!(render("<b>x</b>"), "<p>&lt;b&gt;x&lt;/b&gt;</p>\n");
    }

    #[test]
    fn a_link_is_offered_only_to_an_address_this_site_will_stand_by() {
        assert_eq!(
            render("[here](https://example.org/x)"),
            "<p><a href=\"https://example.org/x\">here</a></p>\n"
        );
        assert_eq!(
            render("[back](/sections/general)"),
            "<p><a href=\"/sections/general\">back</a></p>\n"
        );
        assert_eq!(
            render("[mail](mailto:a@example.org)"),
            "<p><a href=\"mailto:a@example.org\">mail</a></p>\n"
        );
        assert_eq!(
            render("[no](javascript:alert(1))"),
            "<p>[no](javascript:alert(1))</p>\n"
        );
    }

    #[test]
    fn a_markup_mistake_is_left_as_it_was_written() {
        assert_eq!(render("a ` b"), "<p>a ` b</p>\n");
        assert_eq!(render("a ** b"), "<p>a ** b</p>\n");
        assert_eq!(render("[text] (x)"), "<p>[text] (x)</p>\n");
        assert_eq!(render("#### four"), "<p>#### four</p>\n");
    }

    #[test]
    fn rendered_text_carries_no_scripting() {
        let sources = [
            "<script>alert(1)</script>",
            "<a href=\"javascript:go()\">x</a>",
            "> <iframe src=\"/a\"></iframe>",
            "`onerror=go()`",
            "**<b onclick=\"go()\">x</b>**",
        ];
        for source in sources {
            let html = render(source);
            assert!(
                !html.contains("href=\"javascript:"),
                "an address was let through: {html}"
            );
            assert!(
                scripting_free(&html),
                "not scripting free: {source} -> {html}"
            );
        }
    }
}
