use crate::theme::Theme;
use client::{PageInfo, Paged, Section, Topic};

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
    page_at(theme, title, body, "/")
}

pub fn page_at(theme: Theme, title: &str, body: &str, return_to: &str) -> String {
    format!(
        concat!(
            "<!DOCTYPE html>\n",
            "<html lang=\"en\" class=\"theme-{theme}\">\n",
            "<head>\n",
            "<meta charset=\"utf-8\">\n",
            "<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n",
            "<title>{title} &#183; Forum</title>\n",
            "<link rel=\"stylesheet\" href=\"/static/style.css\">\n",
            "</head>\n",
            "<body class=\"theme-{theme}\">\n",
            "<p class=\"skip\"><a href=\"#main\">Skip to the content</a></p>\n",
            "<header class=\"masthead\">\n",
            "<h1><a href=\"/\">Forum</a></h1>\n",
            "{picker}",
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
        picker = theme_picker(theme, return_to),
        body = body,
    )
}

pub fn section_list(sections: &[Section]) -> String {
    if sections.is_empty() {
        return "<p>No sections yet.</p>\n".to_owned();
    }
    let mut out = String::from(
        "<table class=\"sections\">\n<caption>Sections</caption>\n<thead>\n<tr><th scope=\"col\">Section</th><th scope=\"col\">Address</th><th scope=\"col\">May post</th></tr>\n</thead>\n<tbody>\n",
    );
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

pub fn topic_list(section: &Section, topics: &Paged<Topic>) -> String {
    let mut out = format!(
        "<h2>{title}</h2>\n<table class=\"topics\">\n<caption>{count}</caption>\n<thead>\n<tr><th scope=\"col\">Subject</th><th scope=\"col\">Written by</th><th scope=\"col\">Written at</th><th scope=\"col\">Tags</th></tr>\n</thead>\n<tbody>\n",
        title = escape(&section.title),
        count = escape(&subject_count(topics.page.total)),
    );
    if topics.items.is_empty() {
        out.push_str("<tr><td colspan=\"4\">No subjects here yet.</td></tr>\n");
    }
    for topic in &topics.items {
        out.push_str(&format!(
            "<tr><th scope=\"row\"><a href=\"/topics/{id}\">{title}</a>{marks}</th><td>{author}</td><td><time datetime=\"{stamp}\">{shown}</time></td><td>{tags}</td></tr>\n",
            id = escape(&topic.id),
            title = escape(&topic.title),
            marks = marks(topic),
            author = escape(&topic.author_username),
            stamp = escape(&topic.created_at),
            shown = escape(&shown_date(&topic.created_at)),
            tags = tag_links(&topic.tags),
        ));
    }
    out.push_str("</tbody>\n</table>\n");
    out.push_str(&pager(&section.slug, &topics.page));
    out
}

fn subject_count(total: u64) -> String {
    match total {
        1 => "1 subject".to_owned(),
        other => format!("{other} subjects"),
    }
}

fn marks(topic: &Topic) -> String {
    let mut marks = Vec::new();
    if topic.sticky {
        marks.push("pinned");
    }
    if topic.resolved {
        marks.push("resolved");
    }
    if topic.pending {
        marks.push("awaiting a look");
    }
    if topic.draft {
        marks.push("draft");
    }
    if topic.deleted {
        marks.push("removed");
    }
    if marks.is_empty() {
        return String::new();
    }
    format!(
        " <span class=\"marks\">({})</span>",
        escape(&marks.join(", "))
    )
}

fn tag_links(tags: &[String]) -> String {
    if tags.is_empty() {
        return "<span class=\"none\">none</span>".to_owned();
    }
    tags.iter()
        .map(|tag| {
            format!(
                "<a href=\"/tags/{tag}\">{shown}</a>",
                tag = escape(tag),
                shown = escape(tag),
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn pager(slug: &str, page: &PageInfo) -> String {
    let mut out = format!(
        "<nav class=\"pager\">\n<p>Page {number} of {pages}</p>\n<ul>\n",
        number = page.number,
        pages = page.total_pages,
    );
    if page.has_previous {
        out.push_str(&format!(
            "<li><a rel=\"prev\" href=\"/sections/{slug}?page={previous}\">Previous page</a></li>\n",
            slug = escape(slug),
            previous = page.number.saturating_sub(1),
        ));
    }
    if page.has_next {
        out.push_str(&format!(
            "<li><a rel=\"next\" href=\"/sections/{slug}?page={next}\">Next page</a></li>\n",
            slug = escape(slug),
            next = page.number.saturating_add(1),
        ));
    }
    out.push_str("</ul>\n</nav>\n");
    out
}

pub fn shown_date(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let shape = bytes.len() >= 16
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':';
    match (shape, raw.get(..10), raw.get(11..16)) {
        (true, Some(day), Some(clock)) => format!("{day} {clock}"),
        _ => raw.to_owned(),
    }
}

pub fn theme_picker(current: Theme, return_to: &str) -> String {
    let mut options = String::new();
    for theme in Theme::ALL {
        options.push_str(&format!(
            "<option value=\"{name}\"{selected}>{label}</option>",
            name = theme.name(),
            label = escape(theme.label()),
            selected = if theme == current { " selected" } else { "" },
        ));
    }
    format!(
        concat!(
            "<form class=\"theme-picker\" method=\"post\" action=\"/theme\">\n",
            "<label for=\"theme\">Theme</label>\n",
            "<select id=\"theme\" name=\"theme\">{options}</select>\n",
            "<input type=\"hidden\" name=\"return_to\" value=\"{return_to}\">\n",
            "<button type=\"submit\">Apply</button>\n",
            "</form>\n"
        ),
        options = options,
        return_to = escape(return_to),
    )
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
        let tail: String = rest
            .chars()
            .skip(2)
            .take_while(|c| c.is_alphanumeric())
            .collect();
        let after: String = rest
            .chars()
            .skip(2 + tail.chars().count())
            .take(2)
            .collect();
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
    fn a_page_is_a_whole_document_with_a_title_and_a_stylesheet() {
        let html = page(Theme::Light, "Sections", "<p>body</p>");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\" class=\"theme-light\">"));
        assert!(html.contains("<title>Sections &#183; Forum</title>"));
        assert!(html.contains("href=\"/static/style.css\""));
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

    fn topic(id: &str, title: &str) -> Topic {
        Topic {
            id: id.to_owned(),
            section_slug: "general".to_owned(),
            title: title.to_owned(),
            author_username: "alice".to_owned(),
            created_at: "2024-06-07T10:11:12Z".to_owned(),
            tags: vec!["rust".to_owned()],
            sticky: false,
            resolved: false,
            deleted: false,
            pending: false,
            draft: false,
            postscore: 0,
        }
    }

    fn paged(items: Vec<Topic>, total: u64) -> Paged<Topic> {
        Paged {
            items,
            page: PageInfo {
                number: 1,
                size: 25,
                total,
                total_pages: if total > 25 { 2 } else { 1 },
                has_next: total > 25,
                has_previous: false,
            },
        }
    }

    #[test]
    fn a_topic_list_offers_every_subject_by_title() {
        let html = topic_list(
            &section("general", "General Talk", true),
            &paged(vec![topic("1", "First subject"), topic("2", "Second")], 2),
        );
        assert!(html.contains("<h2>General Talk</h2>"));
        assert!(html.contains("href=\"/topics/1\""));
        assert!(html.contains("First subject"));
        assert!(html.contains("href=\"/topics/2\""));
        assert!(html.contains("<caption>2 subjects</caption>"));
        assert!(html.contains(">alice<"));
        assert!(html.contains("datetime=\"2024-06-07T10:11:12Z\""));
        assert!(html.contains(">2024-06-07 10:11<"));
        assert!(html.contains("href=\"/tags/rust\""));
    }

    #[test]
    fn a_topic_list_with_nothing_in_it_says_so() {
        let html = topic_list(&section("general", "General Talk", true), &paged(vec![], 0));
        assert!(html.contains("No subjects here yet."));
        assert!(html.contains("<caption>0 subjects</caption>"));
    }

    #[test]
    fn one_subject_is_counted_in_the_singular() {
        let html = topic_list(&section("general", "General", true), &paged(vec![], 1));
        assert!(html.contains("<caption>1 subject</caption>"));
    }

    #[test]
    fn what_a_subject_is_carries_as_a_mark() {
        let mut pinned = topic("1", "Pinned");
        pinned.sticky = true;
        pinned.resolved = true;
        let html = topic_list(
            &section("general", "General", true),
            &paged(vec![pinned], 1),
        );
        assert!(html.contains("(pinned, resolved)"));
        let plain = topic_list(
            &section("general", "General", true),
            &paged(vec![topic("2", "Plain")], 1),
        );
        assert!(!plain.contains("class=\"marks\""));
    }

    #[test]
    fn a_subject_with_no_tags_says_so() {
        let mut bare = topic("1", "Bare");
        bare.tags.clear();
        let html = topic_list(&section("general", "General", true), &paged(vec![bare], 1));
        assert!(html.contains(">none<"));
    }

    #[test]
    fn text_from_the_api_cannot_become_markup_in_a_topic_list() {
        let html = topic_list(
            &section("general", "General", true),
            &paged(vec![topic("1", "<b>bold</b>\"x'")], 1),
        );
        assert!(!html.contains("<b>bold</b>"));
        assert!(html.contains("&lt;b&gt;bold&lt;/b&gt;"));
    }

    #[test]
    fn a_pager_offers_the_neighbouring_pages_only() {
        let mut page = paged(vec![], 60).page;
        page.number = 2;
        page.has_previous = true;
        page.has_next = true;
        let html = pager("general", &page);
        assert!(html.contains("<p>Page 2 of 2</p>"));
        assert!(html.contains("href=\"/sections/general?page=1\""));
        assert!(html.contains("rel=\"prev\""));
        assert!(html.contains("href=\"/sections/general?page=3\""));
        assert!(html.contains("rel=\"next\""));
    }

    #[test]
    fn the_first_page_offers_no_step_back() {
        let html = pager("general", &paged(vec![], 0).page);
        assert!(html.contains("<p>Page 1 of 1</p>"));
        assert!(!html.contains("rel=\"prev\""));
        assert!(!html.contains("rel=\"next\""));
    }

    #[test]
    fn a_stamp_becomes_a_day_and_a_clock_and_anything_else_is_left_alone() {
        assert_eq!(shown_date("2024-06-07T10:11:12Z"), "2024-06-07 10:11");
        assert_eq!(shown_date("later"), "later");
        assert_eq!(shown_date(""), "");
        assert_eq!(shown_date("2024-06-07"), "2024-06-07");
    }

    #[test]
    fn a_topic_list_page_carries_no_scripting() {
        let html = page(
            Theme::Light,
            "General Talk",
            &topic_list(
                &section("general", "General Talk", true),
                &paged(vec![topic("1", "First")], 1),
            ),
        );
        assert!(scripting_free(&html), "not scripting free: {html}");
    }

    #[test]
    fn the_theme_picker_offers_every_theme_and_remembers_the_current_one() {
        let html = theme_picker(Theme::Contrast, "/sections/general");
        for theme in Theme::ALL {
            assert!(html.contains(&format!("value=\"{}\"", theme.name())));
        }
        assert!(html.contains("value=\"contrast\" selected"));
        assert!(html.contains("value=\"/sections/general\""));
        assert!(html.contains("method=\"post\" action=\"/theme\""));
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
