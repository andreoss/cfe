use domain::Topic;
use time::format_description::well_known::Rfc3339;

pub struct FeedEntry {
    pub id: String,
    pub title: String,
    pub author: String,
    pub updated: String,
    pub summary: String,
}

pub fn escape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

pub fn entry_from(topic: &Topic, author: &str) -> FeedEntry {
    FeedEntry {
        id: topic.id().as_uuid().to_string(),
        title: topic.title().as_str().to_owned(),
        author: author.to_owned(),
        updated: topic
            .created_at()
            .format(&Rfc3339)
            .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z")),
        summary: topic.body().as_str().to_owned(),
    }
}

pub fn render(title: &str, self_url: &str, updated: &str, entries: &[FeedEntry]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
    out.push_str(&format!("  <title>{}</title>\n", escape(title)));
    out.push_str(&format!("  <id>{}</id>\n", escape(self_url)));
    out.push_str(&format!(
        "  <link rel=\"self\" href=\"{}\"/>\n",
        escape(self_url)
    ));
    out.push_str(&format!("  <updated>{}</updated>\n", escape(updated)));
    for entry in entries {
        out.push_str("  <entry>\n");
        out.push_str(&format!("    <id>urn:uuid:{}</id>\n", escape(&entry.id)));
        out.push_str(&format!("    <title>{}</title>\n", escape(&entry.title)));
        out.push_str(&format!(
            "    <author><name>{}</name></author>\n",
            escape(&entry.author)
        ));
        out.push_str(&format!(
            "    <updated>{}</updated>\n",
            escape(&entry.updated)
        ));
        out.push_str(&format!(
            "    <summary>{}</summary>\n",
            escape(&entry.summary)
        ));
        out.push_str("  </entry>\n");
    }
    out.push_str("</feed>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, summary: &str) -> FeedEntry {
        FeedEntry {
            id: "11111111-1111-1111-1111-111111111111".to_owned(),
            title: title.to_owned(),
            author: "alice_01".to_owned(),
            updated: "2026-09-03T00:00:00Z".to_owned(),
            summary: summary.to_owned(),
        }
    }

    #[test]
    fn escapes_every_markup_character() {
        assert_eq!(
            escape("a & b < c > d \" e ' f"),
            "a &amp; b &lt; c &gt; d &quot; e &apos; f"
        );
    }

    #[test]
    fn renders_an_empty_feed() {
        let xml = render("General", "http://host/feed", "2026-09-03T00:00:00Z", &[]);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(xml.contains("<title>General</title>"));
        assert!(!xml.contains("<entry>"));
        assert!(xml.ends_with("</feed>\n"));
    }

    #[test]
    fn renders_one_entry_per_topic_in_order() {
        let xml = render(
            "General",
            "http://host/feed",
            "2026-09-03T00:00:00Z",
            &[entry("First", "One"), entry("Second", "Two")],
        );
        let first = xml.find("First").unwrap();
        let second = xml.find("Second").unwrap();
        assert!(first < second);
        assert_eq!(xml.matches("<entry>").count(), 2);
    }

    #[test]
    fn a_title_containing_markup_cannot_break_out_of_the_document() {
        let xml = render(
            "General",
            "http://host/feed",
            "2026-09-03T00:00:00Z",
            &[entry("</title><script>alert(1)</script>", "Body & more")],
        );
        assert!(!xml.contains("<script>"));
        assert!(xml.contains("&lt;script&gt;"));
        assert!(xml.contains("Body &amp; more"));
        assert_eq!(xml.matches("<entry>").count(), 1);
    }

    #[test]
    fn a_feed_title_is_escaped_too() {
        let xml = render(
            "Tag & <b>",
            "http://host/feed?q=1&x=2",
            "2026-01-01T00:00:00Z",
            &[],
        );
        assert!(xml.contains("<title>Tag &amp; &lt;b&gt;</title>"));
        assert!(xml.contains("href=\"http://host/feed?q=1&amp;x=2\""));
    }
}
