use regex::Regex;
use std::sync::LazyLock;

/// Regex matching full bracketed or starred tags:
/// [tag], 【tag】, (tag), （tag）, *tag*
static FULL_TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[(?P<tag>[^\]]+)\]|【(?P<ztag>[^】]+)】|\((?P<ptag>[^)]+)\)|（(?P<fptag>[^）]+)）|\*(?P<atag>[^*]+)\*")
        .expect("Failed to compile FULL_TAG_REGEX in irodori mapper")
});

/// Missing opening bracket: e.g. "chuckle] Hello", " clear throat] text"
static MISSING_OPEN_BRACKET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?P<prefix>^|[\s.,!?。、！？])(?P<otag>[a-z][a-z0-9_\- ]{1,25})\]")
        .expect("Failed to compile MISSING_OPEN_BRACKET_REGEX in irodori mapper")
});

/// Missing closing bracket: e.g. "[whisper Hello", " [sigh text"
static MISSING_CLOSE_BRACKET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\[(?P<itag>[a-z][a-z0-9_\- ]{1,25})(?P<suffix>[\s.,!?。、！？]|$)")
        .expect("Failed to compile MISSING_CLOSE_BRACKET_REGEX in irodori mapper")
});

/// Cleans up orphan bracket or asterisk symbols from text boundaries
static ORPHAN_BRACKETS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\[\]【】()（）*]+|[\[\]【】()（）*]+$")
        .expect("Failed to compile ORPHAN_BRACKETS_REGEX in irodori mapper")
});

static MULTI_SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[ \t]{2,}")
        .expect("Failed to compile MULTI_SPACE_REGEX in irodori mapper")
});

#[derive(Debug, Clone, PartialEq)]
pub struct IrodoriMappedText {
    /// Text with tags replaced by Irodori-compatible emojis
    pub prompt_text: String,
    /// Detected tags in original order
    pub detected_tags: Vec<String>,
}

pub struct IrodoriMapper;

impl IrodoriMapper {
    /// Maps a raw tag string to an Irodori emoji, or None if it should be removed.
    pub fn tag_to_emoji(tag: &str) -> Option<&'static str> {
        let normalized = tag.trim().to_lowercase();
        match normalized.as_str() {
            // Whispering / Gentle / Soft
            "whisper" | "whispering" | "whispers" | "gentle" | "sweet" | "soft" | "softly"
            | "shush" | "shh" | "quiet" | "ささやき" | "囁き" | "静かに" => Some("👂"),

            // Sigh / Breathing / Panting
            "sigh" | "sighs" | "ため息" | "息" | "pant" | "pants" | "息切れ" => Some("😮‍💨"),

            // Laughter / Chuckling / Giggling
            "chuckle" | "chuckles" | "giggle" | "giggles" | "laugh" | "laughs" | "laughter"
            | "くすくす" | "笑い" | "笑" => Some("🤭"),

            // Happy / Joy / Cheerful
            "happy" | "joy" | "cheer" | "smile" | "喜" | "歓喜" => Some("😊"),

            // Angry / Screaming / Yelling
            "angry" | "anger" | "mad" | "shout" | "shouts" | "yell" | "yells" | "screaming"
            | "furious" | "irritated" | "怒り" | "怒" | "叫び" => Some("😡"),

            // Sad / Crying / Weeping
            "sad" | "sorrow" | "grief" | "crying" | "cry" | "cries" | "weep" | "weeping"
            | "悲しい" | "哀" | "泣き" | "泣" => Some("😭"),

            // Fear / Terrified
            "fear" | "fearful" | "scared" | "terrified" | "frightened" | "恐れ" | "怖"
            | "恐怖" => Some("😱"),

            // Surprise / Shock / Gasp
            "surprise" | "surprised" | "shock" | "shocked" | "驚き" | "驚" | "gasp" | "gasps"
            | "ハッ" | "息をのむ" => Some("😲"),

            // Sniffing / Cold voice
            "sniff" | "sniffs" | "鼻をすする" => Some("🤧"),

            // Groan / Moan
            "groan" | "groans" | "うめき" | "呻き" => Some("😩"),

            // Yawn
            "yawn" | "yawns" | "あくび" => Some("🥱"),

            // Pauses / Silence
            "pause" | "間" => Some("⏸️"),

            // Speed: Slowly
            "slowly" | "ゆっくり" => Some("🐢"),

            // Speed: Fast
            "fast" | "早口" => Some("⏩"),

            // Excited
            "excited" | "わくわく" | "興奮" => Some("✨"),

            // Sarcastic / Teasing / Smirk
            "sarcastic" | "sarcasm" | "ironic" | "皮肉" => Some("😏"),

            // Dramatic tone
            "dramatic" | "dramatic tone" => Some("🎭"),

            // Sound tags without direct emoji: remove so they are not spoken aloud
            "clear throat" | "clears throat" | "cough" | "coughs" | "throat-clearing" => None,

            // Unknown tags: default to removing
            _ => None,
        }
    }

    /// Converts raw text containing audio/emotion tags into Irodori emoji-embedded text.
    pub fn convert(input: &str) -> IrodoriMappedText {
        let mut detected_tags = Vec::new();

        // 1. Full enclosed tags: [tag], (tag), *tag*, etc.
        let text_after_full = FULL_TAG_REGEX.replace_all(input, |caps: &regex::Captures| {
            let raw_tag = caps
                .name("tag")
                .or_else(|| caps.name("ztag"))
                .or_else(|| caps.name("ptag"))
                .or_else(|| caps.name("fptag"))
                .or_else(|| caps.name("atag"))
                .map(|m| m.as_str().trim().to_lowercase())
                .unwrap_or_default();

            if !raw_tag.is_empty() {
                detected_tags.push(raw_tag.clone());
            }

            match Self::tag_to_emoji(&raw_tag) {
                Some(emoji) => format!(" {emoji} "),
                None => " ".to_string(),
            }
        });

        // 2. Rescue tags missing opening bracket (e.g. "chuckle] Hello")
        let text_after_missing_open = MISSING_OPEN_BRACKET_REGEX.replace_all(&text_after_full, |caps: &regex::Captures| {
            let prefix = caps.name("prefix").map(|m| m.as_str()).unwrap_or("");
            let raw_tag = caps.name("otag").map(|m| m.as_str().trim().to_lowercase()).unwrap_or_default();

            if !raw_tag.is_empty() {
                detected_tags.push(raw_tag.clone());
            }

            match Self::tag_to_emoji(&raw_tag) {
                Some(emoji) => format!("{prefix} {emoji} "),
                None => format!("{prefix} "),
            }
        });

        // 3. Rescue tags missing closing bracket (e.g. "[whisper Hello")
        let text_after_missing_close = MISSING_CLOSE_BRACKET_REGEX.replace_all(&text_after_missing_open, |caps: &regex::Captures| {
            let suffix = caps.name("suffix").map(|m| m.as_str()).unwrap_or("");
            let raw_tag = caps.name("itag").map(|m| m.as_str().trim().to_lowercase()).unwrap_or_default();

            if !raw_tag.is_empty() {
                detected_tags.push(raw_tag.clone());
            }

            match Self::tag_to_emoji(&raw_tag) {
                Some(emoji) => format!(" {emoji} {suffix}"),
                None => format!(" {suffix}"),
            }
        });

        // 4. Sanitize whitespace and boundary orphan brackets
        let collapsed = MULTI_SPACE_REGEX.replace_all(&text_after_missing_close, " ").to_string();
        let trimmed = collapsed.trim();
        let sanitized = ORPHAN_BRACKETS_REGEX.replace_all(trimmed, "").to_string();
        let prompt_text = sanitized.trim().to_string();

        IrodoriMappedText {
            prompt_text,
            detected_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irodori_mapper_tags() {
        let input = "[whisper] パパ、寒くない？ [sigh]";
        let mapped = IrodoriMapper::convert(input);
        assert_eq!(mapped.detected_tags, vec!["whisper", "sigh"]);
        assert!(mapped.prompt_text.contains("👂"));
        assert!(mapped.prompt_text.contains("😮‍💨"));
        assert!(mapped.prompt_text.contains("パパ、寒くない？"));
    }

    #[test]
    fn test_irodori_mapper_missing_brackets() {
        let input = "chuckle] さあケンジ君、やっとウィンターホールドに着いたわ。";
        let mapped = IrodoriMapper::convert(input);
        assert_eq!(mapped.detected_tags, vec!["chuckle"]);
        assert!(mapped.prompt_text.contains("🤭"));
        assert!(mapped.prompt_text.contains("さあケンジ君、やっとウィンターホールドに着いたわ。"));
        assert!(!mapped.prompt_text.contains("chuckle]"));
    }

    #[test]
    fn test_irodori_mapper_sound_tag_stripping() {
        let input = "[clear throat] でも、馬車の中なら変な輩に邪魔される心配もないわね。";
        let mapped = IrodoriMapper::convert(input);
        assert_eq!(mapped.detected_tags, vec!["clear throat"]);
        assert!(!mapped.prompt_text.contains("clear throat"));
        assert_eq!(mapped.prompt_text, "でも、馬車の中なら変な輩に邪魔される心配もないわね。");
    }

    #[test]
    fn test_irodori_mapper_parentheses_and_asterisks() {
        let input = "(sigh) イリアさん、どうかその魔法を収めて！ *giggle*";
        let mapped = IrodoriMapper::convert(input);
        assert_eq!(mapped.detected_tags, vec!["sigh", "giggle"]);
        assert!(mapped.prompt_text.contains("😮‍💨"));
        assert!(mapped.prompt_text.contains("🤭"));
    }
}
