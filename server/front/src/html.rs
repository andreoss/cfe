use crate::markup;
use crate::theme::Theme;
use client::{
    AddressBlock, ArchiveMonth, Change, Comment, Criteria, Group, Hit, Image, Invitation,
    Notification, Order, PageInfo, Paged, Poll, Profile, Reactions, Scope, Section, Subject, Tag,
    Topic, Version,
};
use std::collections::BTreeMap;

pub const REACTIONS: [&str; 4] = ["like", "agree", "disagree", "thanks"];
pub const REPORTS_ADDRESS: &str = "/reports";
pub const ADMIN_ADDRESS: &str = "/admin";
pub const ADMIN_INVITATIONS_ADDRESS: &str = "/admin/invitations";
pub const ADMIN_BLOCKS_ADDRESS: &str = "/admin/blocks";

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
    standing: Option<String>,
    return_to: String,
}

impl Chrome {
    pub fn new(theme: Theme, token: &str) -> Self {
        Self {
            theme,
            token: token.to_owned(),
            account: None,
            standing: None,
            return_to: "/".to_owned(),
        }
    }

    pub fn account(mut self, name: &str, standing: &str) -> Self {
        self.account = Some(name.to_owned());
        self.standing = Some(standing.to_owned());
        self
    }

    pub fn standing(&self) -> Option<&str> {
        self.standing.as_deref()
    }

    pub fn return_to(mut self, address: &str) -> Self {
        self.return_to = address.to_owned();
        self
    }

    pub fn account_name(&self) -> Option<&str> {
        self.account.as_deref()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct ProfileView {
    pub own: bool,
    pub has_avatar: bool,
    pub ignored: Option<bool>,
    pub ban: Option<BanState>,
    pub moderator: bool,
    pub remark: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct BanState {
    pub banned: bool,
    pub reason: Option<String>,
    pub until: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct WarningView {
    pub reason: String,
    pub created_at: String,
    pub acknowledged: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ReportView {
    pub id: String,
    pub topic_id: String,
    pub comment_id: Option<String>,
    pub reporter: String,
    pub kind: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct GroupView {
    pub slug: String,
    pub name: String,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SectionView {
    pub slug: String,
    pub title: String,
    pub score: String,
    pub groups: Vec<GroupView>,
}

#[derive(Clone, Default)]
pub struct SubjectView {
    pub holding: bool,
    pub writer: bool,
    pub standing: Option<String>,
    pub token: String,
    pub groups: Vec<Group>,
    pub pictures: Vec<Image>,
    pub poll: Option<Poll>,
    pub reactions: Option<Reactions>,
    pub remark_reactions: BTreeMap<String, Reactions>,
}

impl SubjectView {
    fn moderator(&self) -> bool {
        self.standing.as_deref() == Some("moderator")
    }
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
        ways = wayfinding(chrome),
        account = account_box(chrome),
        picker = theme_picker(chrome.theme, &chrome.return_to, &chrome.token),
        body = body,
    )
}

fn wayfinding(chrome: &Chrome) -> String {
    let mut out = concat!(
        "<nav class=\"ways\" aria-label=\"Wayfinding\">\n",
        "<a href=\"/search\">Search</a>\n",
        "<a href=\"/archive\">Archive</a>\n",
        "<a href=\"/activity\">Activity</a>\n",
    )
    .to_owned();
    if chrome.account_name().is_some() {
        out.push_str(concat!(
            "<a href=\"/bookmarks\">Bookmarks</a>\n",
            "<a href=\"/watched\">Watched</a>\n",
            "<a href=\"/notifications\">Notifications</a>\n",
        ));
    }
    out.push_str("</nav>\n");
    out
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

pub fn archive_page(months: &[ArchiveMonth]) -> String {
    if months.is_empty() {
        return concat!(
            "<h2>Archive</h2>\n",
            "<p class=\"empty\">Nothing has been written yet.</p>\n"
        )
        .to_owned();
    }
    let mut out = concat!(
        "<h2>Archive</h2>\n",
        "<table class=\"archive\">\n<caption>Months and the subjects written in them</caption>\n",
        "<thead>\n<tr><th scope=\"col\">Month</th>",
        "<th scope=\"col\">Subjects</th></tr>\n</thead>\n<tbody>\n"
    )
    .to_owned();
    for month in months {
        out.push_str(&format!(
            "<tr><th scope=\"row\"><a href=\"{address}\">{name} {year}</a></th><td>{count}</td></tr>\n",
            address = escape(&archive_month_address(month.year, month.month)),
            name = month_name(month.month),
            year = month.year,
            count = escape(&subject_count(month.topics)),
        ));
    }
    out.push_str("</tbody>\n</table>\n");
    out
}

pub fn month_title(year: i32, month: u8) -> String {
    format!("{} {}", month_name(month), year)
}

pub fn archive_month_page(year: i32, month: u8, topics: &Paged<Topic>) -> String {
    let mut out = format!(
        concat!(
            "<h2>{name} {year}</h2>\n",
            "<h3>{count}</h3>\n",
            "<table class=\"topics\">\n<thead>\n<tr><th scope=\"col\">Subject</th>",
            "<th scope=\"col\">Written by</th><th scope=\"col\">Written at</th>",
            "<th scope=\"col\">Tags</th></tr>\n</thead>\n<tbody>\n"
        ),
        name = month_name(month),
        year = year,
        count = escape(&subject_count(topics.page.total)),
    );
    if topics.items.is_empty() {
        out.push_str("<tr><td colspan=\"4\">No subject was written in this month.</td></tr>\n");
    }
    out.push_str(&topic_rows(&topics.items));
    out.push_str("</tbody>\n</table>\n");
    out.push_str(&archive_link());
    out.push_str(&pager(&archive_month_address(year, month), &topics.page));
    out
}

pub fn activity_page(hits: &[Hit]) -> String {
    let mut out = format!(
        "<h2>Activity</h2>\n<h3>{count}</h3>\n",
        count = escape(&entry_count(hits.len())),
    );
    if hits.is_empty() {
        out.push_str("<p class=\"none\">Nothing has been written yet.</p>\n");
        return out;
    }
    out.push_str("<ol class=\"hits\">\n");
    for hit in hits {
        out.push_str(&hit_item(hit));
    }
    out.push_str("</ol>\n");
    out
}

fn entry_count(total: usize) -> String {
    match total {
        1 => "1 entry".to_owned(),
        other => format!("{other} entries"),
    }
}

pub fn bookmarks_page(topics: &Paged<Topic>) -> String {
    kept_page(
        "Bookmarks",
        "/bookmarks",
        "Nothing has been kept yet.",
        topics,
    )
}

pub fn watched_page(topics: &Paged<Topic>) -> String {
    kept_page("Watched", "/watched", "Nothing is watched yet.", topics)
}

pub fn notifications_page(
    notices: &Paged<Notification>,
    tags: &[String],
    chrome: &Chrome,
) -> String {
    let mut out = format!(
        "<h2>Notifications</h2>\n<h3>{count}</h3>\n",
        count = escape(&notice_count(notices.page.total)),
    );
    if notices.items.is_empty() {
        out.push_str("<p class=\"empty\">Nothing has happened yet.</p>\n");
    } else {
        out.push_str("<ol class=\"notices\">\n");
        for notice in &notices.items {
            out.push_str(&notice_item(notice, chrome));
        }
        out.push_str("</ol>\n");
        out.push_str(&pager("/notifications", &notices.page));
    }
    out.push_str("<h3>Followed tags</h3>\n");
    if tags.is_empty() {
        out.push_str("<p class=\"empty\">No tag is followed yet.</p>\n");
    } else {
        out.push_str(&format!(
            "<ul class=\"followed\">\n<li>{}</li>\n</ul>\n",
            tags.iter()
                .map(|tag| format!(
                    "<a href=\"/tags/{tag}\">{shown}</a>",
                    tag = escape(tag),
                    shown = escape(tag),
                ))
                .collect::<Vec<_>>()
                .join("</li>\n<li>")
        ));
    }
    out
}

fn notice_count(total: u64) -> String {
    match total {
        0 => "No notifications".to_owned(),
        1 => "1 notification".to_owned(),
        other => format!("{other} notifications"),
    }
}

fn notice_item(notice: &Notification, chrome: &Chrome) -> String {
    let mut out = format!(
        concat!(
            "<li class=\"notice {state}\">\n",
            "<p class=\"what\"><span class=\"actor\">{actor}</span> {did} in ",
            "<a href=\"/topics/{topic}\">{title}</a></p>\n",
            "<time datetime=\"{stamp}\">{shown}</time>\n",
        ),
        state = if notice.read { "read" } else { "unread" },
        actor = escape(&notice.actor_username),
        did = escape(&notice_words(&notice.kind)),
        topic = escape(&notice.topic_id),
        title = escape(&notice.topic_title),
        stamp = escape(&notice.created_at),
        shown = escape(&shown_date(&notice.created_at)),
    );
    if !notice.read {
        out.push_str(&format!(
            concat!(
                "<form class=\"mark-read\" method=\"post\" ",
                "action=\"/notifications/{id}/read\">\n",
                "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                "<button type=\"submit\">Mark read</button>\n",
                "</form>\n",
            ),
            id = escape(&notice.id),
            token = escape(&chrome.token),
        ));
    }
    out.push_str("</li>\n");
    out
}

fn notice_words(kind: &str) -> String {
    match kind {
        "reply" => "answered".to_owned(),
        "comment" => "remarked".to_owned(),
        "watch" => "remarked".to_owned(),
        "mention" => "named this account".to_owned(),
        other => other.to_owned(),
    }
}

fn kept_page(heading: &str, address: &str, empty: &str, topics: &Paged<Topic>) -> String {
    let mut out = format!(
        concat!(
            "<h2>{heading}</h2>\n",
            "<h3>{count}</h3>\n",
            "<table class=\"topics\">\n<thead>\n<tr><th scope=\"col\">Subject</th>",
            "<th scope=\"col\">Written by</th><th scope=\"col\">Written at</th>",
            "<th scope=\"col\">Tags</th></tr>\n</thead>\n<tbody>\n"
        ),
        heading = escape(heading),
        count = escape(&subject_count(topics.page.total)),
    );
    if topics.items.is_empty() {
        out.push_str(&format!(
            "<tr><td colspan=\"4\">{empty}</td></tr>\n",
            empty = escape(empty),
        ));
    }
    out.push_str(&topic_rows(&topics.items));
    out.push_str("</tbody>\n</table>\n");
    out.push_str(&pager(address, &topics.page));
    out
}

pub fn archive_month_address(year: i32, month: u8) -> String {
    format!("/archive/{year}/{month}")
}

fn archive_link() -> String {
    "<p class=\"back\"><a href=\"/archive\">The archive</a></p>\n".to_owned()
}

fn month_name(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
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

pub fn subject_page(subject: &Subject, comments: &Paged<Comment>, view: &SubjectView) -> String {
    let mut out = format!(
        concat!(
            "<article class=\"subject\">\n",
            "<h2>{title}</h2>\n",
            "<p class=\"byline\">Written by <span class=\"writer\">{author}</span> at ",
            "<time datetime=\"{stamp}\">{shown}</time> in ",
            "<a href=\"/sections/{section}\">{section}</a>{marks}</p>\n",
            "<div class=\"remark-text\">{body}</div>\n",
            "<p class=\"tags\">Tags: {tags}</p>\n",
            "{pictures}",
            "{actions}",
            "{life}",
            "{poll}",
            "{reactions}",
            "{report}",
            "<p class=\"links\"><a href=\"{history}\">What changed</a> | ",
            "<a href=\"{gallery}\">Pictures</a> | ",
            "<a href=\"/sections/{section}\">Back to the section</a></p>\n",
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
        pictures = subject_pictures(subject, view),
        gallery = escape(&pictures_address(&subject.id)),
        actions = subject_actions(subject, view),
        life = subject_life(subject, view),
        poll = subject_poll(subject, view),
        reactions = reaction_bar(&ReactionView {
            action: react_address(&subject.id),
            clear: clear_reaction_address(&subject.id),
            counts: view.reactions.as_ref(),
            token: &view.token,
            holding: view.holding,
        }),
        history = escape(&history_address(&subject.id)),
        count = escape(&remark_count(comments.page.total)),
        report = subject_report(subject, view),
    );
    out.push_str(&comment_list(&comments.items, view));
    out.push_str(&pager(
        &format!("/topics/{}", client::encode_path(&subject.id)),
        &comments.page,
    ));
    out
}

pub fn picture_page(subject: &Subject, pictures: &[Image], chrome: &Chrome) -> String {
    let mut out = format!(
        concat!(
            "<h2>Pictures of {title}</h2>\n",
            "<p class=\"links\"><a href=\"{subject}\">Back to the subject</a></p>\n",
        ),
        title = escape(&subject.title),
        subject = escape(&subject_address(&subject.id)),
    );
    if pictures.is_empty() {
        out.push_str("<p>No pictures here yet.</p>\n");
    } else {
        out.push_str("<ul class=\"pictures\">\n");
        for picture in pictures {
            out.push_str(&format!(
                concat!(
                    "<li><img src=\"{src}\" alt=\"A picture put up by {by}\"> ",
                    "<span class=\"byline\">by <span class=\"writer\">{by}</span></span>",
                    " <form class=\"inline\" method=\"post\" action=\"{action}\">",
                    "<input type=\"hidden\" name=\"token\" value=\"{token}\">",
                    "<button type=\"submit\">Take down</button></form></li>\n",
                ),
                src = escape(&picture_address(&subject.id, &picture.id)),
                by = escape(&picture.uploaded_by),
                action = escape(&picture_removal_address(&subject.id, &picture.id)),
                token = escape(&chrome.token),
            ));
        }
        out.push_str("</ul>\n");
    }
    out.push_str(&format!(
        concat!(
            "<form class=\"picture-form\" method=\"post\" enctype=\"multipart/form-data\" ",
            "action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"picture\">A picture</label>\n",
            "<input id=\"picture\" name=\"picture\" type=\"file\" accept=\"image/*\" required></p>\n",
            "<p><button type=\"submit\">Put up</button></p>\n",
            "</form>\n",
        ),
        action = escape(&pictures_address(&subject.id)),
        token = escape(&chrome.token),
    ));
    out
}

pub fn subject_address(id: &str) -> String {
    format!("/topics/{}", client::encode_path(id))
}

pub fn pictures_address(id: &str) -> String {
    format!("/topics/{}/images", client::encode_path(id))
}

fn subject_pictures(subject: &Subject, view: &SubjectView) -> String {
    if view.pictures.is_empty() {
        return String::new();
    }
    let mut out = String::from("<ul class=\"pictures\">\n");
    for picture in &view.pictures {
        out.push_str(&format!(
            concat!(
                "<li><img src=\"{src}\" alt=\"A picture put up by {by}\"> ",
                "<span class=\"byline\">by <span class=\"writer\">{by}</span></span>{take}</li>\n",
            ),
            src = escape(&picture_address(&subject.id, &picture.id)),
            by = escape(&picture.uploaded_by),
            take = match view.holding {
                true => format!(
                    concat!(
                        " <form class=\"inline\" method=\"post\" action=\"{action}\">",
                        "<input type=\"hidden\" name=\"token\" value=\"{token}\">",
                        "<button type=\"submit\">Take down</button></form>",
                    ),
                    action = escape(&picture_removal_address(&subject.id, &picture.id)),
                    token = escape(&view.token),
                ),
                false => String::new(),
            },
        ));
    }
    out.push_str("</ul>\n");
    out
}

fn subject_poll(subject: &Subject, view: &SubjectView) -> String {
    let Some(poll) = view.poll.as_ref() else {
        return match view.holding && view.writer {
            true => format!(
                "<p class=\"links\"><a href=\"{}\">Put a poll up</a></p>\n",
                escape(&poll_form_address(&subject.id))
            ),
            false => String::new(),
        };
    };
    let mut out = format!(
        "<section class=\"poll\">\n<h3>{question}</h3>\n",
        question = escape(&poll.question),
    );
    if view.holding {
        out.push_str(&format!(
            "<form method=\"post\" action=\"{action}\">\n<input type=\"hidden\" name=\"token\" value=\"{token}\">\n<ul class=\"options\">\n",
            action = escape(&vote_address(&subject.id)),
            token = escape(&view.token),
        ));
        for option in &poll.options {
            out.push_str(&format!(
                concat!(
                    "<li><button type=\"submit\" name=\"option\" value=\"{id}\"{mine}>",
                    "{text} ({votes})</button></li>\n",
                ),
                id = escape(&option.id),
                mine = match poll.mine.as_deref() == Some(option.id.as_str()) {
                    true => " class=\"mine\"",
                    false => "",
                },
                text = escape(&option.text),
                votes = option.votes,
            ));
        }
        out.push_str("</ul>\n</form>\n");
    } else {
        out.push_str("<ul class=\"options\">\n");
        for option in &poll.options {
            out.push_str(&format!(
                "<li><button type=\"button\" disabled>{text} ({votes})</button></li>\n",
                text = escape(&option.text),
                votes = option.votes,
            ));
        }
        out.push_str("</ul>\n");
    }
    out.push_str(&format!(
        "<p class=\"byline\">{total}</p>\n</section>\n",
        total = escape(&vote_count(poll.total_votes)),
    ));
    out
}

fn vote_count(total: u64) -> String {
    match total {
        1 => "1 vote".to_owned(),
        other => format!("{other} votes"),
    }
}

pub struct ReactionView<'a> {
    pub action: String,
    pub clear: String,
    pub counts: Option<&'a Reactions>,
    pub token: &'a str,
    pub holding: bool,
}

pub fn reaction_bar(view: &ReactionView<'_>) -> String {
    let mine = view
        .counts
        .and_then(|reactions| reactions.mine.as_deref())
        .unwrap_or_default();
    let mut out = String::from("<div class=\"reactions\">\n");
    for kind in REACTIONS {
        let count = view
            .counts
            .and_then(|reactions| {
                reactions
                    .counts
                    .iter()
                    .find(|found| found.kind == kind)
                    .map(|found| found.count)
            })
            .unwrap_or(0);
        let label = format!("{kind} {count}");
        out.push_str(&match view.holding {
            true => {
                let (action, mine) = match mine == kind {
                    true => (&view.clear, " class=\"reaction mine\""),
                    false => (&view.action, " class=\"reaction\""),
                };
                format!(
                    concat!(
                        "<form class=\"inline\" method=\"post\" action=\"{action}\">",
                        "<input type=\"hidden\" name=\"token\" value=\"{token}\">",
                        "<input type=\"hidden\" name=\"kind\" value=\"{kind}\">",
                        "<button type=\"submit\"{mine}>{label}</button></form>",
                    ),
                    action = escape(action),
                    token = escape(view.token),
                    kind = escape(kind),
                    mine = mine,
                    label = escape(&label),
                )
            }
            false => format!(
                "<button type=\"button\" class=\"reaction\" disabled>{label}</button>",
                label = escape(&label),
            ),
        });
    }
    out.push_str("</div>\n");
    out
}

pub fn poll_form_address(id: &str) -> String {
    format!("/topics/{}/poll/new", client::encode_path(id))
}

pub fn vote_address(id: &str) -> String {
    format!("/topics/{}/poll/vote", client::encode_path(id))
}

pub fn react_address(id: &str) -> String {
    format!("/topics/{}/react", client::encode_path(id))
}

pub fn clear_reaction_address(id: &str) -> String {
    format!("/topics/{}/reactions/clear", client::encode_path(id))
}

pub fn remark_react_address(topic_id: &str, remark_id: &str) -> String {
    format!(
        "/topics/{}/comments/{}/react",
        client::encode_path(topic_id),
        client::encode_path(remark_id)
    )
}

pub fn remark_clear_address(topic_id: &str, remark_id: &str) -> String {
    format!(
        "/topics/{}/comments/{}/reactions/clear",
        client::encode_path(topic_id),
        client::encode_path(remark_id)
    )
}

pub fn poll_form_page(subject: &Subject, chrome: &Chrome, problem: Option<&str>) -> String {
    format!(
        concat!(
            "<h2>A poll on {title}</h2>\n",
            "{problem}",
            "<form class=\"panel\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"question\">The question</label>\n",
            "<input id=\"question\" name=\"question\" type=\"text\" maxlength=\"200\" required></p>\n",
            "<p><label for=\"options\">The ways to answer, one on each line</label>\n",
            "<textarea id=\"options\" name=\"options\" rows=\"4\" required></textarea></p>\n",
            "<p><button type=\"submit\">Put up</button></p>\n",
            "</form>\n",
            "<p class=\"links\"><a href=\"{subject}\">Back to the subject</a></p>\n",
        ),
        title = escape(&subject.title),
        problem = problem_paragraph(problem),
        action = escape(&poll_create_address(&subject.id)),
        token = escape(&chrome.token),
        subject = escape(&subject_address(&subject.id)),
    )
}

pub fn poll_create_address(id: &str) -> String {
    format!("/topics/{}/poll", client::encode_path(id))
}

fn subject_life(subject: &Subject, view: &SubjectView) -> String {
    if !view.holding {
        return String::new();
    }
    let mut out = String::new();
    if (subject.pending || subject.draft) && (view.writer || view.moderator()) {
        out.push_str(&flag_form(
            &publish_address(&subject.id),
            &view.token,
            "",
            "Publish",
        ));
    }
    if view.moderator() {
        out.push_str(&flag_form(
            &sticky_address(&subject.id),
            &view.token,
            match subject.sticky {
                true => "no",
                false => "yes",
            },
            match subject.sticky {
                true => "Unpin",
                false => "Pin",
            },
        ));
        out.push_str(&flag_form(
            &front_address(&subject.id),
            &view.token,
            match subject.off_front {
                true => "yes",
                false => "no",
            },
            match subject.off_front {
                true => "Put on the front page",
                false => "Keep off the front page",
            },
        ));
        out.push_str(&flag_form(
            &commit_address(&subject.id),
            &view.token,
            match subject.pending {
                true => "yes",
                false => "no",
            },
            match subject.pending {
                true => "Commit",
                false => "Commit no more",
            },
        ));
        out.push_str(&format!(
            concat!(
                "<form class=\"inline\" method=\"post\" action=\"{action}\">\n",
                "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                "<label for=\"postscore\">Score</label>\n",
                "<input id=\"postscore\" name=\"score\" type=\"number\" value=\"{score}\">\n",
                "<button type=\"submit\">Set</button>\n",
                "</form>\n",
            ),
            action = escape(&postscore_address(&subject.id)),
            token = escape(&view.token),
            score = subject.postscore,
        ));
        if !view.groups.is_empty() {
            out.push_str(&format!(
                concat!(
                    "<form class=\"inline\" method=\"post\" action=\"{action}\">\n",
                    "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                    "<label for=\"group\">Move to</label>\n",
                    "<select id=\"group\" name=\"group\">{options}</select>\n",
                    "<button type=\"submit\">Move</button>\n",
                    "</form>\n",
                ),
                action = escape(&move_address(&subject.id)),
                token = escape(&view.token),
                options = group_options(&view.groups, subject.group_slug.as_deref()),
            ));
        }
    }
    if view.moderator() || view.writer {
        out.push_str(&flag_form(
            &resolved_address(&subject.id),
            &view.token,
            match subject.resolved {
                true => "no",
                false => "yes",
            },
            match subject.resolved {
                true => "Open again",
                false => "Mark resolved",
            },
        ));
    }
    if out.is_empty() {
        return String::new();
    }
    format!("<div class=\"life\">\n{out}</div>\n")
}

fn flag_form(action: &str, token: &str, on: &str, label: &str) -> String {
    format!(
        concat!(
            "<form class=\"inline\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "{flag}<button type=\"submit\">{label}</button>\n",
            "</form>\n",
        ),
        action = escape(action),
        token = escape(token),
        flag = match on.is_empty() {
            true => String::new(),
            false => format!("<input type=\"hidden\" name=\"on\" value=\"{on}\">\n"),
        },
        label = escape(label),
    )
}

fn group_options(groups: &[Group], chosen: Option<&str>) -> String {
    let mut out = String::new();
    for group in groups {
        out.push_str(&format!(
            "<option value=\"{slug}\"{picked}>{name}</option>",
            slug = escape(&group.slug),
            picked = match chosen == Some(group.slug.as_str()) {
                true => " selected",
                false => "",
            },
            name = escape(&group.name),
        ));
    }
    out
}

pub fn publish_address(id: &str) -> String {
    format!("/topics/{}/publish", client::encode_path(id))
}

pub fn sticky_address(id: &str) -> String {
    format!("/topics/{}/sticky", client::encode_path(id))
}

pub fn front_address(id: &str) -> String {
    format!("/topics/{}/front", client::encode_path(id))
}

pub fn commit_address(id: &str) -> String {
    format!("/topics/{}/commit", client::encode_path(id))
}

pub fn postscore_address(id: &str) -> String {
    format!("/topics/{}/score", client::encode_path(id))
}

pub fn move_address(id: &str) -> String {
    format!("/topics/{}/group", client::encode_path(id))
}

pub fn resolved_address(id: &str) -> String {
    format!("/topics/{}/resolved", client::encode_path(id))
}

pub fn picture_address(topic_id: &str, image_id: &str) -> String {
    format!(
        "/topics/{}/images/{}",
        client::encode_path(topic_id),
        client::encode_path(image_id)
    )
}

pub fn picture_removal_address(topic_id: &str, image_id: &str) -> String {
    format!(
        "/topics/{}/images/{}/remove",
        client::encode_path(topic_id),
        client::encode_path(image_id)
    )
}

fn subject_actions(subject: &Subject, view: &SubjectView) -> String {
    if !view.holding {
        return String::new();
    }
    let mut links = vec![
        format!(
            "<a href=\"{}\">Answer</a>",
            escape(&reply_address(&subject.id))
        ),
        format!(
            "<a href=\"{}\">Change</a>",
            escape(&subject_edit_address(&subject.id))
        ),
    ];
    links.push(match subject.deleted {
        true => format!(
            "<a href=\"{}\">Bring back</a>",
            escape(&restore_address(&subject.id))
        ),
        false => format!(
            "<a href=\"{}\">Remove</a>",
            escape(&removal_form_address(&subject.id))
        ),
    });
    format!("<p class=\"actions\">{}</p>\n", links.join(" | "))
}

pub fn subject_form_page(section: &Section, chrome: &Chrome, problem: Option<&str>) -> String {
    format!(
        concat!(
            "<h2>Write a subject</h2>\n",
            "{problem}",
            "<form class=\"subject-form\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"title\">Title</label>\n",
            "<input id=\"title\" name=\"title\" type=\"text\" required maxlength=\"120\"></p>\n",
            "<p><label for=\"body\">Text</label>\n",
            "<textarea id=\"body\" name=\"body\" rows=\"12\" cols=\"60\" required></textarea></p>\n",
            "<p><label for=\"tags\">Tags</label>\n",
            "<input id=\"tags\" name=\"tags\" type=\"text\" autocomplete=\"off\"></p>\n",
            "<p><label for=\"draft\">Keep it as a draft</label>\n",
            "<input id=\"draft\" name=\"draft\" type=\"checkbox\" value=\"yes\"></p>\n",
            "<p><button type=\"submit\">Write</button></p>\n",
            "</form>\n"
        ),
        problem = problem_paragraph(problem),
        action = escape(&subject_form_address(&section.slug)),
        token = escape(&chrome.token),
    )
}

pub fn subject_form_address(slug: &str) -> String {
    format!("/sections/{}/post", client::encode_path(slug))
}

pub fn remark_form_address(id: &str) -> String {
    format!("/topics/{}/comments", client::encode_path(id))
}

pub fn reply_address(id: &str) -> String {
    format!("/topics/{}/reply", client::encode_path(id))
}

pub fn remark_form_page(
    subject: &Subject,
    answered: Option<&Comment>,
    chrome: &Chrome,
    problem: Option<&str>,
) -> String {
    format!(
        concat!(
            "<h2>Answer</h2>\n",
            "<p class=\"byline\">Answering <a href=\"/topics/{id}\">{title}</a> ",
            "by <span class=\"writer\">{author}</span></p>\n",
            "{quote}",
            "{problem}",
            "<form class=\"remark-form\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "{parent}",
            "<p><label for=\"body\">Text</label>\n",
            "<textarea id=\"body\" name=\"body\" rows=\"10\" cols=\"60\" required></textarea></p>\n",
            "<p><button type=\"submit\">Answer</button></p>\n",
            "</form>\n"
        ),
        id = escape(&subject.id),
        title = escape(&subject.title),
        author = escape(&subject.author_username),
        quote = answered.map(quote_of).unwrap_or_default(),
        problem = problem_paragraph(problem),
        action = escape(&remark_form_address(&subject.id)),
        token = escape(&chrome.token),
        parent = match answered {
            Some(remark) => format!(
                "<input type=\"hidden\" name=\"parent_id\" value=\"{}\">\n",
                escape(&remark.id)
            ),
            None => String::new(),
        },
    )
}

pub fn subject_edit_page(subject: &Subject, chrome: &Chrome, problem: Option<&str>) -> String {
    format!(
        concat!(
            "<h2>Change a subject</h2>\n",
            "{problem}",
            "<form class=\"subject-form\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"title\">Title</label>\n",
            "<input id=\"title\" name=\"title\" type=\"text\" required maxlength=\"120\" ",
            "value=\"{title}\"></p>\n",
            "<p><label for=\"body\">Text</label>\n",
            "<textarea id=\"body\" name=\"body\" rows=\"12\" cols=\"60\" required>",
            "{body}</textarea></p>\n",
            "<p><label for=\"tags\">Tags</label>\n",
            "<input id=\"tags\" name=\"tags\" type=\"text\" autocomplete=\"off\" ",
            "value=\"{tags}\"></p>\n",
            "<p><label for=\"minor\">A small mend</label>\n",
            "<input id=\"minor\" name=\"minor\" type=\"checkbox\" value=\"yes\"></p>\n",
            "<p><button type=\"submit\">Change</button></p>\n",
            "</form>\n"
        ),
        problem = problem_paragraph(problem),
        action = escape(&subject_edit_address(&subject.id)),
        token = escape(&chrome.token),
        title = escape(&subject.title),
        body = escape(&subject.body),
        tags = escape(&subject.tags.join(", ")),
    )
}

pub fn subject_edit_address(id: &str) -> String {
    format!("/topics/{}/edit", client::encode_path(id))
}

pub fn remark_edit_page(
    subject: &Subject,
    remark: &Comment,
    chrome: &Chrome,
    problem: Option<&str>,
) -> String {
    format!(
        concat!(
            "<h2>Change a remark</h2>\n",
            "<p class=\"byline\">In <a href=\"/topics/{topic}\">{title}</a> ",
            "by <span class=\"writer\">{author}</span></p>\n",
            "{problem}",
            "<form class=\"remark-form\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"body\">Text</label>\n",
            "<textarea id=\"body\" name=\"body\" rows=\"10\" cols=\"60\" required>",
            "{body}</textarea></p>\n",
            "<p><button type=\"submit\">Change</button></p>\n",
            "</form>\n"
        ),
        topic = escape(&subject.id),
        title = escape(&subject.title),
        author = escape(&remark.author_username),
        problem = problem_paragraph(problem),
        action = escape(&remark_edit_address(&subject.id, &remark.id)),
        token = escape(&chrome.token),
        body = escape(&remark.body),
    )
}

pub fn remark_edit_address(topic: &str, remark: &str) -> String {
    format!(
        "/topics/{}/comments/{}/edit",
        client::encode_path(topic),
        client::encode_path(remark)
    )
}

pub fn removal_page(subject: &Subject, chrome: &Chrome, problem: Option<&str>) -> String {
    removal_form(
        "Remove a subject",
        &removal_address(&subject.id),
        &chrome.token,
        problem,
        &format!(
            "<p class=\"byline\">Removing <a href=\"/topics/{id}\">{title}</a></p>\n",
            id = escape(&subject.id),
            title = escape(&subject.title),
        ),
    )
}

pub fn remark_removal_page(
    subject: &Subject,
    remark: &Comment,
    chrome: &Chrome,
    problem: Option<&str>,
) -> String {
    removal_form(
        "Remove a remark",
        &remark_removal_address(&subject.id, &remark.id),
        &chrome.token,
        problem,
        &format!(
            concat!(
                "<p class=\"byline\">Removing what <span class=\"writer\">{author}</span> ",
                "said in <a href=\"/topics/{topic}\">{title}</a></p>\n",
            ),
            author = escape(&remark.author_username),
            topic = escape(&subject.id),
            title = escape(&subject.title),
        ),
    )
}

fn removal_form(
    heading: &str,
    action: &str,
    token: &str,
    problem: Option<&str>,
    byline: &str,
) -> String {
    format!(
        concat!(
            "<h2>{heading}</h2>\n",
            "{byline}",
            "{problem}",
            "<form class=\"removal-form\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"reason\">Why</label>\n",
            "<input id=\"reason\" name=\"reason\" type=\"text\" required maxlength=\"200\"></p>\n",
            "<p><button type=\"submit\">Remove</button></p>\n",
            "</form>\n"
        ),
        heading = escape(heading),
        byline = byline,
        problem = problem_paragraph(problem),
        action = escape(action),
        token = escape(token),
    )
}

pub fn removal_address(id: &str) -> String {
    format!("/topics/{}/delete", client::encode_path(id))
}

pub fn removal_form_address(id: &str) -> String {
    format!("/topics/{}/remove", client::encode_path(id))
}

pub fn remark_removal_address(topic: &str, remark: &str) -> String {
    format!(
        "/topics/{}/comments/{}/delete",
        client::encode_path(topic),
        client::encode_path(remark)
    )
}

pub fn remark_removal_form_address(topic: &str, remark: &str) -> String {
    format!(
        "/topics/{}/comments/{}/remove",
        client::encode_path(topic),
        client::encode_path(remark)
    )
}

pub fn restore_page(subject: &Subject, chrome: &Chrome, problem: Option<&str>) -> String {
    restore_form(
        "Bring a subject back",
        &restore_address(&subject.id),
        &chrome.token,
        problem,
        &format!(
            "<p class=\"byline\">Bringing <a href=\"/topics/{id}\">{title}</a> back</p>\n",
            id = escape(&subject.id),
            title = escape(&subject.title),
        ),
    )
}

pub fn remark_restore_page(
    subject: &Subject,
    remark: &Comment,
    chrome: &Chrome,
    problem: Option<&str>,
) -> String {
    restore_form(
        "Bring a remark back",
        &remark_restore_address(&subject.id, &remark.id),
        &chrome.token,
        problem,
        &format!(
            concat!(
                "<p class=\"byline\">Bringing back what <span class=\"writer\">{author}</span> ",
                "said in <a href=\"/topics/{topic}\">{title}</a></p>\n",
            ),
            author = escape(&remark.author_username),
            topic = escape(&subject.id),
            title = escape(&subject.title),
        ),
    )
}

fn restore_form(
    heading: &str,
    action: &str,
    token: &str,
    problem: Option<&str>,
    byline: &str,
) -> String {
    format!(
        concat!(
            "<h2>{heading}</h2>\n",
            "{byline}",
            "{problem}",
            "<form class=\"restore-form\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><button type=\"submit\">Bring back</button></p>\n",
            "</form>\n"
        ),
        heading = escape(heading),
        byline = byline,
        problem = problem_paragraph(problem),
        action = escape(action),
        token = escape(token),
    )
}

pub fn restore_address(id: &str) -> String {
    format!("/topics/{}/restore", client::encode_path(id))
}

pub fn remark_restore_address(topic: &str, remark: &str) -> String {
    format!(
        "/topics/{}/comments/{}/restore",
        client::encode_path(topic),
        client::encode_path(remark)
    )
}

pub fn history_page(subject: &Subject, versions: &[Version]) -> String {
    let mut out = format!(
        concat!(
            "<h2>What changed</h2>\n",
            "<p class=\"byline\">Versions of <a href=\"/topics/{id}\">{title}</a></p>\n",
        ),
        id = escape(&subject.id),
        title = escape(&subject.title),
    );
    if versions.is_empty() {
        out.push_str("<p>Nothing was changed here yet.</p>\n");
        return out;
    }
    out.push_str("<ol class=\"versions\">\n");
    for version in versions {
        out.push_str(&format!(
            concat!(
                "<li><a href=\"{address}\"><time datetime=\"{stamp}\">{shown}</time></a> ",
                "by <span class=\"writer\">{editor}</span></li>\n",
            ),
            address = escape(&version_address(&subject.id, &version.id)),
            stamp = escape(&version.written_at),
            shown = escape(&shown_date(&version.written_at)),
            editor = escape(&version.editor),
        ));
    }
    out.push_str("</ol>\n");
    out
}

pub fn history_address(id: &str) -> String {
    format!("/topics/{}/history", client::encode_path(id))
}

pub fn difference_page(subject: &Subject, version: &Version, changes: &[Change]) -> String {
    let mut out = format!(
        concat!(
            "<h2>What changed at {shown}</h2>\n",
            "<p class=\"byline\"><span class=\"writer\">{editor}</span> in ",
            "<a href=\"/topics/{id}\">{title}</a></p>\n",
        ),
        shown = escape(&shown_date(&version.written_at)),
        editor = escape(&version.editor),
        id = escape(&subject.id),
        title = escape(&subject.title),
    );
    if changes.is_empty() {
        out.push_str("<p>Nothing is different in this version.</p>\n");
    } else {
        out.push_str("<ul class=\"changes\">\n");
        for change in changes {
            out.push_str(&format!(
                "<li class=\"change\"><span class=\"kind\">{kind}</span> {line}</li>\n",
                kind = escape(&change.kind),
                line = escape(&change.line),
            ));
        }
        out.push_str("</ul>\n");
    }
    out.push_str(&format!(
        "<p><a href=\"{history}\">Back to what changed</a></p>\n",
        history = escape(&history_address(&subject.id)),
    ));
    out
}

pub fn version_address(id: &str, version: &str) -> String {
    format!(
        "/topics/{}/history/{}",
        client::encode_path(id),
        client::encode_path(version)
    )
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

pub fn comment_list(comments: &[Comment], view: &SubjectView) -> String {
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
            view,
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

fn subject_report(subject: &Subject, view: &SubjectView) -> String {
    if !view.holding {
        return String::new();
    }
    report_form(
        "subject",
        &view.token,
        &report_address(&subject.id),
        "Report this subject",
    )
}

fn remark_item(comment: &Comment, answered: Option<&Comment>, view: &SubjectView) -> String {
    let out = format!(
        "<li class=\"remark\" id=\"remark-{id}\">\n<p class=\"byline\"><span class=\"writer\">{author}</span> <time datetime=\"{stamp}\">{shown}</time>{marks}</p>\n{quote}{body}\n{actions}{reactions}{report}",
        id = escape(&comment.id),
        author = escape(&comment.author_username),
        stamp = escape(&comment.created_at),
        shown = escape(&shown_date(&comment.created_at)),
        marks = comment_marks(comment),
        quote = answered.map(quote_of).unwrap_or_default(),
        body = remark_body(comment),
        actions = remark_actions(comment, view.holding),
        reactions = reaction_bar(&ReactionView {
            action: remark_react_address(&comment.topic_id, &comment.id),
            clear: remark_clear_address(&comment.topic_id, &comment.id),
            counts: view.remark_reactions.get(&comment.id),
            token: &view.token,
            holding: view.holding,
        }),
        report = remark_report(comment, view),
    );
    out
}

fn remark_report(comment: &Comment, view: &SubjectView) -> String {
    if !view.holding || comment.deleted {
        return String::new();
    }
    report_form(
        &format!("remark-{}", comment.id),
        &view.token,
        &remark_report_address(&comment.topic_id, &comment.id),
        "Report this remark",
    )
}

fn remark_actions(comment: &Comment, holding: bool) -> String {
    if !holding {
        return String::new();
    }
    if comment.deleted {
        return format!(
            "<p class=\"actions\"><a href=\"{}\">Bring back</a></p>\n",
            escape(&remark_restore_address(&comment.topic_id, &comment.id))
        );
    }
    format!(
        "<p class=\"actions\"><a href=\"{edit}\">Change</a> | <a href=\"{remove}\">Remove</a></p>\n",
        edit = escape(&remark_edit_address(&comment.topic_id, &comment.id)),
        remove = escape(&remark_removal_form_address(&comment.topic_id, &comment.id)),
    )
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
            "{moderation}",
            "{bio}",
            "{form}",
        ),
        name = escape(&profile.username),
        picture = profile_picture(profile, view.has_avatar),
        score = profile.score,
        role = escape(&profile.role),
        moderation = moderation_notes(chrome, &profile.username, view),
        bio = profile_bio(profile.bio.as_deref()),
        form = bio_form(chrome, profile, view.own, problem),
    )
}

fn moderation_notes(chrome: &Chrome, name: &str, view: &ProfileView) -> String {
    format!(
        "{}{}{}",
        ignore_note(view.ignored),
        ban_note(view.ban.as_ref()),
        moderation_forms(chrome, name, view),
    )
}

fn moderation_forms(chrome: &Chrome, name: &str, view: &ProfileView) -> String {
    let mut out = ignore_form(chrome, name, view.ignored);
    if !view.moderator {
        return out;
    }
    out.push_str(&ban_form(chrome, name, view.ban.as_ref()));
    out.push_str(&reason_form(
        chrome,
        &warn_address(name),
        "Warn",
        "Warn this account",
    ));
    out.push_str(&token_form(
        chrome,
        &promote_address(name),
        "Promote to moderator",
    ));
    out.push_str(&role_form(chrome, name));
    out.push_str(&remark_form(chrome, name, view.remark.as_deref()));
    out
}

fn ignore_form(chrome: &Chrome, name: &str, ignored: Option<bool>) -> String {
    match ignored {
        Some(true) => token_form(chrome, &stop_ignoring_address(name), "Stop ignoring"),
        Some(false) => token_form(chrome, &ignore_address(name), "Ignore this account"),
        None => String::new(),
    }
}

fn ban_form(chrome: &Chrome, name: &str, ban: Option<&BanState>) -> String {
    if ban.map(|ban| ban.banned) == Some(true) {
        return token_form(chrome, &lift_ban_address(name), "Lift the ban");
    }
    format!(
        concat!(
            "<form class=\"moderation\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"ban-reason\">Reason</label>\n",
            "<input id=\"ban-reason\" name=\"reason\" type=\"text\" required></p>\n",
            "<p><label for=\"ban-days\">Days</label>\n",
            "<input id=\"ban-days\" name=\"days\" type=\"number\" min=\"1\"></p>\n",
            "<p><button type=\"submit\">Ban this account</button></p>\n",
            "</form>\n",
        ),
        action = escape(&ban_address(name)),
        token = escape(&chrome.token),
    )
}

fn reason_form(chrome: &Chrome, action: &str, label: &str, legend: &str) -> String {
    format!(
        concat!(
            "<form class=\"moderation\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"{id}\">{legend}</label>\n",
            "<input id=\"{id}\" name=\"reason\" type=\"text\" required></p>\n",
            "<p><button type=\"submit\">{label}</button></p>\n",
            "</form>\n",
        ),
        action = escape(action),
        token = escape(&chrome.token),
        id = escape(&label.to_lowercase().replace(' ', "-")),
        legend = escape(legend),
        label = escape(label),
    )
}

fn token_form(chrome: &Chrome, action: &str, label: &str) -> String {
    format!(
        concat!(
            "<form class=\"moderation\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><button type=\"submit\">{label}</button></p>\n",
            "</form>\n",
        ),
        action = escape(action),
        token = escape(&chrome.token),
        label = escape(label),
    )
}

fn role_form(chrome: &Chrome, name: &str) -> String {
    format!(
        concat!(
            "<form class=\"moderation\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"role\">Standing</label>\n",
            "<select id=\"role\" name=\"role\">\n",
            "<option value=\"user\">user</option>\n",
            "<option value=\"corrector\">corrector</option>\n",
            "<option value=\"moderator\">moderator</option>\n",
            "</select></p>\n",
            "<p><button type=\"submit\">Set the standing</button></p>\n",
            "</form>\n",
        ),
        action = escape(&role_address(name)),
        token = escape(&chrome.token),
    )
}

fn remark_form(chrome: &Chrome, name: &str, remark: Option<&str>) -> String {
    let mut out = format!(
        concat!(
            "<form class=\"moderation\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><label for=\"remark-text\">A remark about this account</label>\n",
            "<textarea id=\"remark-text\" name=\"text\" rows=\"3\">{remark}</textarea></p>\n",
            "<p><button type=\"submit\">Keep the remark</button></p>\n",
            "</form>\n",
        ),
        action = escape(&remark_address(name)),
        token = escape(&chrome.token),
        remark = escape(remark.unwrap_or_default()),
    );
    if remark.is_some() {
        out.push_str(&token_form(
            chrome,
            &account_remark_removal_address(name),
            "Take the remark away",
        ));
    }
    out
}

pub fn ban_address(name: &str) -> String {
    format!("/u/{name}/ban")
}

pub fn lift_ban_address(name: &str) -> String {
    format!("/u/{name}/ban/lift")
}

pub fn warn_address(name: &str) -> String {
    format!("/u/{name}/warn")
}

pub fn promote_address(name: &str) -> String {
    format!("/u/{name}/promote")
}

pub fn role_address(name: &str) -> String {
    format!("/u/{name}/role")
}

pub fn ignore_address(name: &str) -> String {
    format!("/u/{name}/ignore")
}

pub fn stop_ignoring_address(name: &str) -> String {
    format!("/u/{name}/ignore/stop")
}

pub fn remark_address(name: &str) -> String {
    format!("/u/{name}/remark")
}

pub fn account_remark_removal_address(name: &str) -> String {
    format!("/u/{name}/remark/remove")
}

fn ignore_note(ignored: Option<bool>) -> String {
    let words = match ignored {
        Some(true) => "You ignore this account.",
        Some(false) => "You do not ignore this account.",
        None => return String::new(),
    };
    format!("<p class=\"ignore\">{words}</p>\n")
}

fn ban_note(ban: Option<&BanState>) -> String {
    let Some(ban) = ban else {
        return String::new();
    };
    if !ban.banned {
        return "<p class=\"ban\">This account is not banned.</p>\n".to_owned();
    }
    let mut words = "This account is banned".to_owned();
    if let Some(reason) = ban.reason.as_deref() {
        words.push_str(&format!(" for {}", escape(reason)));
    }
    if let Some(until) = ban.until.as_deref() {
        words.push_str(&format!(" until {}", escape(until)));
    }
    format!("<p class=\"ban\">{words}.</p>\n")
}

pub fn reports_page(chrome: &Chrome, reports: &[ReportView], page: &PageInfo) -> String {
    let mut out = "<h2>Open reports</h2>\n".to_owned();
    if reports.is_empty() {
        out.push_str("<p class=\"empty\">No open reports.</p>\n");
        return out;
    }
    out.push_str("<ol class=\"reports\">\n");
    for report in reports {
        out.push_str(&format!(
            concat!(
                "<li class=\"report\">\n",
                "<p class=\"target\"><a href=\"{target}\">{what}</a></p>\n",
                "<p class=\"reason\">{reason}</p>\n",
                "<p class=\"when\">{kind} &#183; by <span class=\"writer\">{reporter}</span> ",
                "at <time datetime=\"{stamp}\">{shown}</time></p>\n",
                "<form class=\"inline\" method=\"post\" action=\"{close}\">\n",
                "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                "<button type=\"submit\">Close</button>\n",
                "</form>\n",
                "</li>\n",
            ),
            target = escape(&report_target(report)),
            what = match report.comment_id {
                Some(_) => "A remark",
                None => "A subject",
            },
            reason = escape(&report.reason),
            kind = escape(&report.kind),
            reporter = escape(&report.reporter),
            stamp = escape(&report.created_at),
            shown = escape(&shown_date(&report.created_at)),
            close = escape(&report_close_address(&report.id)),
            token = escape(&chrome.token),
        ));
    }
    out.push_str("</ol>\n");
    out.push_str(&pager(REPORTS_ADDRESS, page));
    out
}

fn report_target(report: &ReportView) -> String {
    let subject = subject_address(&report.topic_id);
    match report.comment_id.as_deref() {
        Some(remark) => format!("{subject}#remark-{remark}"),
        None => subject,
    }
}

fn report_kinds() -> String {
    let kinds = [
        ("rule", "against the rules"),
        ("spelling", "spelling"),
        ("tag", "wrong tags"),
        ("group", "wrong group"),
    ];
    kinds
        .iter()
        .map(|(kind, words)| format!("<option value=\"{kind}\">{words}</option>\n"))
        .collect()
}

fn report_form(id: &str, token: &str, action: &str, legend: &str) -> String {
    format!(
        concat!(
            "<form class=\"report\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<label for=\"{id}-kind\">{legend}</label>\n",
            "<select id=\"{id}-kind\" name=\"kind\">{kinds}</select>\n",
            "<label for=\"{id}-reason\">Why</label>\n",
            "<input id=\"{id}-reason\" name=\"reason\" type=\"text\" required>\n",
            "<button type=\"submit\">Report</button>\n",
            "</form>\n",
        ),
        action = escape(action),
        token = escape(token),
        id = escape(id),
        legend = escape(legend),
        kinds = report_kinds(),
    )
}

pub fn report_address(topic: &str) -> String {
    format!("/topics/{}/report", client::encode_path(topic))
}

pub fn remark_report_address(topic: &str, remark: &str) -> String {
    format!(
        "/topics/{}/comments/{}/report",
        client::encode_path(topic),
        client::encode_path(remark)
    )
}

pub fn report_close_address(id: &str) -> String {
    format!("/reports/{}/close", client::encode_path(id))
}

pub fn section_rename_address(slug: &str) -> String {
    format!("/admin/sections/{}/rename", client::encode_path(slug))
}

pub fn section_score_address(slug: &str) -> String {
    format!("/admin/sections/{}/score", client::encode_path(slug))
}

pub fn section_make_group_address(slug: &str) -> String {
    format!("/admin/sections/{}/groups", client::encode_path(slug))
}

pub fn group_rename_address(section: &str, group: &str) -> String {
    format!(
        "/admin/sections/{}/groups/{}/rename",
        client::encode_path(section),
        client::encode_path(group)
    )
}

pub fn admin_page(chrome: &Chrome, sections: &[SectionView], moderator: bool) -> String {
    let mut out = "<h2>Administration</h2>\n".to_owned();
    if !moderator {
        out.push_str("<p class=\"empty\">Only a moderator administers the board.</p>\n");
        return out;
    }
    out.push_str(&format!(
        concat!(
            "<h3>Make a section</h3>\n",
            "<form class=\"admin\" method=\"post\" action=\"/admin/sections\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<label for=\"section-slug\">Address</label>\n",
            "<input id=\"section-slug\" name=\"slug\" type=\"text\" required>\n",
            "<label for=\"section-title\">Title</label>\n",
            "<input id=\"section-title\" name=\"title\" type=\"text\" required>\n",
            "<button type=\"submit\">Make a section</button>\n",
            "</form>\n"
        ),
        token = escape(&chrome.token),
    ));
    if sections.is_empty() {
        out.push_str("<p class=\"empty\">No sections yet.</p>\n");
        return out;
    }
    for section in sections {
        out.push_str(&format!(
            "<section class=\"admin-section\">\n<h3>{title} <code>{slug}</code> &#183; it takes {score} to post</h3>\n",
            title = escape(&section.title),
            slug = escape(&section.slug),
            score = escape(&section.score),
        ));
        out.push_str(&form(
            &section_rename_address(&section.slug),
            "section-rename",
            "Rename to",
            "title",
            "Rename",
            chrome.token.as_str(),
        ));
        out.push_str(&form(
            &section_score_address(&section.slug),
            "section-score",
            "What it takes to post",
            "score",
            "Set the score",
            chrome.token.as_str(),
        ));
        out.push_str("<h4>Groups</h4>\n");
        if section.groups.is_empty() {
            out.push_str("<p class=\"empty\">No groups yet.</p>\n");
        } else {
            for group in &section.groups {
                out.push_str(&format!(
                    "<p class=\"admin-group\"><code>{slug}</code> {name}</p>\n",
                    slug = escape(&group.slug),
                    name = escape(&group.name),
                ));
                out.push_str(&form(
                    &group_rename_address(&section.slug, &group.slug),
                    "group-rename",
                    "Rename to",
                    "name",
                    "Rename",
                    chrome.token.as_str(),
                ));
            }
        }
        out.push_str(&format!(
            concat!(
                "<h4>Make a group</h4>\n",
                "<form class=\"admin\" method=\"post\" action=\"{action}\">\n",
                "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                "<label for=\"group-name\">Group name</label>\n",
                "<input id=\"group-name\" name=\"name\" type=\"text\" required>\n",
                "<label for=\"group-slug\">Group address</label>\n",
                "<input id=\"group-slug\" name=\"slug\" type=\"text\" required>\n",
                "<button type=\"submit\">Make a group</button>\n",
                "</form>\n",
            ),
            action = escape(&section_make_group_address(&section.slug)),
            token = escape(&chrome.token),
        ));
        out.push_str("</section>\n");
    }
    out.push_str(&format!(
        concat!(
            "<h3>Maintenance</h3>\n",
            "<form class=\"admin\" method=\"post\" action=\"/admin/maintenance\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><button type=\"submit\">Run the maintenance</button></p>\n",
            "</form>\n"
        ),
        token = escape(&chrome.token),
    ));
    out
}

fn form(action: &str, id: &str, label: &str, name: &str, words: &str, token: &str) -> String {
    format!(
        concat!(
            "<form class=\"admin\" method=\"post\" action=\"{action}\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<label for=\"{id}\">{label}</label>\n",
            "<input id=\"{id}\" name=\"{name}\" type=\"text\" required>\n",
            "<button type=\"submit\">{words}</button>\n",
            "</form>\n"
        ),
        action = escape(action),
        token = escape(token),
        id = escape(id),
        label = escape(label),
        name = escape(name),
        words = escape(words),
    )
}

pub fn invitation_page(chrome: &Chrome, required: bool, invitations: &Paged<Invitation>) -> String {
    let mut out = "<h2>Invitations</h2>\n".to_owned();
    out.push_str(if required {
        "<p class=\"state\">The board requires an invitation.</p>\n"
    } else {
        "<p class=\"state\">The board asks for no invitation.</p>\n"
    });
    out.push_str(&format!(
        concat!(
            "<form class=\"admin\" method=\"post\" action=\"/admin/invitations\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<p><button type=\"submit\">Issue an invitation</button></p>\n",
            "</form>\n"
        ),
        token = escape(&chrome.token),
    ));
    if invitations.items.is_empty() {
        out.push_str("<p class=\"empty\">No invitations yet.</p>\n");
    } else {
        out.push_str("<ol class=\"invitations\">\n");
        for invitation in &invitations.items {
            let state = match invitation.spent_by.as_deref() {
                Some(who) => format!("spent by {}", escape(who)),
                None if invitation.spent => "spent".to_owned(),
                None => "unused".to_owned(),
            };
            out.push_str(&format!(
                concat!(
                    "<li class=\"invitation\">\n",
                    "<p class=\"code\"><code>{code}</code></p>\n",
                    "<p class=\"when\">expires at <time datetime=\"{stamp}\">{shown}</time> &#183; {state}</p>\n",
                    "</li>\n"
                ),
                code = escape(&invitation.code),
                stamp = escape(&invitation.expires_at),
                shown = escape(&shown_date(&invitation.expires_at)),
                state = state,
            ));
        }
        out.push_str("</ol>\n");
    }
    out.push_str(&pager(ADMIN_INVITATIONS_ADDRESS, &invitations.page));
    out
}

pub fn block_lift_address(addr: &str) -> String {
    format!("/admin/blocks/{}/lift", client::encode_path(addr))
}

pub fn block_page(chrome: &Chrome, blocks: &[AddressBlock]) -> String {
    let mut out = "<h2>Address blocks</h2>\n".to_owned();
    out.push_str(&format!(
        concat!(
            "<h3>Block an address</h3>\n",
            "<form class=\"admin\" method=\"post\" action=\"/admin/blocks\">\n",
            "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
            "<label for=\"block-addr\">Address</label>\n",
            "<input id=\"block-addr\" name=\"addr\" type=\"text\" required>\n",
            "<label for=\"block-reason\">Why</label>\n",
            "<input id=\"block-reason\" name=\"reason\" type=\"text\" required>\n",
            "<label for=\"block-days\">Days (leave empty for no end)</label>\n",
            "<input id=\"block-days\" name=\"days\" type=\"text\">\n",
            "<button type=\"submit\">Block the address</button>\n",
            "</form>\n"
        ),
        token = escape(&chrome.token),
    ));
    if blocks.is_empty() {
        out.push_str("<p class=\"empty\">No address blocks yet.</p>\n");
        return out;
    }
    out.push_str("<ol class=\"blocks\">\n");
    for block in blocks {
        let ends = match block.until.as_deref() {
            Some(stamp) => format!(
                "until <time datetime=\"{stamp}\">{shown}</time>",
                stamp = escape(stamp),
                shown = escape(&shown_date(stamp))
            ),
            None => "with no end".to_owned(),
        };
        out.push_str(&format!(
            concat!(
                "<li class=\"block\">\n",
                "<p class=\"addr\"><code>{addr}</code></p>\n",
                "<p class=\"reason\">{reason}</p>\n",
                "<p class=\"when\">blocked at <time datetime=\"{stamp}\">{shown}</time> &#183; {ends} &#183; {mode}</p>\n",
                "<form class=\"inline\" method=\"post\" action=\"{lift}\">\n",
                "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                "<button type=\"submit\">Lift</button>\n",
                "</form>\n",
                "</li>\n"
            ),
            addr = escape(&block.addr),
            reason = escape(&block.reason),
            stamp = escape(&block.blocked_at),
            shown = escape(&shown_date(&block.blocked_at)),
            ends = ends,
            mode = escape(&block.mode),
            lift = escape(&block_lift_address(&block.addr)),
            token = escape(&chrome.token),
        ));
    }
    out.push_str("</ol>\n");
    out
}

pub fn warnings_page(chrome: &Chrome, warnings: &[WarningView]) -> String {
    let open = warnings.iter().any(|warning| !warning.acknowledged);
    let mut out = "<h2>Warnings</h2>\n".to_owned();
    if warnings.is_empty() {
        out.push_str("<p class=\"empty\">No warnings have been given.</p>\n");
    } else {
        out.push_str("<ol class=\"warnings\">\n");
        for warning in warnings {
            out.push_str(&format!(
                concat!(
                    "<li class=\"warning\">\n",
                    "<p class=\"reason\">{reason}</p>\n",
                    "<p class=\"when\">{when} &#183; {state}</p>\n",
                    "</li>\n"
                ),
                reason = escape(&warning.reason),
                when = escape(&warning.created_at),
                state = if warning.acknowledged {
                    "acknowledged"
                } else {
                    "open"
                },
            ));
        }
        out.push_str("</ol>\n");
    }
    if open {
        out.push_str(&format!(
            concat!(
                "<form class=\"acknowledge\" method=\"post\" action=\"/me/warnings/acknowledge\">\n",
                "<input type=\"hidden\" name=\"token\" value=\"{token}\">\n",
                "<p><button type=\"submit\">Acknowledge</button></p>\n",
                "</form>\n"
            ),
            token = escape(&chrome.token),
        ));
    }
    out
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
            ignored: None,
            ban: None,
            moderator: false,
            remark: None,
        }
    }

    #[test]
    fn a_profile_page_tells_whether_one_ignores_the_account() {
        let page = profile_page(
            &chrome(Theme::Light),
            &profile("alice", None),
            &ProfileView {
                own: false,
                has_avatar: false,
                ignored: Some(true),
                ban: None,
                moderator: false,
                remark: None,
            },
            None,
        );
        assert!(page.contains("You ignore this account."), "{page}");
    }

    #[test]
    fn a_profile_page_tells_a_moderator_about_a_ban() {
        let page = profile_page(
            &chrome(Theme::Light),
            &profile("alice", None),
            &ProfileView {
                own: false,
                has_avatar: false,
                ignored: Some(false),
                ban: Some(BanState {
                    banned: true,
                    reason: Some("spam".to_owned()),
                    until: None,
                }),
                moderator: true,
                remark: None,
            },
            None,
        );
        assert!(page.contains("This account is banned for spam."), "{page}");
    }

    #[test]
    fn a_warnings_page_carries_every_warning_and_the_way_to_acknowledge_them() {
        let warnings = vec![
            WarningView {
                reason: "spam".to_owned(),
                created_at: "2024-12-17T23:00:00Z".to_owned(),
                acknowledged: false,
            },
            WarningView {
                reason: "<script>".to_owned(),
                created_at: "2024-12-17T23:10:00Z".to_owned(),
                acknowledged: true,
            },
        ];
        let page = warnings_page(&chrome(Theme::Light), &warnings);
        assert!(page.contains("spam"), "{page}");
        assert!(page.contains("acknowledged"), "{page}");
        assert!(page.contains("open"), "{page}");
        assert!(page.contains("&lt;script&gt;"), "{page}");
        assert!(!page.contains("<script"), "{page}");
        assert!(
            page.contains("action=\"/me/warnings/acknowledge\""),
            "{page}"
        );
    }

    #[test]
    fn a_warnings_page_with_nothing_to_acknowledge_offers_no_way() {
        let warnings = vec![WarningView {
            reason: "spam".to_owned(),
            created_at: "2024-12-17T23:00:00Z".to_owned(),
            acknowledged: true,
        }];
        let page = warnings_page(&chrome(Theme::Light), &warnings);
        assert!(!page.contains("me/warnings/acknowledge"), "{page}");
        let empty = warnings_page(&chrome(Theme::Light), &[]);
        assert!(empty.contains("No warnings"), "{empty}");
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
                ignored: None,
                ban: None,
                moderator: false,
                remark: None,
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

    fn held() -> SubjectView {
        SubjectView {
            holding: true,
            ..SubjectView::default()
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
            &held(),
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
            &held(),
        );
        assert!(html.contains("<h3>1 remark</h3>"));
    }

    #[test]
    fn a_subject_with_no_remarks_says_so() {
        let html = subject_page(&subject("First", "body"), &remarks(vec![], 0), &held());
        assert!(html.contains("No remarks here yet."));
        assert!(html.contains("<h3>0 remarks</h3>"));
    }

    #[test]
    fn what_a_subject_and_a_remark_are_carries_as_a_mark() {
        let mut pinned = subject("First", "body");
        pinned.sticky = true;
        pinned.edited = true;
        let html = subject_page(&pinned, &remarks(vec![], 0), &held());
        assert!(html.contains("(pinned, edited)"));
        let mut edited = remark("1", "bob", "x");
        edited.edited = true;
        assert!(
            subject_page(
                &subject("First", "body"),
                &remarks(vec![edited], 1),
                &held()
            )
            .contains("(edited)")
        );
    }

    #[test]
    fn a_removed_remark_says_why_and_a_kept_one_says_so() {
        let mut removed = remark("1", "bob", "gone");
        removed.deleted = true;
        removed.deleted_reason = Some("off topic".to_owned());
        let mut kept = remark("2", "carol", "hidden");
        kept.ignored = true;
        let html = subject_page(
            &subject("First", "body"),
            &remarks(vec![removed, kept], 2),
            &held(),
        );
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
            &held(),
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
            &held(),
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
            &held(),
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
        let html = subject_page(
            &subject("First", "body"),
            &remarks(vec![removed], 1),
            &held(),
        );
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
                    &held(),
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
        let html = comment_list(&[remark("1", "bob", "First thought"), answer], &held());
        assert!(html.contains("class=\"answer\""), "no quote: {html}");
        assert!(
            html.contains("<a href=\"#remark-1\">bob wrote</a>: First thought"),
            "the writer and the words are missing: {html}"
        );
    }

    #[test]
    fn a_remark_that_answers_nothing_carries_no_quote() {
        let html = comment_list(&[remark("1", "bob", "First thought")], &held());
        assert!(!html.contains("class=\"answer\""), "a quote: {html}");
    }

    #[test]
    fn a_reply_to_a_remark_off_the_page_carries_no_quote() {
        let mut answer = remark("2", "carol", "I agree");
        answer.parent_id = Some("9".to_owned());
        let html = comment_list(&[remark("1", "bob", "First thought"), answer], &held());
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
        let html = comment_list(&[gone, answer], &held());
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
        let html = comment_list(&[remark("1", "bob", &long), answer], &held());
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
        let html = comment_list(&[remark("1", "bob", "ask @alice")], &held());
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
        let html = comment_list(&[remark("1", "bob", "A reply"), answer, deeper], &held());
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
        let html = comment_list(&[answer, remark("1", "bob", "A reply")], &held());
        assert!(html.find("id=\"remark-1\"").unwrap() < html.find("id=\"remark-2\"").unwrap());
    }

    #[test]
    fn a_reply_whose_remark_is_not_on_this_page_is_still_shown() {
        let mut orphan = remark("2", "carol", "An answer");
        orphan.parent_id = Some("gone".to_owned());
        let html = comment_list(&[orphan], &held());
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
                &held(),
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
    fn one_page(total: u64) -> PageInfo {
        PageInfo {
            number: 1,
            size: 25,
            total,
            total_pages: 1,
            has_next: false,
            has_previous: false,
        }
    }

    fn report(id: &str, remark: Option<&str>) -> ReportView {
        ReportView {
            id: id.to_owned(),
            topic_id: "11111111-1111-1111-1111-111111111111".to_owned(),
            comment_id: remark.map(|remark| remark.to_owned()),
            reporter: "bob".to_owned(),
            kind: "rule".to_owned(),
            reason: "spam".to_owned(),
            created_at: "2024-12-18T01:00:00Z".to_owned(),
        }
    }

    #[test]
    fn a_subject_carries_the_way_to_report_it() {
        let page = subject_page(
            &subject("First", "body"),
            &remarks(vec![remark("1", "bob", "A reply")], 1),
            &held(),
        );
        assert!(
            page.contains("action=\"/topics/11111111-1111-1111-1111-111111111111/report\""),
            "{page}"
        );
        assert!(page.contains("name=\"kind\""), "{page}");
        assert!(page.contains("name=\"reason\""), "{page}");
    }

    #[test]
    fn a_remark_carries_the_way_to_report_it() {
        let page = comment_list(&[remark("1", "bob", "A reply")], &held());
        assert!(
            page.contains(
                "action=\"/topics/11111111-1111-1111-1111-111111111111/comments/1/report\""
            ),
            "{page}"
        );
    }

    #[test]
    fn one_with_no_account_is_offered_no_way_to_report() {
        let page = subject_page(
            &subject("First", "body"),
            &remarks(vec![remark("1", "bob", "A reply")], 1),
            &SubjectView::default(),
        );
        assert!(!page.contains("/report"), "{page}");
    }

    #[test]
    fn the_open_reports_are_on_a_page_of_their_own() {
        let page = reports_page(&chrome(Theme::Light), &[report("1", None)], &one_page(1));
        assert!(page.contains("<h2>Open reports</h2>"), "{page}");
        assert!(page.contains(">bob<"), "{page}");
        assert!(page.contains("rule"), "{page}");
        assert!(page.contains("spam"), "{page}");
        assert!(
            page.contains("href=\"/topics/11111111-1111-1111-1111-111111111111\""),
            "{page}"
        );
        assert!(page.contains("action=\"/reports/1/close\""), "{page}");
    }

    #[test]
    fn a_report_of_a_remark_points_at_the_remark() {
        let page = reports_page(
            &chrome(Theme::Light),
            &[report("1", Some("7"))],
            &one_page(1),
        );
        assert!(
            page.contains("href=\"/topics/11111111-1111-1111-1111-111111111111#remark-7\""),
            "{page}"
        );
    }

    #[test]
    fn a_board_with_no_open_reports_says_so() {
        let page = reports_page(&chrome(Theme::Light), &[], &one_page(0));
        assert!(page.contains("No open reports"), "{page}");
    }

    #[test]
    fn a_reports_page_carries_no_scripting() {
        let page = page(
            &chrome(Theme::Light),
            "Open reports",
            &reports_page(
                &chrome(Theme::Light),
                &[ReportView {
                    id: "1".to_owned(),
                    topic_id: "2".to_owned(),
                    comment_id: None,
                    reporter: "<script>".to_owned(),
                    kind: "rule".to_owned(),
                    reason: "<img src=1 onerror=go()>".to_owned(),
                    created_at: "2024-12-18T01:00:00Z".to_owned(),
                }],
                &one_page(1),
            ),
        );
        assert!(scripting_free(&page), "not scripting free: {page}");
    }
}
