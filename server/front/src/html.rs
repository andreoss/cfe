use crate::markup;
use crate::theme::Theme;
use client::{
    Comment, Criteria, Hit, Order, PageInfo, Paged, Profile, Scope, Section, Subject, Tag, Topic,
};

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

#[derive(Clone)]
pub struct Chrome {
    theme: Theme,
    token: String,
    account: Option<String>,
    return_to: String,
}

impl Chrome {
    pub fn new(theme: Theme, token: &str) -> Self {
        Self {
            theme,
            token: token.to_owned(),
            account: None,
            return_to: "/".to_owned(),
        }
    }

    pub fn account(mut self, name: &str) -> Self {
        self.account = Some(name.to_owned());
        self
    }

    pub fn return_to(mut self, address: &str) -> Self {
        self.return_to = address.to_owned();
        self
    }

    pub fn account_name(&self) -> Option<&str> {
        self.account.as_deref()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ProfileView {
    pub own: bool,
    pub has_avatar: bool,
}

pub fn page(chrome: &Chrome, title: &str, body: &str) -> String {
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
            "{ways}",
            "{account}",
            "{picker}",
            "</header>\n",
            "<main id=\"main\">\n",
            "{body}\n",
            "</main>\n",
            "<footer class=\"foot\"><p>Server-rendered pages. Nothing here needs scripts.</p></footer>\n",
            "</body>\n",
            "</html>\n"
        ),
        theme = chrome.theme.name(),
        title = escape(title),
        ways = wayfinding(),
        account = account_box(chrome),
        picker = theme_picker(chrome.theme, &chrome.return_to, &chrome.token),
        body = body,
    )
}

fn wayfinding() -> String {
    concat!(
        "<nav class=\"ways\" aria-label=\"Wayfinding\">\n",
        "<a href=\"/search\">Search</a>\n",
        "</nav>\n",
    )
    .to_owned()
}

pub fn account_box(chrome: &Chrome) -> String {
    match chrome.account.as_deref() {
        Some(name) => format!(
            concat!(
                "<div class=\"account\">\n",
                "<p class=\"who\">Signed in as <a href=\"/u/{name}\">{shown}</a></p>\n",
                "<form method=\"post\" action=\"/sign-out\">\n",
                "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                "<button type=\"submit\">Sign out</button>\n",
                "</form>\n",
                "</div>\n"
            ),
            name = escape(name),
            shown = escape(name),
            token = escape(&chrome.token),
        ),
        None => concat!(
            "<div class=\"account\">\n",
            "<p class=\"who\"><a href=\"/sign-in\">Sign in</a> or ",
            "<a href=\"/register\">register</a></p>\n",
            "</div>\n"
        )
        .to_owned(),
    }
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
    out.push_str(&topic_rows(&topics.items));
    out.push_str("</tbody>\n</table>\n");
    out.push_str(&feed_link(
        &section_feed_address(&section.slug),
        "The feed of this section",
    ));
    out.push_str(&pager(
        &format!("/sections/{}", client::encode_path(&section.slug)),
        &topics.page,
    ));
    out
}

pub fn section_feed_address(slug: &str) -> String {
    format!("/sections/{}/feed", client::encode_path(slug))
}

pub fn tag_feed_address(name: &str) -> String {
    format!("/tags/{}/feed", client::encode_path(name))
}

pub fn feed_link(address: &str, words: &str) -> String {
    format!(
        "<p class=\"feed\"><a href=\"{address}\" type=\"application/atom+xml\">{words}</a></p>\n",
        address = escape(address),
        words = escape(words),
    )
}

fn topic_rows(topics: &[Topic]) -> String {
    let mut out = String::new();
    for topic in topics {
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
    out
}

pub fn tag_page(tag: &Tag, topics: &Paged<Topic>, chrome: &Chrome) -> String {
    let mut out = format!("<h2>{slug}</h2>\n", slug = escape(&tag.slug),);
    if let Some(words) = tag.description.as_deref() {
        out.push_str(&format!(
            "<p class=\"tag-words\">{words}</p>\n",
            words = escape(words),
        ));
    }
    if let Some(means) = tag.means.as_deref() {
        out.push_str(&format!(
            concat!("<p class=\"tag-means\">Means <a href=\"/tags/{means}\">{shown}</a>.</p>\n",),
            means = escape(means),
            shown = escape(means),
        ));
    }
    if chrome.account_name().is_some() {
        out.push_str(&follow_form(chrome, tag));
        out.push_str(&describe_form(chrome, tag));
    }
    out.push_str(&format!(
        "<h3>{count}</h3>\n",
        count = escape(&subject_count(topics.page.total))
    ));
    out.push_str("<table class=\"topics\">\n<thead>\n<tr><th scope=\"col\">Subject</th><th scope=\"col\">Written by</th><th scope=\"col\">Written at</th><th scope=\"col\">Tags</th></tr>\n</thead>\n<tbody>\n");
    if topics.items.is_empty() {
        out.push_str("<tr><td colspan=\"4\">No subject carries this tag yet.</td></tr>\n");
    }
    out.push_str(&topic_rows(&topics.items));
    out.push_str("</tbody>\n</table>\n");
    out.push_str(&feed_link(
        &tag_feed_address(&tag.slug),
        "The feed of this tag",
    ));
    out.push_str(&pager(
        &format!("/tags/{}", client::encode_path(&tag.slug)),
        &topics.page,
    ));
    out
}

fn follow_form(chrome: &Chrome, tag: &Tag) -> String {
    format!(
        concat!(
            "<form class=\"follow\" method=\"post\" action=\"/tags/{slug}/{doing}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<button type=\"submit\">{words}</button>\n",
            "</form>\n",
        ),
        slug = escape(&tag.slug),
        doing = if tag.following { "unfollow" } else { "follow" },
        token = escape(&chrome.token),
        words = if tag.following {
            "Leave this tag"
        } else {
            "Follow this tag"
        },
    )
}

fn describe_form(chrome: &Chrome, tag: &Tag) -> String {
    format!(
        concat!(
            "<form class=\"describe\" method=\"post\" action=\"/tags/{slug}/describe\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"description\">What it is for</label>\n",
            "<input id=\"description\" name=\"description\" type=\"text\" ",
            "value=\"{words}\" maxlength=\"200\"></p>\n",
            "<p><label for=\"means\">The tag it means</label>\n",
            "<input id=\"means\" name=\"means\" type=\"text\" value=\"{means}\" ",
            "maxlength=\"64\"></p>\n",
            "<p><button type=\"submit\">Say it</button></p>\n",
            "</form>\n",
        ),
        slug = escape(&tag.slug),
        token = escape(&chrome.token),
        words = escape(tag.description.as_deref().unwrap_or("")),
        means = escape(tag.means.as_deref().unwrap_or("")),
    )
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

pub fn search_page(criteria: &Criteria, hits: Option<&[Hit]>) -> String {
    let mut out = format!(
        concat!(
            "<h2>Search</h2>\n",
            "<form class=\"search-form\" method=\"get\" action=\"/search\">\n",
            "<p><label for=\"q\">Words</label>\n",
            "<input id=\"q\" name=\"q\" type=\"search\" value=\"{words}\" required></p>\n",
            "<p><label for=\"scope\">Narrow to</label>\n",
            "<select id=\"scope\" name=\"scope\">{scopes}</select></p>\n",
            "<p><label for=\"order\">Order</label>\n",
            "<select id=\"order\" name=\"order\">{orders}</select></p>\n",
            "<p><button type=\"submit\">Search</button></p>\n",
            "</form>\n",
        ),
        words = escape(criteria.query()),
        scopes = scope_options(criteria.scope()),
        orders = order_options(criteria.order()),
    );
    let Some(hits) = hits else {
        return out;
    };
    out.push_str(&narrowing(criteria));
    out.push_str(&format!(
        "<h3>{count}</h3>\n",
        count = escape(&hit_count(hits.len())),
    ));
    if hits.is_empty() {
        out.push_str("<p class=\"none\">Nothing was written for those words.</p>\n");
        return out;
    }
    out.push_str("<ol class=\"hits\">\n");
    for hit in hits {
        out.push_str(&hit_item(hit));
    }
    out.push_str("</ol>\n");
    out
}

fn narrowing(criteria: &Criteria) -> String {
    let mut kinds = String::from("<p class=\"kinds\">Narrow to:");
    for scope in Scope::ALL {
        kinds.push_str(&choice(
            &crate::search_address(&Criteria::new(criteria.query(), scope, criteria.order())),
            scope.words(),
            scope == criteria.scope(),
        ));
    }
    kinds.push_str("</p>\n");
    let mut orders = String::from("<p class=\"orders\">Order:");
    for order in Order::ALL {
        orders.push_str(&choice(
            &crate::search_address(&Criteria::new(criteria.query(), criteria.scope(), order)),
            order.words(),
            order == criteria.order(),
        ));
    }
    orders.push_str("</p>\n");
    format!("<nav class=\"narrowing\" aria-label=\"Narrowing\">\n{kinds}{orders}</nav>\n")
}

fn choice(address: &str, words: &str, current: bool) -> String {
    format!(
        " <a href=\"{address}\"{mark}>{words}</a>",
        address = escape(address),
        mark = if current {
            " class=\"current\" aria-current=\"true\""
        } else {
            ""
        },
        words = escape(words),
    )
}

fn hit_count(total: usize) -> String {
    match total {
        1 => "1 hit".to_owned(),
        other => format!("{other} hits"),
    }
}

fn scope_options(current: Scope) -> String {
    let mut out = String::new();
    for scope in Scope::ALL {
        out.push_str(&format!(
            "<option value=\"{name}\"{selected}>{words}</option>",
            name = scope.label(),
            words = escape(scope.words()),
            selected = if scope == current { " selected" } else { "" },
        ));
    }
    out
}

fn order_options(current: Order) -> String {
    let mut out = String::new();
    for order in Order::ALL {
        out.push_str(&format!(
            "<option value=\"{name}\"{selected}>{words}</option>",
            name = order.label(),
            words = escape(order.words()),
            selected = if order == current { " selected" } else { "" },
        ));
    }
    out
}

fn hit_item(hit: &Hit) -> String {
    match hit {
        Hit::Topic(topic) => format!(
            concat!(
                "<li class=\"hit hit-topic\"><span class=\"kind\">Subject</span> ",
                "<a href=\"/topics/{id}\">{title}</a> ",
                "<span class=\"byline\">by <span class=\"writer\">{author}</span> at ",
                "<time datetime=\"{stamp}\">{shown}</time> in ",
                "<a href=\"/sections/{section}\">{section}</a></span></li>\n",
            ),
            id = escape(&topic.id),
            title = escape(&topic.title),
            author = escape(&topic.author_username),
            stamp = escape(&topic.created_at),
            shown = escape(&shown_date(&topic.created_at)),
            section = escape(&topic.section_slug),
        ),
        Hit::Comment(remark) => format!(
            concat!(
                "<li class=\"hit hit-comment\"><span class=\"kind\">Remark</span> ",
                "<a href=\"/topics/{topic}#remark-{id}\">{said}</a> ",
                "<span class=\"byline\">by <span class=\"writer\">{author}</span> at ",
                "<time datetime=\"{stamp}\">{shown}</time></span></li>\n",
            ),
            topic = escape(&remark.topic_id),
            id = escape(&remark.id),
            said = escape(&excerpt(&remark.body)),
            author = escape(&remark.author_username),
            stamp = escape(&remark.created_at),
            shown = escape(&shown_date(&remark.created_at)),
        ),
    }
}

pub fn pager(address: &str, page: &PageInfo) -> String {
    let base = escape(address);
    let mut out = format!(
        "<nav class=\"pager\">\n<p>Page {number} of {pages}</p>\n<ul>\n",
        number = page.number,
        pages = page.total_pages,
    );
    if page.has_previous {
        out.push_str(&format!(
            "<li><a rel=\"prev\" href=\"{base}?page={previous}\">Previous page</a></li>\n",
            previous = page.number.saturating_sub(1),
        ));
    }
    if page.has_next {
        out.push_str(&format!(
            "<li><a rel=\"next\" href=\"{base}?page={next}\">Next page</a></li>\n",
            next = page.number.saturating_add(1),
        ));
    }
    out.push_str("</ul>\n</nav>\n");
    out
}

pub fn subject_page(subject: &Subject, comments: &Paged<Comment>) -> String {
    let mut out = format!(
        concat!(
            "<article class=\"subject\">\n",
            "<h2>{title}</h2>\n",
            "<p class=\"byline\">Written by <span class=\"writer\">{author}</span> at ",
            "<time datetime=\"{stamp}\">{shown}</time> in ",
            "<a href=\"/sections/{section}\">{section}</a>{marks}</p>\n",
            "<div class=\"remark-text\">{body}</div>\n",
            "<p class=\"tags\">Tags: {tags}</p>\n",
            "<p><a href=\"/sections/{section}\">Back to the section</a></p>\n",
            "</article>\n",
            "<h3>{count}</h3>\n",
        ),
        title = escape(&subject.title),
        author = escape(&subject.author_username),
        stamp = escape(&subject.created_at),
        shown = escape(&shown_date(&subject.created_at)),
        section = escape(&subject.section_slug),
        marks = subject_marks(subject),
        body = markup::render(&subject.body),
        tags = tag_links(&subject.tags),
        count = escape(&remark_count(comments.page.total)),
    );
    out.push_str(&comment_list(&comments.items));
    out.push_str(&pager(
        &format!("/topics/{}", client::encode_path(&subject.id)),
        &comments.page,
    ));
    out
}

fn remark_count(total: u64) -> String {
    match total {
        1 => "1 remark".to_owned(),
        other => format!("{other} remarks"),
    }
}

fn subject_marks(subject: &Subject) -> String {
    let mut marks = Vec::new();
    if subject.sticky {
        marks.push("pinned".to_owned());
    }
    if subject.resolved {
        marks.push("resolved".to_owned());
    }
    if subject.pending {
        marks.push("awaiting a look".to_owned());
    }
    if subject.draft {
        marks.push("draft".to_owned());
    }
    if subject.minor {
        marks.push("minor".to_owned());
    }
    if subject.edited {
        marks.push("edited".to_owned());
    }
    if subject.deleted {
        marks.push(match &subject.deleted_reason {
            Some(reason) => format!("removed: {reason}"),
            None => "removed".to_owned(),
        });
    }
    if marks.is_empty() {
        return String::new();
    }
    format!(
        " <span class=\"marks\">({})</span>",
        escape(&marks.join(", "))
    )
}

pub fn comment_list(comments: &[Comment]) -> String {
    let branches = client::thread(comments);
    if branches.is_empty() {
        return "<p>No remarks here yet.</p>\n".to_owned();
    }
    let mut out = String::from("<ol class=\"remarks\">\n");
    let mut before: Option<usize> = None;
    for branch in &branches {
        match before {
            None => {}
            Some(last) if branch.depth > last => out.push_str("<ol>\n"),
            Some(last) if branch.depth < last => {
                for _ in branch.depth..last {
                    out.push_str("</ol>\n</li>\n");
                }
                out.push_str("</li>\n");
            }
            Some(_) => out.push_str("</li>\n"),
        }
        out.push_str(&remark_item(
            &branch.comment,
            answered(&branch.comment, comments),
        ));
        before = Some(branch.depth);
    }
    if let Some(last) = before {
        for _ in 0..last {
            out.push_str("</ol>\n</li>\n");
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ol>\n");
    out
}

fn remark_item(comment: &Comment, answered: Option<&Comment>) -> String {
    let out = format!(
        "<li class=\"remark\" id=\"remark-{id}\">\n<p class=\"byline\"><span class=\"writer\">{author}</span> <time datetime=\"{stamp}\">{shown}</time>{marks}</p>\n{quote}{body}\n",
        id = escape(&comment.id),
        author = escape(&comment.author_username),
        stamp = escape(&comment.created_at),
        shown = escape(&shown_date(&comment.created_at)),
        marks = comment_marks(comment),
        quote = answered.map(quote_of).unwrap_or_default(),
        body = remark_body(comment),
    );
    out
}

fn answered<'a>(comment: &'a Comment, known: &'a [Comment]) -> Option<&'a Comment> {
    let parent = comment.parent_id.as_deref()?;
    known.iter().find(|other| other.id == parent)
}

fn quote_of(parent: &Comment) -> String {
    let said = match (parent.ignored, parent.deleted, &parent.deleted_reason) {
        (true, _, _) => return String::new(),
        (false, true, Some(reason)) => format!("Removed: {reason}"),
        (false, true, None) => "Removed.".to_owned(),
        (false, false, _) => excerpt(&parent.body),
    };
    if said.is_empty() {
        return String::new();
    }
    format!(
        "<blockquote class=\"answer\"><p><a href=\"#remark-{id}\">{author} wrote</a>: {said}</p></blockquote>\n",
        id = escape(&parent.id),
        author = escape(&parent.author_username),
        said = escape(&said),
    )
}

const QUOTE_WORDS: usize = 120;

fn excerpt(body: &str) -> String {
    let line = body
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let cut: String = line.chars().take(QUOTE_WORDS).collect();
    if cut.chars().count() < line.chars().count() {
        format!("{cut}...")
    } else {
        cut
    }
}

fn remark_body(comment: &Comment) -> String {
    if comment.ignored {
        return "<div class=\"remark-text\">Kept from you.</div>".to_owned();
    }
    if comment.deleted {
        return match &comment.deleted_reason {
            Some(reason) => format!(
                "<div class=\"remark-text\">Removed: {}</div>",
                escape(reason)
            ),
            None => "<div class=\"remark-text\">Removed.</div>".to_owned(),
        };
    }
    format!(
        "<div class=\"remark-text\">{}</div>",
        markup::render(&comment.body)
    )
}

fn comment_marks(comment: &Comment) -> String {
    if !comment.edited {
        return String::new();
    }
    " <span class=\"marks\">(edited)</span>".to_owned()
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

pub fn theme_picker(current: Theme, return_to: &str, token: &str) -> String {
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
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<button type=\"submit\">Apply</button>\n",
            "</form>\n"
        ),
        options = options,
        return_to = escape(return_to),
        token = escape(token),
    )
}

pub fn register_page(chrome: &Chrome, problem: Option<&str>) -> String {
    format!(
        concat!(
            "<h2>Register</h2>\n",
            "{problem}",
            "<form class=\"account-form\" method=\"post\" action=\"/register\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"username\">Name</label>\n",
            "<input id=\"username\" name=\"username\" type=\"text\" required autocomplete=\"username\"></p>\n",
            "<p><label for=\"email\">Address</label>\n",
            "<input id=\"email\" name=\"email\" type=\"email\" required autocomplete=\"email\"></p>\n",
            "<p><label for=\"password\">Password</label>\n",
            "<input id=\"password\" name=\"password\" type=\"password\" required autocomplete=\"new-password\"></p>\n",
            "<p><label for=\"invitation\">Invitation</label>\n",
            "<input id=\"invitation\" name=\"invitation\" type=\"text\" autocomplete=\"off\"></p>\n",
            "<p><button type=\"submit\">Register</button></p>\n",
            "</form>\n",
            "<p><a href=\"/sign-in\">I already have an account</a></p>\n"
        ),
        problem = problem_paragraph(problem),
        token = escape(&chrome.token),
    )
}

pub fn sign_in_page(chrome: &Chrome, problem: Option<&str>) -> String {
    format!(
        concat!(
            "<h2>Sign in</h2>\n",
            "{problem}",
            "<form class=\"account-form\" method=\"post\" action=\"/sign-in\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<input type=\"hidden\" name=\"return_to\" value=\"{return_to}\">\n",
            "<p><label for=\"username\">Name</label>\n",
            "<input id=\"username\" name=\"username\" type=\"text\" required autocomplete=\"username\"></p>\n",
            "<p><label for=\"password\">Password</label>\n",
            "<input id=\"password\" name=\"password\" type=\"password\" required autocomplete=\"current-password\"></p>\n",
            "<p><button type=\"submit\">Sign in</button></p>\n",
            "</form>\n",
            "<p><a href=\"/register\">I have no account yet</a></p>\n"
        ),
        problem = problem_paragraph(problem),
        token = escape(&chrome.token),
        return_to = escape(&chrome.return_to),
    )
}

pub fn profile_page(
    chrome: &Chrome,
    profile: &Profile,
    view: &ProfileView,
    problem: Option<&str>,
) -> String {
    format!(
        concat!(
            "<h2>{name}</h2>\n",
            "{picture}",
            "<p class=\"standing\">Score {score} &#183; {role}</p>\n",
            "{bio}",
            "{form}",
        ),
        name = escape(&profile.username),
        picture = profile_picture(profile, view.has_avatar),
        score = profile.score,
        role = escape(&profile.role),
        bio = profile_bio(profile.bio.as_deref()),
        form = bio_form(chrome, profile, view.own, problem),
    )
}

fn profile_picture(profile: &Profile, has_avatar: bool) -> String {
    if !has_avatar {
        return String::new();
    }
    format!(
        concat!(
            "<p class=\"picture\">\n",
            "<img class=\"avatar\" src=\"/u/{name}/avatar\" alt=\"The picture of {shown}\">\n",
            "</p>\n"
        ),
        name = escape(&profile.username),
        shown = escape(&profile.username),
    )
}

fn profile_bio(bio: Option<&str>) -> String {
    match bio.map(str::trim).filter(|words| !words.is_empty()) {
        Some(words) => format!(
            "<div class=\"bio\"><p>{words}</p></div>\n",
            words = escape(words).replace('\n', "<br>\n")
        ),
        None => "<div class=\"bio\"><p>No words about themselves yet.</p></div>\n".to_owned(),
    }
}

fn bio_form(chrome: &Chrome, profile: &Profile, own: bool, problem: Option<&str>) -> String {
    if !own {
        return String::new();
    }
    let current = profile.bio.as_deref().unwrap_or_default();
    format!(
        concat!(
            "<h3>About you</h3>\n",
            "{problem}",
            "<form class=\"bio-form\" method=\"post\" action=\"/u/{name}/bio\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"bio\">About you</label>\n",
            "<textarea id=\"bio\" name=\"bio\" rows=\"5\" cols=\"60\" maxlength=\"500\">{current}</textarea></p>\n",
            "<p><button type=\"submit\">Save</button></p>\n",
            "</form>\n"
        ),
        problem = problem_paragraph(problem),
        name = escape(&profile.username),
        token = escape(&chrome.token),
        current = escape(current),
    )
}

fn problem_paragraph(problem: Option<&str>) -> String {
    match problem {
        Some(words) => format!("<p class=\"problem\">{words}</p>\n", words = escape(words)),
        None => String::new(),
    }
}

pub fn password_page(chrome: &Chrome, problem: Option<&str>) -> String {
    form_page(
        "Change password",
        "/settings/password",
        &chrome.token,
        &format!(
            "{}{}",
            field(
                "current_password",
                "Current password",
                "password",
                "current-password",
                ""
            ),
            field(
                "new_password",
                "New password",
                "password",
                "new-password",
                ""
            )
        ),
        "Change password",
        problem,
    )
}

pub fn email_page(chrome: &Chrome, problem: Option<&str>) -> String {
    format!(
        concat!(
            "<h2>Change the address on file</h2>\n",
            "{problem}",
            "<form class=\"account-form\" method=\"post\" action=\"/settings/email\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "{address}",
            "<p><button type=\"submit\">Send the secret</button></p>\n",
            "</form>\n",
            "<h3>Confirm with the secret</h3>\n",
            "<form class=\"account-form\" method=\"post\" action=\"/settings/email/confirm\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "{secret}",
            "<p><button type=\"submit\">Confirm</button></p>\n",
            "</form>\n"
        ),
        problem = problem_paragraph(problem),
        token = escape(&chrome.token),
        address = field("email", "New address", "email", "email", ""),
        secret = field("code", "Secret", "text", "one-time-code", ""),
    )
}

pub fn forgot_page(chrome: &Chrome, problem: Option<&str>) -> String {
    form_page(
        "Recover a password",
        "/forgot",
        &chrome.token,
        &field("email", "Address on file", "email", "email", ""),
        "Send the secret",
        problem,
    )
}

pub fn forgot_confirm_page(chrome: &Chrome, code: &str, problem: Option<&str>) -> String {
    form_page(
        "Choose a new password",
        "/forgot/confirm",
        &chrome.token,
        &format!(
            "{}{}",
            field("code", "Secret", "text", "one-time-code", code),
            field(
                "new_password",
                "New password",
                "password",
                "new-password",
                ""
            )
        ),
        "Set the password",
        problem,
    )
}

pub fn activate_page(chrome: &Chrome, code: &str, problem: Option<&str>) -> String {
    form_page(
        "Activate an account",
        "/activate",
        &chrome.token,
        &field("code", "Secret", "text", "one-time-code", code),
        "Activate",
        problem,
    )
}

pub fn deregister_page(chrome: &Chrome, problem: Option<&str>) -> String {
    format!(
        concat!(
            "<h2>Leave the board</h2>\n",
            "{problem}",
            "<p>Your name stays on what you wrote. Your account, its session and its ",
            "picture go away.</p>\n",
            "<form class=\"account-form\" method=\"post\" action=\"/settings/deregister\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><button type=\"submit\">Delete my account</button></p>\n",
            "</form>\n"
        ),
        problem = problem_paragraph(problem),
        token = escape(&chrome.token),
    )
}

fn field(id: &str, label: &str, kind: &str, autocomplete: &str, value: &str) -> String {
    format!(
        concat!(
            "<p><label for=\"{id}\">{label}</label>\n",
            "<input id=\"{id}\" name=\"{id}\" type=\"{kind}\" autocomplete=\"{auto}\"",
            " value=\"{value}\" required></p>\n"
        ),
        id = escape(id),
        label = escape(label),
        kind = kind,
        auto = autocomplete,
        value = escape(value),
    )
}

fn form_page(
    heading: &str,
    action: &str,
    token: &str,
    fields: &str,
    button: &str,
    problem: Option<&str>,
) -> String {
    format!(
        concat!(
            "<h2>{heading}</h2>\n",
            "{problem}",
            "<form class=\"account-form\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "{fields}",
            "<p><button type=\"submit\">{button}</button></p>\n",
            "</form>\n"
        ),
        heading = escape(heading),
        problem = problem_paragraph(problem),
        action = escape(action),
        token = escape(token),
        fields = fields,
        button = escape(button),
    )
}

pub fn message(chrome: &Chrome, heading: &str, detail: &str) -> String {
    page(
        chrome,
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
    if lower.contains("<script") || lower.contains("<noscript") || lower.contains("<iframe") {
        return false;
    }
    let bytes: Vec<char> = lower.chars().collect();
    let mut inside_tag = false;
    let mut tag_starts_at = 0;
    for (index, character) in bytes.iter().enumerate() {
        match character {
            '<' => {
                inside_tag = true;
                tag_starts_at = index;
                continue;
            }
            '>' => {
                if inside_tag && tag(&bytes, tag_starts_at..index + 1).contains("javascript:") {
                    return false;
                }
                inside_tag = false;
                continue;
            }
            ' ' if inside_tag => {}
            _ => continue,
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
    !inside_tag || !tag(&bytes, tag_starts_at..bytes.len()).contains("javascript:")
}

fn tag(bytes: &[char], span: std::ops::Range<usize>) -> String {
    bytes[span].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chrome(theme: Theme) -> Chrome {
        Chrome::new(theme, "token-value")
    }

    fn hit_topic(id: &str, title: &str, author: &str) -> Topic {
        Topic {
            id: id.to_owned(),
            section_slug: "general".to_owned(),
            title: title.to_owned(),
            author_username: author.to_owned(),
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

    fn hit_comment(id: &str, topic_id: &str, body: &str) -> Comment {
        Comment {
            id: id.to_owned(),
            topic_id: topic_id.to_owned(),
            parent_id: None,
            body: body.to_owned(),
            author_username: "bob".to_owned(),
            created_at: "2024-06-08T09:08:07Z".to_owned(),
            deleted: false,
            deleted_reason: None,
            edited: false,
            ignored: false,
        }
    }

    fn section(slug: &str, title: &str, may_post: bool) -> Section {
        Section {
            slug: slug.to_owned(),
            title: title.to_owned(),
            topics_score: "anyone".to_owned(),
            may_post,
        }
    }

    fn profile(username: &str, bio: Option<&str>) -> Profile {
        Profile {
            id: "7f2c".to_owned(),
            username: username.to_owned(),
            bio: bio.map(|words| words.to_owned()),
            score: 12,
            role: "user".to_owned(),
        }
    }

    fn own(has_avatar: bool) -> ProfileView {
        ProfileView {
            own: true,
            has_avatar,
        }
    }

    #[test]
    fn a_profile_page_shows_the_name_the_standing_and_the_words() {
        let page = profile_page(
            &chrome(Theme::Light),
            &profile("alice", Some("Rust and forums.")),
            &own(false),
            None,
        );
        assert!(page.contains("<h2>alice</h2>"), "{page}");
        assert!(page.contains("Score 12 &#183; user"), "{page}");
        assert!(page.contains("<p>Rust and forums.</p>"), "{page}");
    }

    #[test]
    fn words_the_account_wrote_about_itself_stay_words() {
        let page = profile_page(
            &chrome(Theme::Light),
            &profile("alice", Some("<script>alert(1)</script>")),
            &own(false),
            None,
        );
        assert!(!page.contains("<script"), "{page}");
        assert!(page.contains("&lt;script&gt;"), "{page}");
    }

    #[test]
    fn an_account_without_words_says_so() {
        let page = profile_page(
            &chrome(Theme::Light),
            &profile("alice", None),
            &own(false),
            None,
        );
        assert!(page.contains("No words about themselves yet."), "{page}");
    }

    #[test]
    fn the_form_to_change_the_words_is_offered_to_the_account_itself_only() {
        let alice = profile("alice", Some("hello"));
        let shown = profile_page(&chrome(Theme::Light), &alice, &own(false), None);
        assert!(shown.contains("action=\"/u/alice/bio\""), "{shown}");
        assert!(shown.contains("id=\"bio\" name=\"bio\""), "{shown}");
        assert!(shown.contains("value=\"token-value\""), "{shown}");
        assert!(shown.contains(">hello</textarea>"), "{shown}");
        let other = profile_page(
            &chrome(Theme::Light),
            &alice,
            &ProfileView {
                own: false,
                has_avatar: false,
            },
            None,
        );
        assert!(!other.contains("action=\"/u/alice/bio\""), "{other}");
    }

    #[test]
    fn a_refused_bio_comes_back_on_the_form() {
        let page = profile_page(
            &chrome(Theme::Light),
            &profile("alice", None),
            &own(false),
            Some("bio too long"),
        );
        assert!(
            page.contains("<p class=\"problem\">bio too long</p>"),
            "{page}"
        );
    }

    #[test]
    fn an_account_with_a_picture_shows_it_and_one_without_does_not() {
        let alice = profile("alice", None);
        let with = profile_page(&chrome(Theme::Light), &alice, &own(true), None);
        assert!(
            with.contains("<img class=\"avatar\" src=\"/u/alice/avatar\""),
            "{with}"
        );
        let without = profile_page(&chrome(Theme::Light), &alice, &own(false), None);
        assert!(!without.contains("class=\"avatar\""), "{without}");
    }

    #[test]
    fn escaping_takes_the_markup_out_of_text() {
        assert_eq!(escape("a<b>&c\"d'e"), "a&lt;b&gt;&amp;c&quot;d&#39;e");
    }

    #[test]
    fn a_page_is_a_whole_document_with_a_title_and_a_stylesheet() {
        let html = page(&chrome(Theme::Light), "Sections", "<p>body</p>");
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html lang=\"en\" class=\"theme-light\">"));
        assert!(html.contains("<title>Sections &#183; Forum</title>"));
        assert!(html.contains("href=\"/static/style.css\""));
        assert!(html.contains("<p>body</p>"));
        assert!(html.ends_with("</html>\n"));
    }

    #[test]
    fn a_page_shows_the_theme_it_was_rendered_with() {
        assert!(page(&chrome(Theme::Dark), "x", "").contains("class=\"theme-dark\""));
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
        let html = pager("/sections/general", &page);
        assert!(html.contains("<p>Page 2 of 2</p>"));
        assert!(html.contains("href=\"/sections/general?page=1\""));
        assert!(html.contains("rel=\"prev\""));
        assert!(html.contains("href=\"/sections/general?page=3\""));
        assert!(html.contains("rel=\"next\""));
    }

    #[test]
    fn the_first_page_offers_no_step_back() {
        let html = pager("/sections/general", &paged(vec![], 0).page);
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
            &chrome(Theme::Light),
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
        let html = theme_picker(Theme::Contrast, "/sections/general", "token-value");
        assert!(html.contains("name=\"token\""));
        for theme in Theme::ALL {
            assert!(html.contains(&format!("value=\"{}\"", theme.name())));
        }
        assert!(html.contains("value=\"contrast\" selected"));
        assert!(html.contains("value=\"/sections/general\""));
        assert!(html.contains("method=\"post\" action=\"/theme\""));
    }

    #[test]
    fn a_message_page_names_its_heading_and_its_detail() {
        let html = message(
            &chrome(Theme::Light),
            "Unavailable",
            "The board is not answering",
        );
        assert!(html.contains("<h2>Unavailable</h2>"));
        assert!(html.contains("The board is not answering"));
    }

    #[test]
    fn rendered_pages_carry_no_scripting() {
        let pages = [
            page(
                &chrome(Theme::Light),
                "Sections",
                &section_list(&[section("general", "General", true)]),
            ),
            message(&chrome(Theme::Dark), "Unavailable", "try again"),
        ];
        for html in pages {
            assert!(scripting_free(&html), "not scripting free: {html}");
        }
    }

    fn subject(title: &str, body: &str) -> Subject {
        Subject {
            id: "11111111-1111-1111-1111-111111111111".to_owned(),
            section_slug: "general".to_owned(),
            group_slug: None,
            title: title.to_owned(),
            body: body.to_owned(),
            tags: vec!["rust".to_owned()],
            author_username: "alice".to_owned(),
            created_at: "2024-06-07T10:11:12Z".to_owned(),
            deleted: false,
            deleted_reason: None,
            edited: false,
            postscore: 0,
            pending: false,
            draft: false,
            sticky: false,
            off_front: false,
            resolved: false,
            minor: false,
            open_reports: 0,
        }
    }

    fn remark(id: &str, author: &str, body: &str) -> Comment {
        Comment {
            id: id.to_owned(),
            topic_id: "11111111-1111-1111-1111-111111111111".to_owned(),
            parent_id: None,
            body: body.to_owned(),
            author_username: author.to_owned(),
            created_at: "2024-06-07T11:00:00Z".to_owned(),
            deleted: false,
            deleted_reason: None,
            edited: false,
            ignored: false,
        }
    }

    fn remarks(items: Vec<Comment>, total: u64) -> Paged<Comment> {
        Paged {
            items,
            page: PageInfo {
                number: 1,
                size: 25,
                total,
                total_pages: 1,
                has_next: false,
                has_previous: false,
            },
        }
    }

    #[test]
    fn a_subject_page_shows_the_remark_and_the_remarks_under_it() {
        let html = subject_page(
            &subject("First subject", "The opening remark"),
            &remarks(
                vec![
                    remark("1", "bob", "A reply"),
                    remark("2", "carol", "Another"),
                ],
                2,
            ),
        );
        assert!(html.contains("<h2>First subject</h2>"));
        assert!(html.contains(">alice<"));
        assert!(html.contains("datetime=\"2024-06-07T10:11:12Z\""));
        assert!(html.contains("The opening remark"));
        assert!(html.contains("href=\"/sections/general\""));
        assert!(html.contains("<h3>2 remarks</h3>"));
        assert!(html.contains("id=\"remark-1\""));
        assert!(html.contains(">bob<"));
        assert!(html.contains("A reply"));
        assert!(html.contains(">carol<"));
        assert!(html.contains("Another"));
    }

    #[test]
    fn one_remark_is_counted_in_the_singular() {
        let html = subject_page(
            &subject("First", "body"),
            &remarks(vec![remark("1", "bob", "x")], 1),
        );
        assert!(html.contains("<h3>1 remark</h3>"));
    }

    #[test]
    fn a_subject_with_no_remarks_says_so() {
        let html = subject_page(&subject("First", "body"), &remarks(vec![], 0));
        assert!(html.contains("No remarks here yet."));
        assert!(html.contains("<h3>0 remarks</h3>"));
    }

    #[test]
    fn what_a_subject_and_a_remark_are_carries_as_a_mark() {
        let mut pinned = subject("First", "body");
        pinned.sticky = true;
        pinned.edited = true;
        let html = subject_page(&pinned, &remarks(vec![], 0));
        assert!(html.contains("(pinned, edited)"));
        let mut edited = remark("1", "bob", "x");
        edited.edited = true;
        assert!(
            subject_page(&subject("First", "body"), &remarks(vec![edited], 1)).contains("(edited)")
        );
    }

    #[test]
    fn a_removed_remark_says_why_and_a_kept_one_says_so() {
        let mut removed = remark("1", "bob", "gone");
        removed.deleted = true;
        removed.deleted_reason = Some("off topic".to_owned());
        let mut kept = remark("2", "carol", "hidden");
        kept.ignored = true;
        let html = subject_page(&subject("First", "body"), &remarks(vec![removed, kept], 2));
        assert!(html.contains("Removed: off topic"));
        assert!(!html.contains(">gone<"));
        assert!(html.contains("Kept from you."));
        assert!(!html.contains(">hidden<"));
    }

    #[test]
    fn text_from_the_api_cannot_become_markup_in_a_subject_page() {
        let html = subject_page(
            &subject("<b>bold</b>", "<img src=1 onerror=alert(1)>"),
            &remarks(vec![remark("1", "<i>bob", "<script>go()</script>")], 1),
        );
        assert!(!html.contains("<b>"));
        assert!(!html.contains("<img"));
        assert!(!html.contains("<script"));
        assert!(html.contains("&lt;b&gt;bold&lt;/b&gt;"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn a_subject_written_in_markup_is_rendered() {
        let html = subject_page(
            &subject("First", "One.\n\nTwo *soft* lines"),
            &remarks(vec![], 0),
        );
        assert!(html.contains("<p>One.</p>"), "no paragraph: {html}");
        assert!(
            html.contains("<p>Two <em>soft</em> lines</p>"),
            "markup was not rendered: {html}"
        );
        assert!(!html.contains("*soft*"), "the source was left in: {html}");
    }

    #[test]
    fn a_remark_written_in_markup_is_rendered() {
        let html = subject_page(
            &subject("First", "body"),
            &remarks(vec![remark("1", "bob", "> spoken\n\n- one")], 1),
        );
        assert!(html.contains("<blockquote>"), "no quote: {html}");
        assert!(html.contains("<li>one</li>"), "no item: {html}");
        assert!(!html.contains("> spoken"), "the source was left in: {html}");
    }

    #[test]
    fn a_removed_remark_keeps_its_reason_as_text() {
        let mut removed = remark("1", "bob", "gone");
        removed.deleted = true;
        removed.deleted_reason = Some("*why*".to_owned());
        let html = subject_page(&subject("First", "body"), &remarks(vec![removed], 1));
        assert!(html.contains("Removed: *why*"), "the reason: {html}");
        assert!(!html.contains("<em>"));
    }

    #[test]
    fn a_subject_page_written_in_markup_carries_no_scripting() {
        let sources = [
            "<script>alert(1)</script>",
            "[go](javascript:alert(1))",
            "> <iframe src=\"/a\"></iframe>",
            "**<b onclick=\"go()\">x</b>**",
            "```\n<script>go()</script>\n```",
        ];
        for source in sources {
            let html = page(
                &chrome(Theme::Light),
                "First subject",
                &subject_page(
                    &subject("First", source),
                    &remarks(vec![remark("1", "bob", source)], 1),
                ),
            );
            assert!(
                scripting_free(&html),
                "not scripting free: {source} -> {html}"
            );
        }
    }

    #[test]
    fn a_reply_quotes_the_remark_it_answers() {
        let mut answer = remark("2", "carol", "I agree");
        answer.parent_id = Some("1".to_owned());
        let html = comment_list(&[remark("1", "bob", "First thought"), answer]);
        assert!(html.contains("class=\"answer\""), "no quote: {html}");
        assert!(
            html.contains("<a href=\"#remark-1\">bob wrote</a>: First thought"),
            "the writer and the words are missing: {html}"
        );
    }

    #[test]
    fn a_remark_that_answers_nothing_carries_no_quote() {
        let html = comment_list(&[remark("1", "bob", "First thought")]);
        assert!(!html.contains("class=\"answer\""), "a quote: {html}");
    }

    #[test]
    fn a_reply_to_a_remark_off_the_page_carries_no_quote() {
        let mut answer = remark("2", "carol", "I agree");
        answer.parent_id = Some("9".to_owned());
        let html = comment_list(&[remark("1", "bob", "First thought"), answer]);
        assert!(!html.contains("class=\"answer\""), "a quote: {html}");
        assert!(html.contains("I agree"), "the reply is missing: {html}");
    }

    #[test]
    fn a_quote_of_a_remark_that_was_taken_away_says_so() {
        let mut gone = remark("1", "bob", "First thought");
        gone.deleted = true;
        gone.deleted_reason = Some("off topic".to_owned());
        let mut answer = remark("2", "carol", "I agree");
        answer.parent_id = Some("1".to_owned());
        let html = comment_list(&[gone, answer]);
        assert!(
            html.contains("Removed: off topic"),
            "the reason is missing: {html}"
        );
        assert!(!html.contains("First thought"), "the text leaked: {html}");
    }

    #[test]
    fn a_long_remark_is_quoted_only_in_part() {
        let long = "x".repeat(400);
        let mut answer = remark("2", "carol", "I agree");
        answer.parent_id = Some("1".to_owned());
        let html = comment_list(&[remark("1", "bob", &long), answer]);
        let quote = html.split("class=\"answer\"").nth(1).unwrap_or_default();
        assert!(
            quote.contains(&format!("{}...", "x".repeat(120))),
            "not cut at 120: {quote}"
        );
        assert!(
            !quote.contains(&"x".repeat(121)),
            "the cut came too late: {quote}"
        );
        assert!(html.contains(&long), "the remark itself was cut: {html}");
    }

    #[test]
    fn an_account_named_in_a_remark_is_a_link() {
        let html = comment_list(&[remark("1", "bob", "ask @alice")]);
        assert!(
            html.contains("<a class=\"mention\" href=\"/u/alice\">@alice</a>"),
            "no link: {html}"
        );
    }

    #[test]
    fn a_reply_is_set_under_the_remark_it_answers_however_deep() {
        let mut answer = remark("2", "carol", "An answer");
        answer.parent_id = Some("1".to_owned());
        let mut deeper = remark("3", "dave", "Deeper still");
        deeper.parent_id = Some("2".to_owned());
        let html = comment_list(&[remark("1", "bob", "A reply"), answer, deeper]);
        let opened = html.matches("<ol").count();
        let closed = html.matches("</ol>").count();
        assert_eq!(
            opened, 3,
            "one list for the top and one per nesting: {html}"
        );
        assert_eq!(closed, 3, "every list is closed: {html}");
        let first = html.find("id=\"remark-1\"").unwrap();
        let second = html.find("id=\"remark-2\"").unwrap();
        let third = html.find("id=\"remark-3\"").unwrap();
        assert!(first < second);
        assert!(second < third);
        assert!(
            html[first..second].contains("<ol>"),
            "the answer opens a list inside its remark: {html}"
        );
        assert!(html[second..third].contains("<ol>"));
    }

    #[test]
    fn a_reply_comes_after_the_remark_it_answers_whatever_order_they_arrive_in() {
        let mut answer = remark("2", "carol", "An answer");
        answer.parent_id = Some("1".to_owned());
        let html = comment_list(&[answer, remark("1", "bob", "A reply")]);
        assert!(html.find("id=\"remark-1\"").unwrap() < html.find("id=\"remark-2\"").unwrap());
    }

    #[test]
    fn a_reply_whose_remark_is_not_on_this_page_is_still_shown() {
        let mut orphan = remark("2", "carol", "An answer");
        orphan.parent_id = Some("gone".to_owned());
        let html = comment_list(&[orphan]);
        assert!(html.contains("id=\"remark-2\""));
        assert!(html.contains("An answer"));
    }

    #[test]
    fn a_search_page_offers_the_words_it_was_given_and_every_narrowing() {
        let criteria = Criteria::new("adapters", Scope::Comments, Order::Oldest);
        let html = search_page(&criteria, None);
        assert!(html.contains("value=\"adapters\""), "{html}");
        assert!(html.contains("<option value=\"comments\" selected>Remarks</option>"));
        assert!(html.contains("<option value=\"oldest\" selected>Oldest first</option>"));
        assert!(html.contains("<option value=\"newest\">Newest first</option>"));
        assert!(html.contains("<option value=\"everything\">Everything</option>"));
        assert!(!html.contains("hits"));
    }

    #[test]
    fn a_search_page_keeps_the_words_of_a_hit_as_words() {
        let criteria = Criteria::new("<script>x</script>", Scope::Everything, Order::Relevance);
        let html = search_page(
            &criteria,
            Some(&[
                Hit::Topic(hit_topic("11", "<b>Ports</b>", "alice")),
                Hit::Comment(hit_comment(
                    "33",
                    "11",
                    "Adapters keep the <i>domain</i> clean",
                )),
            ]),
        );
        assert!(scripting_free(&html), "not scripting free: {html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
        assert!(html.contains("&lt;b&gt;Ports&lt;/b&gt;"), "{html}");
        assert!(html.contains("<h3>2 hits</h3>"), "{html}");
        assert!(html.contains("<li class=\"hit hit-topic\">"), "{html}");
        assert!(html.contains("<li class=\"hit hit-comment\">"), "{html}");
        assert!(html.contains("href=\"/topics/11#remark-33\""), "{html}");
    }

    #[test]
    fn a_remark_hit_shows_the_first_words_of_the_remark() {
        let criteria = Criteria::new("adapters", Scope::Comments, Order::Relevance);
        let long: String = "word ".repeat(200);
        let html = search_page(
            &criteria,
            Some(&[Hit::Comment(hit_comment("33", "11", &long))]),
        );
        assert!(html.contains("<h3>1 hit</h3>"), "{html}");
        assert!(html.contains("..."), "{html}");
        assert!(!html.contains(&long), "a long remark is cut short");
    }

    #[test]
    fn a_search_without_words_offers_no_narrowing() {
        let criteria = Criteria::new("", Scope::Everything, Order::Relevance);
        assert!(!search_page(&criteria, None).contains("Narrow to:"));
        assert!(search_page(&criteria, Some(&[])).contains("Narrow to:"));
    }

    #[test]
    fn a_search_that_found_nothing_says_so() {
        let criteria = Criteria::new("nothing", Scope::Everything, Order::Relevance);
        let html = search_page(&criteria, Some(&[]));
        assert!(html.contains("<h3>0 hits</h3>"), "{html}");
        assert!(
            html.contains("Nothing was written for those words."),
            "{html}"
        );
    }

    #[test]
    fn a_subject_page_carries_no_scripting() {
        let html = page(
            &chrome(Theme::Light),
            "First subject",
            &subject_page(
                &subject("First", "body"),
                &remarks(vec![remark("1", "bob", "A reply")], 1),
            ),
        );
        assert!(scripting_free(&html), "not scripting free: {html}");
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
        assert!(scripting_free("<p>&lt;img src=1 onerror=go()&gt;</p>"));
        assert!(!scripting_free("<a href=\"javascript:go()\">x</a>"));
        assert!(scripting_free("<p>the words javascript: on a page</p>"));
    }
}
