use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\[(?P<tag>[a-zA-Z0-9_\u3040-\u309F\u30A0-\u30FF\u4E00-\u9FFF]+)\]|【(?P<ztag>[^】]+)】")
        .expect("Failed to compile TAG_REGEX")
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
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_default();

            if !raw_tag.is_empty() {
                detected_tags.push(raw_tag);
            }
        }

        // Apply rules based on detected tags
        for tag in &detected_tags {
            match tag.as_str() {
                "laughter" | "laugh" | "giggle" | "happy" | "joy" | "cheer" | "smile" | "笑" | "笑い" => {
                    emotion_sliders.insert("happy".to_string(), 0.7);
                    emotion_cfg_scale = 1.5;
                }
                "whisper" | "whispering" | "gentle" | "sweet" | "soft" | "囁き" | "ささやき" => {
                    emotion_sliders.insert("happy".to_string(), 0.25);
                    emotion_cfg_scale = 1.3;
                    speed_factor = 0.88;
                }
                "sigh" | "sad" | "sorrow" | "crying" | "ため息" | "悲しい" | "哀" => {
                    emotion_sliders.insert("sad".to_string(), 0.7);
                    emotion_cfg_scale = 1.5;
                    speed_factor = 0.90;
                }
                "angry" | "anger" | "mad" | "shout" | "irritated" | "怒り" | "怒" => {
                    emotion_sliders.insert("angry".to_string(), 0.75);
                    emotion_cfg_scale = 1.6;
                    speed_factor = 1.05;
                }
                "surprise" | "surprised" | "shock" | "驚き" | "驚" => {
                    emotion_sliders.insert("surprised".to_string(), 0.7);
                    emotion_cfg_scale = 1.5;
                }
                "fear" | "scared" | "恐れ" | "怖" => {
                    emotion_sliders.insert("fear".to_string(), 0.7);
                    emotion_cfg_scale = 1.5;
                }
                "disgust" | "嫌悪" => {
                    emotion_sliders.insert("disgust".to_string(), 0.7);
                    emotion_cfg_scale = 1.5;
                }
                _ => {
                    // Unknown tag, treat as neutral
                }
            }
        }

        // Clean tags from text
        let cleaned = TAG_REGEX.replace_all(input, "").to_string();
        let cleaned_trimmed = cleaned.trim().to_string();

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
        assert_eq!(parsed.emotion_cfg_scale, 1.3);
        assert_eq!(parsed.emotion_sliders.get("happy"), Some(&0.25));
    }

    #[test]
    fn test_parse_laughter() {
        let input = "[laughter] えへへ、パパ大好き！";
        let parsed = EmotionParser::parse(input);
        assert_eq!(parsed.cleaned_text, "えへへ、パパ大好き！");
        assert_eq!(parsed.detected_tags, vec!["laughter"]);
        assert_eq!(parsed.emotion_cfg_scale, 1.5);
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
}
