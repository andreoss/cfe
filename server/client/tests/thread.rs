use client::Comment;

fn comment(id: &str, parent: Option<&str>, at: &str) -> Comment {
    Comment {
        id: id.to_owned(),
        topic_id: "t".to_owned(),
        parent_id: parent.map(|value| value.to_owned()),
        body: format!("body of {id}"),
        author_username: "alice".to_owned(),
        created_at: at.to_owned(),
        deleted: false,
        deleted_reason: None,
        edited: false,
        ignored: false,
    }
}

#[test]
fn a_reply_is_set_under_the_remark_it_answers_however_deep() {
    let remarks = vec![
        comment("1", None, "2024-06-07T10:00:00Z"),
        comment("3", Some("2"), "2024-06-07T12:00:00Z"),
        comment("2", Some("1"), "2024-06-07T11:00:00Z"),
        comment("4", None, "2024-06-07T09:00:00Z"),
    ];
    let branches = client::thread(&remarks);
    let order: Vec<(&str, usize)> = branches
        .iter()
        .map(|branch| (branch.comment.id.as_str(), branch.depth))
        .collect();
    assert_eq!(order, vec![("4", 0usize), ("1", 0), ("2", 1), ("3", 2)]);
}

#[test]
fn a_reply_whose_remark_is_not_here_is_kept_at_the_top() {
    let branches = client::thread(&[comment("1", Some("gone"), "2024-06-07T10:00:00Z")]);
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].depth, 0);
}

#[test]
fn no_remarks_at_all_is_no_thread() {
    assert!(client::thread(&[]).is_empty());
}
