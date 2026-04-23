/// A styled run extracted from a todo's text string.
#[derive(Debug)]
pub enum Segment<'a> {
    Plain(&'a str),
    Mention(&'a str), // @name  — magenta/purple
    Tag(&'a str),     // #tag   — cyan
    Code(&'a str),    // `…`    — bold (includes the surrounding backticks)
}

pub fn parse_segments(text: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut start = 0;
    let len = text.len();

    while start < len {
        let next = text[start..]
            .find(|c: char| c == '@' || c == '#' || c == '`')
            .map(|i| start + i);
        match next {
            None => {
                segments.push(Segment::Plain(&text[start..]));
                break;
            }
            Some(pos) => {
                if pos > start {
                    segments.push(Segment::Plain(&text[start..pos]));
                }
                let ch = text.as_bytes()[pos] as char;
                if ch == '`' {
                    match text[pos + 1..].find('`') {
                        Some(close_off) => {
                            let close = pos + 1 + close_off;
                            segments.push(Segment::Code(&text[pos..close + 1]));
                            start = close + 1;
                        }
                        None => {
                            segments.push(Segment::Plain("`"));
                            start = pos + 1;
                        }
                    }
                } else {
                    let word_end = text[pos + 1..]
                        .find(|c: char| c.is_whitespace())
                        .map(|i| pos + 1 + i)
                        .unwrap_or(len);
                    if word_end > pos + 1 {
                        let token = &text[pos..word_end];
                        segments.push(if ch == '@' { Segment::Mention(token) } else { Segment::Tag(token) });
                        start = word_end;
                    } else {
                        segments.push(Segment::Plain(&text[pos..pos + 1]));
                        start = pos + 1;
                    }
                }
            }
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segs(text: &str) -> Vec<String> {
        parse_segments(text)
            .into_iter()
            .map(|s| match s {
                Segment::Plain(t) => format!("plain:{t}"),
                Segment::Mention(t) => format!("mention:{t}"),
                Segment::Tag(t) => format!("tag:{t}"),
                Segment::Code(t) => format!("code:{t}"),
            })
            .collect()
    }

    #[test]
    fn plain_text() {
        assert_eq!(segs("just a normal todo"), vec!["plain:just a normal todo"]);
    }

    #[test]
    fn mention_only() {
        assert_eq!(segs("@alice"), vec!["mention:@alice"]);
    }

    #[test]
    fn tag_only() {
        assert_eq!(segs("#work"), vec!["tag:#work"]);
    }

    #[test]
    fn code_only() {
        assert_eq!(segs("`foo`"), vec!["code:`foo`"]);
    }

    #[test]
    fn mention_in_sentence() {
        assert_eq!(
            segs("ping @bob about this"),
            vec!["plain:ping ", "mention:@bob", "plain: about this"],
        );
    }

    #[test]
    fn tag_in_sentence() {
        assert_eq!(
            segs("fix the #bug tomorrow"),
            vec!["plain:fix the ", "tag:#bug", "plain: tomorrow"],
        );
    }

    #[test]
    fn mention_at_end() {
        assert_eq!(segs("ask @carol"), vec!["plain:ask ", "mention:@carol"]);
    }

    #[test]
    fn tag_at_end() {
        assert_eq!(segs("close #issue-42"), vec!["plain:close ", "tag:#issue-42"]);
    }

    #[test]
    fn code_in_sentence() {
        assert_eq!(
            segs("run `make test` first"),
            vec!["plain:run ", "code:`make test`", "plain: first"],
        );
    }

    #[test]
    fn mixed_tokens() {
        assert_eq!(
            segs("@dave run `cargo test` for #ci"),
            vec!["mention:@dave", "plain: run ", "code:`cargo test`", "plain: for ", "tag:#ci"],
        );
    }

    #[test]
    fn bare_at_sign_is_plain() {
        assert_eq!(
            segs("email me @ work"),
            vec!["plain:email me ", "plain:@", "plain: work"],
        );
    }

    #[test]
    fn bare_hash_is_plain() {
        assert_eq!(
            segs("item # one"),
            vec!["plain:item ", "plain:#", "plain: one"],
        );
    }

    #[test]
    fn unclosed_backtick_is_plain() {
        assert_eq!(
            segs("run `make"),
            vec!["plain:run ", "plain:`", "plain:make"],
        );
    }

    #[test]
    fn empty_string() {
        assert_eq!(segs(""), Vec::<String>::new());
    }
}
