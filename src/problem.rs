use regex::Regex;

pub fn extract_problem_id(file_name: &str) -> Option<String> {
    let regex = Regex::new(r"(?i)^([A-Z]{1,3}\d{3,6}[A-Z0-9]*)\.[^.]+$").ok()?;
    let captures = regex.captures(file_name)?;
    Some(captures.get(1)?.as_str().to_uppercase())
}
