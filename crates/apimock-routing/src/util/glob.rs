#[cfg(test)]
mod tests;

pub fn glob_match(pattern: &str, text: &str) -> bool {
    glob_match_impl(
        &pattern.chars().collect::<Vec<_>>(),
        &text.chars().collect::<Vec<_>>(),
    )
}

fn glob_match_impl(pat: &[char], text: &[char]) -> bool {
    match (pat.first(), text.first()) {
        (Some('*'), _) => {
            // `*` matches 0 or more chars
            glob_match_impl(&pat[1..], text)
                || (!text.is_empty() && glob_match_impl(pat, &text[1..]))
        }
        (Some('?'), Some(_)) => {
            // `?` matches any single char
            glob_match_impl(&pat[1..], &text[1..])
        }
        (Some(p), Some(t)) if p == t => {
            // char match
            glob_match_impl(&pat[1..], &text[1..])
        }
        (None, None) => true,
        _ => false,
    }
}
