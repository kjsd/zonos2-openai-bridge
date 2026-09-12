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
    /// Maps a raw tag string to an official Irodori-TTS emoji (all 36 official styles),
    /// or None if the tag should be safely removed without vocalization.
    pub fn tag_to_emoji(tag: &str) -> Option<&'static str> {
        let normalized = tag.trim().to_lowercase();
        match normalized.as_str() {
            // === 1. Whispering & Soft Ear-level Voice (👂) ===
            "whisper" | "whispering" | "whispers" | "shush" | "shh" | "quiet"
            | "ささやき" | "囁き" | "耳元" | "小声" | "静かに" => Some("👂"),

            // === 2. Gentle & Tender (🫶) ===
            "gentle" | "sweet" | "soft" | "softly" | "優しく" | "慈しむ" => Some("🫶"),

            // === 3. Sigh & Breathing & Sleep breath (😮‍💨) ===
            "sigh" | "sighs" | "ため息" | "息" | "寝息" => Some("😮‍💨"),

            // === 4. Shortness of breath & Panting (🌬️) ===
            "pant" | "pants" | "息切れ" | "激しい呼吸" => Some("🌬️"),

            // === 5. Chuckling & Suppressed Giggling (🤭) ===
            "chuckle" | "chuckles" | "giggle" | "giggles" | "くすくす" | "忍び笑い" | "吹き出し" => Some("🤭"),

            // === 6. Happy & Joy & Bright Smile (😊) ===
            "happy" | "joy" | "smile" | "喜" | "歓喜" | "明るく" | "嬉しそう" => Some("😊"),

            // === 7. Excited & Exuberant Laughter (😆) ===
            "laugh" | "laughs" | "laughter" | "笑い" | "大笑い" | "笑"
            | "excited" | "わくわく" | "興奮" | "楽しそう" => Some("😆"),

            // === 8. Angry & Screaming & Yelling (😡) ===
            "angry" | "anger" | "mad" | "shout" | "shouts" | "yell" | "yells" | "screaming"
            | "furious" | "irritated" | "怒り" | "怒" | "叫び" | "不満" | "拗ね" => Some("😡"),

            // === 9. Sad & Crying & Weeping (😭) ===
            "sad" | "sorrow" | "grief" | "crying" | "cry" | "cries" | "weep" | "weeping"
            | "悲しい" | "哀" | "泣き" | "泣" | "すすり泣き" => Some("😭"),

            // === 10. Trembling Voice & Timid (🥺) ===
            "trembling" | "timid" | "おどおど" | "震え声" | "頼りない" => Some("🥺"),

            // === 11. Fear & Panicked & Flustered (😰) ===
            "fear" | "fearful" | "scared" | "terrified" | "frightened" | "panicked"
            | "恐れ" | "怖" | "恐怖" | "慌て" | "焦り" | "取り乱す" => Some("😰"),

            // === 12. Surprise & Shock & Gasp (😲) ===
            "surprise" | "surprised" | "shock" | "shocked" | "驚き" | "驚" | "gasp" | "gasps"
            | "ハッ" | "息をのむ" | "感嘆" => Some("😲"),

            // === 13. Distressed & In Pain (😖) ===
            "distress" | "pain" | "苦しい" | "苦しそう" => Some("😖"),

            // === 14. Worried & Uneasy (😟) ===
            "worried" | "uneasy" | "心配" | "不安" => Some("😟"),

            // === 15. Annoyed & Exasperated (🙄) ===
            "annoyed" | "exasperated" | "呆れ" | "やれやれ" => Some("🙄"),

            // === 16. Tongue Clicking (😒) ===
            "click tongue" | "tongue click" | "舌打ち" => Some("😒"),

            // === 17. Sarcastic & Teasing & Smirk (😏) ===
            "sarcastic" | "sarcasm" | "ironic" | "皮肉" | "teasing" | "からかい" | "甘え" | "宥め" | "ニヤリ" => Some("😏"),

            // === 18. Shy & Bashful (🫣) ===
            "shy" | "bashful" | "恥ずかしい" | "照れ" => Some("🫣"),

            // === 19. Relieved & Satisfied (😌) ===
            "relieved" | "satisfied" | "安堵" | "ほっとした" | "満足" => Some("😌"),

            // === 20. Sleepy & Sluggish (😴) ===
            "sleepy" | "drowsy" | "眠い" | "気だるげ" => Some("😴"),

            // === 21. Yawn (🥱) ===
            "yawn" | "yawns" | "あくび" => Some("🥱"),

            // === 22. Plead & Beg (🙏) ===
            "plead" | "beg" | "お願い" | "頼み" => Some("🙏"),

            // === 23. Drunk (🥴) ===
            "drunk" | "酔っ払い" | "泥酔" => Some("🥴"),

            // === 24. Wondering & Questioning (🤔) ===
            "wondering" | "questioning" | "疑問" | "思案" => Some("🤔"),

            // === 25. Agreeing / Backchanneling (👌) ===
            "agree" | "nod" | "相槌" | "肯定" => Some("👌"),

            // === 26. Groaning & Moaning & Heavy Breathing (🥵) ===
            "groan" | "groans" | "moan" | "moans" | "うめき" | "呻き" | "喘ぎ" | "息遣い" => Some("🥵"),

            // === 27. Coughing & Throat-clearing & Sneezing & Sniffling (🤧) ===
            "clear throat" | "clears throat" | "cough" | "coughs" | "throat-clearing"
            | "sniff" | "sniffs" | "sneeze" | "sneezes"
            | "咳" | "咳払い" | "くしゃみ" | "鼻をすする" => Some("🤧"),

            // === 28. Wet sounds & Licking & Chewing (👅) ===
            "lick" | "licking" | "chew" | "wet sound" | "舌舐めずり" | "咀嚼音" | "水音" => Some("👅"),

            // === 29. Lip smack & Lip noise (💋) ===
            "lip smack" | "lip noise" | "リップノイズ" | "チュッ" => Some("💋"),

            // === 30. Gulping & Swallowing (🥤) ===
            "gulp" | "swallow" | "飲み込む" | "ゴクリ" => Some("🥤"),

            // === 31. Loudspeaker & Reverb / Echo (📢) ===
            "dramatic" | "dramatic tone" | "loudspeaker" | "megaphone" | "echo" | "reverb"
            | "拡声器" | "エコー" | "リバーブ" => Some("📢"),

            // === 32. Over Phone / Speaker (📞) ===
            "phone" | "telephone" | "電話" | "スピーカー越し" => Some("📞"),

            // === 33. Muffled Voice (🤐) ===
            "muffled" | "こもった声" => Some("🤐"),

            // === 34. Humming (🎵) ===
            "humming" | "hum" | "鼻歌" => Some("🎵"),

            // === 35. Pause & Silence (⏸️) ===
            "pause" | "間" | "沈黙" => Some("⏸️"),

            // === 36. Speed: Slowly (🐢) & Fast (⏩) ===
            "slowly" | "ゆっくり" => Some("🐢"),
            "fast" | "早口" | "急ぎ" => Some("⏩"),

            // Unknown tags: safely remove so they are never spoken aloud
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
    fn test_irodori_mapper_throat_clearing_to_sneeze_emoji() {
        let input = "[clear throat] でも、馬車の中なら変な輩に邪魔される心配もないわね。";
        let mapped = IrodoriMapper::convert(input);
        assert_eq!(mapped.detected_tags, vec!["clear throat"]);
        assert!(mapped.prompt_text.contains("🤧"));
        assert!(!mapped.prompt_text.contains("clear throat"));
        assert!(mapped.prompt_text.contains("でも、馬車の中なら変な輩に邪魔される心配もないわね。"));
    }

    #[test]
    fn test_irodori_mapper_unknown_tag_stripping() {
        let input = "[unknown_tag_123] こんにちは、ケンジ君。";
        let mapped = IrodoriMapper::convert(input);
        assert_eq!(mapped.detected_tags, vec!["unknown_tag_123"]);
        assert!(!mapped.prompt_text.contains("unknown_tag_123"));
        assert_eq!(mapped.prompt_text, "こんにちは、ケンジ君。");
    }

    #[test]
    fn test_irodori_mapper_parentheses_and_asterisks() {
        let input = "(sigh) イリアさん、どうかその魔法を収めて！ *giggle*";
        let mapped = IrodoriMapper::convert(input);
        assert_eq!(mapped.detected_tags, vec!["sigh", "giggle"]);
        assert!(mapped.prompt_text.contains("😮‍💨"));
        assert!(mapped.prompt_text.contains("🤭"));
    }

    #[test]
    fn test_irodori_official_styles_coverage() {
        // Test key official styles: breathing, panting, gentle, excited, pain, throat clearing, etc.
        assert_eq!(IrodoriMapper::tag_to_emoji("pant"), Some("🌬️"));
        assert_eq!(IrodoriMapper::tag_to_emoji("gentle"), Some("🫶"));
        assert_eq!(IrodoriMapper::tag_to_emoji("excited"), Some("😆"));
        assert_eq!(IrodoriMapper::tag_to_emoji("groan"), Some("🥵"));
        assert_eq!(IrodoriMapper::tag_to_emoji("fear"), Some("😰"));
        assert_eq!(IrodoriMapper::tag_to_emoji("clear throat"), Some("🤧"));
        assert_eq!(IrodoriMapper::tag_to_emoji("dramatic"), Some("📢"));
    }
}
