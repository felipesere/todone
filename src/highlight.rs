/// A styled run extracted from a todo's text string.
#[derive(Debug)]
pub enum Segment<'a> {
    Plain(&'a str),
    Mention(&'a str), // @name  — magenta/purple
    Tag(&'a str),     // #tag   — cyan
    Code(&'a str),    // `…`    — bold (includes the surrounding backticks)
    Link { text: &'a str, url: &'a str }, // [text](url)
}

/// The parsed pieces of a `[text](url)` markdown link.
#[derive(Debug, PartialEq, Eq)]
pub struct LinkData<'a> {
    pub text: &'a str,
    pub url: &'a str,
    /// Byte offset (relative to the full input `text`) just past the closing `)`.
    pub end: usize,
}

/// Tries to parse a markdown link `[text](url)` starting at `pos`, where
/// `text.as_bytes()[pos] == b'['`. Returns `None` if the brackets/parens
/// don't close out into a well-formed link.
fn try_parse_link(text: &str, pos: usize) -> Option<LinkData<'_>> {
    let close = pos + text[pos..].find(']')?;
    let link_text = &text[pos + 1..close];
    if !text[close + 1..].starts_with('(') {
        return None;
    }
    let paren = close + 1 + text[close + 1..].find(')')?;
    let url = &text[close + 2..paren];
    Some(LinkData { text: link_text, url, end: paren + 1 })
}

pub fn parse_segments(text: &str) -> Vec<Segment<'_>> {
    let mut segments = Vec::new();
    let mut start = 0;
    let len = text.len();

    while start < len {
        let next = text[start..]
            .find(|c: char| c == '@' || c == '#' || c == '`' || c == '[')
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
                if ch == '[' {
                    match try_parse_link(text, pos) {
                        Some(LinkData { text: link_text, url, end }) => {
                            segments.push(Segment::Link { text: link_text, url });
                            start = end;
                        }
                        None => {
                            segments.push(Segment::Plain("["));
                            start = pos + 1;
                        }
                    }
                } else if ch == '`' {
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
                Segment::Link { text, url } => format!("link:{text}|{url}"),
            })
            .collect()
    }

    #[test]
    fn link_only() {
        assert_eq!(segs("[click me](https://example.com)"), vec!["link:click me|https://example.com"]);
    }

    #[test]
    fn link_in_sentence() {
        assert_eq!(
            segs("see [the docs](https://example.com) for more"),
            vec!["plain:see ", "link:the docs|https://example.com", "plain: for more"],
        );
    }

    #[test]
    fn unclosed_bracket_is_plain() {
        assert_eq!(segs("todo [oops"), vec!["plain:todo ", "plain:[", "plain:oops"]);
    }

    #[test]
    fn bracket_without_parens_is_plain() {
        assert_eq!(segs("[not a link] ok"), vec!["plain:[", "plain:not a link] ok"]);
    }

    #[test]
    fn try_parse_link_ok() {
        let text = "[foo](bar)";
        assert_eq!(try_parse_link(text, 0), Some(LinkData { text: "foo", url: "bar", end: 10 }));
    }

    #[test]
    fn try_parse_link_missing_paren() {
        let text = "[foo] bar";
        assert_eq!(try_parse_link(text, 0), None);
    }

    #[test]
    fn try_parse_link_unclosed_paren() {
        let text = "[foo](bar";
        assert_eq!(try_parse_link(text, 0), None);
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
