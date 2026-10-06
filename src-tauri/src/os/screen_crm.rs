//! Heuristiques CRM à partir d’un texte OCR (Gmail / clients mail).

use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCrmDraft {
    pub name: String,
    pub organisme: String,
    pub phone: String,
    pub email: String,
}

const SKIP_EMAIL_LOCAL: &[&str] = &[
    "noreply",
    "no-reply",
    "no_reply",
    "mailer-daemon",
    "mailerdaemon",
    "notifications",
    "newsletter",
    "support",
    "postmaster",
];

const CONSUMER_DOMAINS: &[&str] = &[
    "gmail.com",
    "googlemail.com",
    "outlook.com",
    "hotmail.com",
    "live.com",
    "yahoo.com",
    "yahoo.fr",
    "wanadoo.fr",
    "orange.fr",
    "free.fr",
    "laposte.net",
    "icloud.com",
    "me.com",
    "proton.me",
    "protonmail.com",
];

const SKIP_NAMES: &[&str] = &[
    "gmail",
    "google",
    "outlook",
    "microsoft",
    "linkedin",
    "facebook",
    "youtube",
    "inbox",
    "primary",
    "promotions",
    "social",
    "mind",
    "gestion",
    "mail",
];

pub fn parse_crm_from_ocr(text: &str) -> ScreenCrmDraft {
    let emails = extract_emails(text);
    let phone = extract_phone(text).unwrap_or_default();
    let email = pick_email(&emails).unwrap_or_default();
    let name = extract_name(text, &email).unwrap_or_default();
    let organisme = extract_organisme(text, &email).unwrap_or_default();
    ScreenCrmDraft {
        name,
        organisme,
        phone,
        email,
    }
}

fn extract_emails(text: &str) -> Vec<String> {
    let re = email_like();
    let mut out = Vec::new();
    for cap in re.captures_iter(text) {
        let raw = cap.get(0).map(|m| m.as_str()).unwrap_or("");
        let email = raw.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '_' && c != '%' && c != '+' && c != '-' && c != '@').to_lowercase();
        if !email.contains('@') || out.iter().any(|e| e == &email) {
            continue;
        }
        let Some((local, domain)) = email.split_once('@') else {
            continue;
        };
        if SKIP_EMAIL_LOCAL.iter().any(|s| local.contains(s)) {
            continue;
        }
        if domain.contains("google.com") || domain.ends_with(".googlemail.com") {
            continue;
        }
        out.push(email);
    }
    out
}

fn email_like() -> regex::Regex {
    regex::Regex::new(r"(?i)[A-Z0-9._%+\-]+@[A-Z0-9.\-]+\.[A-Z]{2,}")
        .expect("email regex")
}

fn pick_email(emails: &[String]) -> Option<String> {
    emails.first().cloned()
}

fn extract_phone(text: &str) -> Option<String> {
    let intl = regex::Regex::new(
        r"\+(?:33)[\s.\-]?[1-9](?:[\s.\-]?\d{2}){4}",
    )
    .ok()?;
    if let Some(m) = intl.find(text) {
        return Some(normalize_phone(m.as_str()));
    }
    let fr = regex::Regex::new(r"(?:^|[^\d])(0[1-9](?:[\s.\-]?\d{2}){4})(?:[^\d]|$)").ok()?;
    fr.captures(text)
        .and_then(|c| c.get(1))
        .map(|m| normalize_phone(m.as_str()))
}

fn normalize_phone(raw: &str) -> String {
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
    if let Some(rest) = digits.strip_prefix("+33") {
        let mut s = String::from("0");
        s.push_str(rest);
        return group_fr(&s);
    }
    group_fr(&digits)
}

fn group_fr(digits: &str) -> String {
    if digits.len() == 10 && digits.starts_with('0') {
        let b = digits.as_bytes();
        return format!(
            "{}{} {}{} {}{} {}{} {}{}",
            b[0] as char,
            b[1] as char,
            b[2] as char,
            b[3] as char,
            b[4] as char,
            b[5] as char,
            b[6] as char,
            b[7] as char,
            b[8] as char,
            b[9] as char
        );
    }
    digits.to_string()
}

fn extract_name(text: &str, email: &str) -> Option<String> {
    let from_re =
        regex::Regex::new(r"(?im)^\s*(?:de|from)\s*:\s*(.+)$").ok()?;
    if let Some(cap) = from_re.captures(text) {
        if let Some(cleaned) = clean_name_line(cap.get(1).map(|m| m.as_str()).unwrap_or("")) {
            return Some(cleaned);
        }
    }
    if !email.is_empty() {
        let quoted = regex::Regex::new(&format!(
            r"(?i)([A-ZÉÈÊÀÂÎÏÔÙÛÇ][^<\n@]{{1,60}})<\s*{}>",
            regex::escape(email)
        ))
        .ok()?;
        if let Some(cap) = quoted.captures(text) {
            if let Some(cleaned) = clean_name_line(cap.get(1).map(|m| m.as_str()).unwrap_or("")) {
                return Some(cleaned);
            }
        }
    }
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let start = lines.len().saturating_sub(18);
    for line in lines[start..].iter() {
        if let Some(cleaned) = clean_name_line(line) {
            if looks_like_person(&cleaned) {
                return Some(cleaned);
            }
        }
    }
    None
}

fn clean_name_line(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_string();
    if let Some(idx) = s.find('<') {
        s = s[..idx].trim().to_string();
    }
    s = s.trim_matches(|c: char| c == '"' || c == '\'' || c == '«' || c == '»').trim().to_string();
    if s.is_empty() || s.contains('@') {
        return None;
    }
    if s.chars().count() < 3 || s.chars().count() > 60 {
        return None;
    }
    let lower = s.to_lowercase();
    if SKIP_NAMES.iter().any(|n| lower == *n || lower.starts_with(&format!("{n} "))) {
        return None;
    }
    Some(s)
}

fn looks_like_person(name: &str) -> bool {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() < 2 || parts.len() > 4 {
        return false;
    }
    parts.iter().all(|p| {
        let mut chars = p.chars();
        match chars.next() {
            Some(c) => c.is_uppercase() || "ÉÈÊÀÂÎÏÔÙÛÇ".contains(c),
            None => false,
        }
    })
}

fn extract_organisme(text: &str, email: &str) -> Option<String> {
    let org_re = regex::Regex::new(
        r"(?i)\b([A-Z0-9][A-Za-z0-9&'’.\-]*(?:\s+[A-Z0-9][A-Za-z0-9&'’.\-]*){0,6}\s+(?:SARL|SAS|SA|SCI|EURL|SASU|Association|Studio|Cie|Company|Ltd|Inc)\.?)\b",
    )
    .ok()?;
    if let Some(cap) = org_re.captures(text) {
        let s = cap.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if s.len() >= 3 {
            return Some(s.to_string());
        }
    }
    domain_as_org(email)
}

fn domain_as_org(email: &str) -> Option<String> {
    let domain = email.split('@').nth(1)?.to_lowercase();
    if CONSUMER_DOMAINS.iter().any(|d| domain == *d) {
        return None;
    }
    let main = domain.split('.').next().unwrap_or("");
    if main.len() < 3 || main == "mail" || main == "email" {
        return None;
    }
    let mut chars = main.chars();
    let first = chars.next()?.to_uppercase().to_string();
    Some(format!("{first}{}", chars.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gmail_from_header() {
        let text = "Inbox\nDe : Marie Dupont <marie.dupont@atelier-lumiere.fr>\nÀ : moi\nTél : 06 12 34 56 78\nAtelier Lumière SARL";
        let d = parse_crm_from_ocr(text);
        assert_eq!(d.email, "marie.dupont@atelier-lumiere.fr");
        assert!(d.name.to_lowercase().contains("marie"));
        assert!(d.phone.contains("06"));
        assert!(d.organisme.to_lowercase().contains("lumière") || d.organisme.to_lowercase().contains("atelier"));
    }

    #[test]
    fn skips_noreply() {
        let text = "From: Notifications <noreply@google.com>\nHello";
        let d = parse_crm_from_ocr(text);
        assert!(d.email.is_empty());
    }

    #[test]
    fn consumer_email_no_org() {
        let text = "De : Jean Martin <jean.martin@gmail.com>";
        let d = parse_crm_from_ocr(text);
        assert_eq!(d.email, "jean.martin@gmail.com");
        assert!(d.organisme.is_empty());
    }

    #[test]
    fn splits_phone_and_email() {
        let text = "De : Claire Roux <claire@scene-nord.fr>\nMobile 07.11.22.33.44";
        let d = parse_crm_from_ocr(text);
        assert_eq!(d.email, "claire@scene-nord.fr");
        assert!(d.phone.starts_with("07"));
        assert!(!d.phone.contains('@'));
        assert!(!d.email.contains("07"));
    }
}
