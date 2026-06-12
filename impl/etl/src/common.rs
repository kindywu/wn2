use dict_models::enums::{DataSource, PosType};

/// POS normalization: stardict "n./v./adj./adv." → wn "n/v/a/r/s"
pub const POS_MAP: &[(&str, Option<PosType>)] = &[
    ("n", Some(PosType::N)), ("n.", Some(PosType::N)),
    ("v", Some(PosType::V)), ("v.", Some(PosType::V)),
    ("vt", Some(PosType::V)), ("vt.", Some(PosType::V)),
    ("vi", Some(PosType::V)), ("vi.", Some(PosType::V)),
    ("a", Some(PosType::A)), ("adj", Some(PosType::A)), ("adj.", Some(PosType::A)),
    ("adv", Some(PosType::R)), ("adv.", Some(PosType::R)),
    ("s", Some(PosType::S)), ("s.", Some(PosType::S)),
    ("r", Some(PosType::R)), ("r.", Some(PosType::R)),
    ("prep", None), ("conj", None), ("pron", None),
];

/// stardict exchange key → form_type mapping
pub const EXCHANGE_MAP: &[(&str, &str)] = &[
    ("pl", "plural"),
    ("p", "past"),
    ("d", "past"),
    ("past", "past"),
    ("pp", "past_participle"),
    ("ing", "present_participle"),
    ("3rd", "third_person"),
    ("comp", "comparative"),
    ("super", "superlative"),
];

pub fn normalize_pos(raw: &str) -> Option<PosType> {
    let key = raw.trim().to_lowercase();
    let first = key.split(|c: char| c.is_whitespace() || c == '；' || c == ';').next().unwrap_or(&key);
    for (pat, pos) in POS_MAP {
        if first == *pat {
            return pos.clone();
        }
    }
    None
}

/// Field priority: higher number = higher priority
pub fn field_priority_for_source(field: &str, source: DataSource) -> i32 {
    match (field, source) {
        // wn > stardict for these
        ("pos", DataSource::Wn) => 2,
        ("pos", DataSource::Stardict) => 1,
        ("phonetic", DataSource::Wn) => 2,
        ("phonetic", DataSource::Stardict) => 1,
        // stardict-only fields
        ("collins", DataSource::Stardict) => 1,
        ("oxford", DataSource::Stardict) => 1,
        ("bnc", DataSource::Stardict) => 1,
        ("frq", DataSource::Stardict) => 1,
        ("tags", DataSource::Stardict) => 1,
        ("exchange", DataSource::Stardict) => 2,
        ("exchange", DataSource::Wn) => 1,
        ("detail", DataSource::Stardict) => 1,
        ("audio", DataSource::Stardict) => 1,
        _ => 0,
    }
}

/// Convert stardict exchange text field to JSON
pub fn parse_exchange(raw: &str) -> Option<serde_json::Value> {
    if raw.is_empty() {
        return None;
    }
    let mut map = serde_json::Map::new();
    for part in raw.split('/') {
        let part = part.trim();
        if let Some(idx) = part.find(':') {
            let key = part[..idx].trim().to_lowercase();
            let val = part[idx+1..].trim().to_string();
            if !key.is_empty() && !val.is_empty() {
                map.insert(key, serde_json::Value::String(val));
            }
        }
    }
    if map.is_empty() { None } else { Some(serde_json::Value::Object(map)) }
}

/// Parse stardict detail JSON (may be wrapped in extra text)
pub fn parse_detail(raw: &str) -> Option<serde_json::Value> {
    if raw.is_empty() {
        return None;
    }
    serde_json::from_str(raw).ok()
}

/// Map stardict tag string to array of tags
pub fn parse_tags(raw: &str) -> Vec<String> {
    if raw.is_empty() {
        return vec![];
    }
    raw.split_whitespace()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty() && ["CET4","CET6","TOEFL","IELTS","GRE","KY","K12","SAT","TEM4","TEM8"].contains(&s.as_str()))
        .collect()
}

/// Determine if oxford is true (0 or '0' or empty = false)
pub fn parse_oxford(val: i32) -> bool {
    val != 0
}
