use crate::Username;

pub fn mentioned_in(body: &str) -> Vec<Username> {
    let mut found: Vec<Username> = Vec::new();
    let bytes: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != '@' {
            i += 1;
            continue;
        }
        if i > 0 && !is_boundary(bytes[i - 1]) {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len() && is_name_char(bytes[end]) {
            end += 1;
        }
        let candidate: String = bytes[start..end].iter().collect();
        if let Ok(name) = Username::parse(&candidate)
            && !found.contains(&name)
        {
            found.push(name);
        }
        i = end.max(start);
    }
    found
}

fn is_boundary(c: char) -> bool {
    !c.is_alphanumeric() && c != '_' && c != '@'
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(body: &str) -> Vec<String> {
        mentioned_in(body)
            .into_iter()
            .map(|n| n.as_str().to_owned())
            .collect()
    }

    #[test]
    fn finds_a_name_on_its_own() {
        assert_eq!(names("@alice_01"), vec!["alice_01"]);
    }

    #[test]
    fn finds_a_name_in_a_sentence() {
        assert_eq!(
            names("I think @alice_01 said so already."),
            vec!["alice_01"]
        );
    }

    #[test]
    fn finds_several_and_keeps_their_order() {
        assert_eq!(
            names("@alice_01 and @bob_02 both replied"),
            vec!["alice_01", "bob_02"]
        );
    }

    #[test]
    fn names_the_same_person_once() {
        assert_eq!(names("@alice_01 @alice_01 @alice_01"), vec!["alice_01"]);
    }

    #[test]
    fn ignores_an_address_that_only_looks_like_a_name() {
        assert!(names("write to alice@example.com").is_empty());
    }

    #[test]
    fn ignores_a_name_nobody_could_have() {
        assert!(names("@ and @! and @-").is_empty());
        assert!(names("@ab").is_empty());
    }

    #[test]
    fn a_bare_at_finds_nothing() {
        assert!(names("@").is_empty());
        assert!(names("").is_empty());
    }

    #[test]
    fn stops_at_punctuation() {
        assert_eq!(names("thanks, @alice_01!"), vec!["alice_01"]);
        assert_eq!(names("(@alice_01)"), vec!["alice_01"]);
    }

    #[test]
    fn a_name_at_the_start_of_a_line_is_found() {
        assert_eq!(names("first line\n@alice_01 second"), vec!["alice_01"]);
    }

    #[test]
    fn does_not_read_a_name_out_of_the_middle_of_a_word() {
        assert!(names("something@alice_01").is_empty());
    }
}
