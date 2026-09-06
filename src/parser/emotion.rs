use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

/// Regex matching any [bracketed tag] or 【bracketed tag】.
/// Matches any characters inside brackets (including spaces, hyphens, and unicode)
/// so that multi-word sound tags like [clear throat] or [dramatic tone] are never missed.
static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[(?P<tag>[^\]]+)\]|【(?P<ztag>[^】]+)】")
        .expect("Failed to compile TAG_REGEX")
});

static MULTI_SPACE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[ \t]{2,}")
        .expect("Failed to compile MULTI_SPACE_REGEX")
});

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedPrompt {
    pub cleaned_text: String,
    pub emotion_sliders: HashMap<String, f32>,
    pub emotion_cfg_scale: f32,
    pub speed_factor: f32,
    pub detected_tags: Vec<String>,
}

impl Default for ParsedPrompt {
    fn default() -> Self {
        Self {
            cleaned_text: String::new(),
            emotion_sliders: HashMap::new(),
            emotion_cfg_scale: 1.0,
            speed_factor: 1.0,
            detected_tags: Vec::new(),
        }
    }
}

pub struct EmotionParser;

impl EmotionParser {
    pub fn parse(input: &str) -> ParsedPrompt {
        let mut detected_tags = Vec::new();
        let mut emotion_sliders = HashMap::new();
        let mut emotion_cfg_scale = 1.0f32;
        let mut speed_factor = 1.0f32;

        for cap in TAG_REGEX.captures_iter(input) {
            let raw_tag = cap
                .name("tag")
                .or_else(|| cap.name("ztag"))
                .map(|m| m.as_str().trim().to_lowercase())
                .unwrap_or_default();

            if !raw_tag.is_empty() {
                detected_tags.push(raw_tag);
            }
        }

        // Apply rules based on detected tags (SkyrimNet / Chatterbox + Nina custom tags)
        for tag in &detected_tags {
            match tag.as_str() {
                // Happy / Laugh / Joy
                "laughter" | "laugh" | "laughs" | "happy" | "joy" | "cheer" | "smile"
                | "笑" | "笑い" | "喜" | "歓喜" => {
                    emotion_sliders.insert("happy".to_string(), 0.7);
                    emotion_cfg_scale = 1.2;
                }
                "chuckle" | "chuckles" | "giggle" | "giggles" | "くすくす" => {
                    emotion_sliders.insert("happy".to_string(), 0.5);
                    emotion_cfg_scale = 1.15;
                }
                "excited" | "興奮" | "わくわく" => {
                    emotion_sliders.insert("happy".to_string(), 0.8);
                    emotion_cfg_scale = 1.2;
                    speed_factor = 1.10;
                }

                // Whisper / Soft / Quiet (deliberate & slower pacing)
                "whisper" | "whispering" | "whispers" | "gentle" | "sweet" | "soft" | "softly"
                | "shush" | "shh" | "quiet" | "囁き" | "ささやき" | "静かに" => {
                    emotion_sliders.insert("happy".to_string(), 0.2);
                    emotion_cfg_scale = 1.1;
                    speed_factor = 0.88;
                }

                // Sad / Cry / Sigh / Groan / Sniff (slower & heavy pacing)
                "sigh" | "sighs" | "ため息" => {
                    emotion_sliders.insert("sad".to_string(), 0.6);
                    emotion_cfg_scale = 1.15;
                    speed_factor = 0.90;
                }
                "sad" | "sorrow" | "grief" | "crying" | "cry" | "cries" | "weep" | "weeping"
                | "悲しい" | "哀" | "泣き" | "泣" => {
                    emotion_sliders.insert("sad".to_string(), 0.7);
                    emotion_cfg_scale = 1.2;
                    speed_factor = 0.90;
                }
                "groan" | "groans" | "うめき" | "呻き" => {
                    emotion_sliders.insert("sad".to_string(), 0.5);
                    emotion_sliders.insert("angry".to_string(), 0.3);
                    emotion_cfg_scale = 1.15;
                    speed_factor = 0.90;
                }
                "sniff" | "sniffs" | "鼻をすする" => {
                    emotion_sliders.insert("sad".to_string(), 0.5);
                    emotion_cfg_scale = 1.15;
                    speed_factor = 0.92;
                }

                // Angry / Shout / Irritated (fast & intense pacing)
                "angry" | "anger" | "mad" | "shout" | "shouts" | "yell" | "yells"
                | "screaming" | "furious" | "irritated" | "怒り" | "怒" | "叫び" => {
                    emotion_sliders.insert("angry".to_string(), 0.75);
                    emotion_cfg_scale = 1.2;
                    speed_factor = 1.15;
                }

                // Surprise / Shock / Gasp
                "surprise" | "surprised" | "shock" | "shocked" | "驚き" | "驚" => {
                    emotion_sliders.insert("surprised".to_string(), 0.7);
                    emotion_cfg_scale = 1.2;
                }
                "gasp" | "gasps" | "ハッ" | "息をのむ" => {
                    emotion_sliders.insert("surprised".to_string(), 0.6);
                    emotion_sliders.insert("fear".to_string(), 0.3);
                    emotion_cfg_scale = 1.15;
                }

                // Fear / Scared
                "fear" | "fearful" | "scared" | "terrified" | "frightened" | "恐れ" | "怖" | "恐怖" => {
                    emotion_sliders.insert("fear".to_string(), 0.7);
                    emotion_cfg_scale = 1.2;
                }

                // Disgust
                "disgust" | "disgusted" | "嫌悪" => {
                    emotion_sliders.insert("disgust".to_string(), 0.7);
                    emotion_cfg_scale = 1.2;
                }

                // Chatterbox specific tags
                "sarcastic" | "sarcasm" | "ironic" | "皮肉" => {
                    emotion_sliders.insert("angry".to_string(), 0.3);
                    emotion_sliders.insert("sad".to_string(), 0.2);
                    emotion_cfg_scale = 1.15;
                    speed_factor = 0.95;
                }
                "dramatic" | "dramatic tone" => {
                    emotion_sliders.insert("surprised".to_string(), 0.3);
                    emotion_sliders.insert("sad".to_string(), 0.2);
                    emotion_cfg_scale = 1.2;
                    speed_factor = 0.90;
                }
                "advertisement" | "commercial" => {
                    emotion_sliders.insert("happy".to_string(), 0.4);
                    emotion_cfg_scale = 1.15;
                    speed_factor = 1.05;
                }
                "narration" | "narrator" => {
                    // Calm and neutral reading
                    emotion_cfg_scale = 1.0;
                    speed_factor = 1.00;
                }
                "slowly" => {
                    // Explicit slow pacing requested by prompt
                    speed_factor = 0.75;
                }

                // Pure sound/action tags (safely cleaned from text without altering emotion)
                "clear throat" | "clears throat" | "cough" | "coughs" | "pause" => {
                    // Do not alter emotions; these are stripped from text so they won't be spoken aloud
                }

                _ => {
                    // Unknown or unsupported tag, safely stripped from text
                }
            }
        }

        // Clean tags from text
        let cleaned = TAG_REGEX.replace_all(input, " ").to_string();
        // Collapse multiple spaces into single space
        let cleaned_collapsed = MULTI_SPACE_REGEX.replace_all(&cleaned, " ").to_string();
        let cleaned_trimmed = cleaned_collapsed.trim().to_string();

        ParsedPrompt {
            cleaned_text: cleaned_trimmed,
            emotion_sliders,
            emotion_cfg_scale,
            speed_factor,
            detected_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_whisper() {
        let input = "[whisper] パパ、寒くない…？";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "パパ、寒くない…？");
        assert_eq!(parsed.detected_tags, vec!["whisper"]);
        assert_eq!(parsed.speed_factor, 0.88);
        assert_eq!(parsed.emotion_cfg_scale, 1.1);
        assert_eq!(parsed.emotion_sliders.get("happy"), Some(&0.2));
    }

    #[test]
    fn test_parse_laughter() {
        let input = "[laughter] えへへ、パパ大好き！";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "えへへ、パパ大好き！");
        assert_eq!(parsed.detected_tags, vec!["laughter"]);
        assert_eq!(parsed.emotion_cfg_scale, 1.2);
        assert_eq!(parsed.emotion_sliders.get("happy"), Some(&0.7));
    }

    #[test]
    fn test_parse_japanese_bracket() {
        let input = "【笑い】パパ、これ見て！";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "パパ、これ見て！");
        assert_eq!(parsed.detected_tags, vec!["笑い"]);
        assert_eq!(parsed.emotion_sliders.get("happy"), Some(&0.7));
    }

    #[test]
    fn test_parse_plain_text() {
        let input = "普通のテキストです。";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "普通のテキストです。");
        assert!(parsed.detected_tags.is_empty());
        assert_eq!(parsed.speed_factor, 1.0);
        assert_eq!(parsed.emotion_cfg_scale, 1.0);
        assert!(parsed.emotion_sliders.is_empty());
    }

    #[test]
    fn test_parse_skyrimnet_chatterbox_tags() {
        // Multi-word tag with space, sound tags in middle of sentence
        let input = "[whispering] Keep your voice down - [shush] - they’ll hear us. [sigh]";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "Keep your voice down - - they’ll hear us.");
        assert_eq!(parsed.detected_tags, vec!["whispering", "shush", "sigh"]);

        let input2 = "[happy] [clear throat] So… we agree? [chuckle] Great.";
        let parsed2 = EmotionParser::parse(input2);
        assert_eq!(parsed2.cleaned_text, "So… we agree? Great.");
        assert_eq!(parsed2.detected_tags, vec!["happy", "clear throat", "chuckle"]);
        assert_eq!(parsed2.emotion_sliders.get("happy"), Some(&0.5)); // chuckle overwrote happy to 0.5
    }

    #[test]
    fn test_parse_sound_tags_stripping() {
        let input = "I told you not to touch that - [groan] - ever again. [cough]";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "I told you not to touch that - - ever again.");
        assert_eq!(parsed.detected_tags, vec!["groan", "cough"]);
    }

    #[test]
    fn test_parse_dramatic_tone_and_sarcastic() {
        let input = "[dramatic tone] The dragons have returned to Skyrim.";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "The dragons have returned to Skyrim.");
        assert_eq!(parsed.detected_tags, vec!["dramatic tone"]);
        assert_eq!(parsed.emotion_cfg_scale, 1.2);
        assert_eq!(parsed.speed_factor, 0.90);

        let input_sarcastic = "[sarcastic] Oh, what an amazing hero you are.";
        let parsed_sarcastic = EmotionParser::parse(input_sarcastic);
        assert_eq!(parsed_sarcastic.cleaned_text, "Oh, what an amazing hero you are.");
        assert_eq!(parsed_sarcastic.detected_tags, vec!["sarcastic"]);
    }

    #[test]
    fn test_parse_slowly_tag() {
        let input = "[slowly] Take your time, traveler.";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "Take your time, traveler.");
        assert_eq!(parsed.detected_tags, vec!["slowly"]);
        assert_eq!(parsed.speed_factor, 0.75);
    }
}
