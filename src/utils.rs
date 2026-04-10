use std::borrow::Cow;

pub fn normalize_verdict(verdict: &str) -> Cow<'_, str> {
    let trimmed = verdict.trim();
    if trimmed.eq_ignore_ascii_case("Unaccepted") {
        Cow::Borrowed("WA")
    } else if trimmed.len() != verdict.len() {
        Cow::Owned(trimmed.to_string())
    } else {
        Cow::Borrowed(verdict)
    }
}

pub fn verdict_equals(left: &str, right: &str) -> bool {
    normalize_verdict(left).eq_ignore_ascii_case(normalize_verdict(right).as_ref())
}

pub fn is_ac_verdict(verdict: &str) -> bool {
    verdict_equals(verdict, "AC")
}
