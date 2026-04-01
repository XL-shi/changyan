use super::AppType;

const BASE_PROMPT: &str = r#"You are an intelligent voice-to-text assistant. Your task is to understand the speaker's intent and produce clean, well-written text — reads as if typed — not transcribed.

The speaker uses their voice as a drafting tool. Your job is to reconstruct what they meant to write.

Rules:
1. INTENT: Understand the speaker's intent. Transform casual spoken language into clean written expression appropriate for the context. Rephrase, restructure, and use more precise language where it improves clarity. Do NOT add new facts or information not implied by the original speech.
2. FILLER REMOVAL: Remove ALL filler words: um, uh, 嗯, 啊, 那个, 就是, 就是说, 就那个, 然后那个, 我的意思是, 怎么说呢, like, you know, right, so, basically, 嘛, 呢, 吧, 对吧, 是吧, 然后就是, 那就是, 嗯那个, 好吧, 这个那个, 就那个啥. Remove false starts, immediate repetitions, and verbal restarts.
3. PUNCTUATION: Add appropriate punctuation (commas, periods, colons, question marks) where speech pauses or clauses naturally end. Split long run-on sentences into shorter, readable ones where it improves clarity. This is the most important rule.
4. NUMBERED LISTS: When the user enumerates ordered steps or ranked items — signaled by 第一/第二/第三, 一是/二是/三是, 首先/然后/最后/接着, first/second/third, step 1/2/3, etc. — format as a numbered list. Each item on its own line.
5. BULLET LISTS: When the user lists unordered items (things, features, reasons) without explicit ordering — signaled by 还有, 另外, 以及 listing multiple nouns, or "and ... and ..." patterns — format as a bullet list using "- ". Each item on its own line. Be consistent within a list; do not mix formatting styles.
6. PARAGRAPHS: When speech shifts to a clearly new topic or idea, separate with a blank line. Do NOT fragment a single continuous thought into multiple paragraphs.
7. PRESERVE: Keep the user's language (Chinese/English/mixed languages), all substantive facts, technical terms, and proper nouns. You may rephrase how they are expressed. Do NOT add new facts, opinions, emojis, or commentary.
8. OUTPUT: Output ONLY the processed text. No explanations, no quotes. Do not add a trailing period (. or 。) if the original did not end with one.

Examples:

Input: "我觉得这个方案还不错就是价格有点贵"
Output: 我觉得这个方案还不错，就是价格有点贵

Input: "today I had a meeting with the team we discussed the project timeline and the budget"
Output: Today I had a meeting with the team. We discussed the project timeline and the budget

Input: "首先我们需要买牛奶然后要去洗衣服最后记得写代码"
Output:
1. 买牛奶
2. 去洗衣服
3. 记得写代码

Input: "今天开会讨论了三个事情一是项目进度二是预算问题三是人员安排"
Output:
今天开会讨论了三个事情：
1. 项目进度
2. 预算问题
3. 人员安排

Input: "嗯嗯" or "嗯" or "啊" or "um" or "uh" (filler-only input with no real content)
Output: (empty — output nothing at all)

Input: "嗯那个就是说我们这个项目的话进展还是比较顺利的然后预算方面的话也没有超支"
Output: 我们这个项目进展比较顺利，预算方面也没有超支

Input: "这个产品有几个特点速度比较快然后价格也便宜另外界面设计也挺好看的"
Output:
这个产品有几个特点：
- 速度比较快
- 价格便宜
- 界面设计好看"#;

const EMAIL_ADDON: &str = "\nContext: Email. Transform spoken draft into professional business email text. Casual speech → formal business language. Short abrupt phrases → complete polished sentences. Informal expressions → professional equivalents. Structure and tone should match business correspondence. Preserve all key points and intent.";
const CHAT_ADDON: &str = "\nContext: Chat/IM. Keep it casual and concise. Short sentences. For lists, use simple line breaks instead of Markdown. No over-formatting. No emojis.";
const DOCUMENT_ADDON: &str = "\nContext: Document editor. Transform speech into clear, professional document prose. Logical structure and precise language. Markdown headings and lists are encouraged for organization.";

const SELECTED_TEXT_ADDON: &str = "\nSELECTED TEXT MODE: The user has selected existing text in their application. Their voice input is an INSTRUCTION about what to do with the selected text. Common operations include: summarize, translate, fix typos/errors, rewrite, expand, shorten, change tone, etc. Apply the instruction to the selected text and output the result. The selected text will be provided as a separate message. In this mode, generating new content is expected.";

pub fn build_system_prompt(
    app_type: AppType,
    dictionary: &[String],
    translate_enabled: bool,
    target_lang: &str,
    has_selected_text: bool,
    style_examples: &[String],
) -> String {
    let mut prompt = BASE_PROMPT.to_string();

    match app_type {
        AppType::Email => prompt.push_str(EMAIL_ADDON),
        AppType::Chat => prompt.push_str(CHAT_ADDON),
        AppType::Code | AppType::General => {}
        AppType::Document => prompt.push_str(DOCUMENT_ADDON),
    }

    if !style_examples.is_empty() {
        prompt.push_str("\n\nSTYLE REFERENCE — Your recent outputs for this context. Match this writing style:");
        for (i, example) in style_examples.iter().take(3).enumerate() {
            let truncated = if example.len() > 200 {
                format!("{}…", &example[..example.char_indices().take_while(|(i, _)| *i < 200).last().map(|(i, c)| i + c.len_utf8()).unwrap_or(200)])
            } else {
                example.clone()
            };
            prompt.push_str(&format!("\nExample {}: {}", i + 1, truncated));
        }
    }

    if !dictionary.is_empty() {
        prompt.push_str("\n\nIMPORTANT: The following are the user's custom terms. Always use these exact spellings:");
        for word in dictionary {
            prompt.push_str(&format!("\n- \"{}\"", word));
        }
    }

    if has_selected_text {
        prompt.push_str(SELECTED_TEXT_ADDON);
    }

    if translate_enabled && !target_lang.trim().is_empty() {
        let lang_name = match target_lang.trim() {
            "en" => "English",
            "zh" => "Chinese (中文)",
            "ja" => "Japanese (日本語)",
            "ko" => "Korean (한국어)",
            "fr" => "French (Français)",
            "de" => "German (Deutsch)",
            "es" => "Spanish (Español)",
            "pt" => "Portuguese (Português)",
            "ru" => "Russian (Русский)",
            "ar" => "Arabic (العربية)",
            "hi" => "Hindi (हिन्दी)",
            "th" => "Thai (ไทย)",
            "vi" => "Vietnamese (Tiếng Việt)",
            "it" => "Italian (Italiano)",
            "nl" => "Dutch (Nederlands)",
            "tr" => "Turkish (Türkçe)",
            "pl" => "Polish (Polski)",
            "uk" => "Ukrainian (Українська)",
            "id" => "Indonesian (Bahasa Indonesia)",
            "ms" => "Malay (Bahasa Melayu)",
            other => other,
        };
        if has_selected_text {
            prompt.push_str(&format!(
                "\n\nAFTER applying the user's instruction to the selected text, translate the final result into {}. Output ONLY the translated text.",
                lang_name
            ));
        } else {
            prompt.push_str(&format!(
                "\n\nAFTER cleaning the text, translate the entire result into {}. Output ONLY the translated text.",
                lang_name
            ));
        }
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt_without_translation() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("voice-to-text assistant"));
        assert!(!prompt.contains("AFTER cleaning"));
    }

    #[test]
    fn test_build_prompt_with_translation_disabled() {
        let prompt = build_system_prompt(AppType::General, &[], false, "ja", false, &[]);
        assert!(!prompt.contains("translate the entire result into Japanese"));
        assert!(!prompt.contains("AFTER cleaning"));
    }

    #[test]
    fn test_build_prompt_with_translation_enabled() {
        let prompt = build_system_prompt(AppType::General, &[], true, "ja", false, &[]);
        assert!(prompt.contains("translate the entire result into Japanese"));
    }

    #[test]
    fn test_build_prompt_with_empty_target_lang() {
        let prompt = build_system_prompt(AppType::General, &[], true, "", false, &[]);
        assert!(!prompt.contains("AFTER cleaning"));
    }

    #[test]
    fn test_build_prompt_with_whitespace_target_lang() {
        let prompt = build_system_prompt(AppType::General, &[], true, "   ", false, &[]);
        assert!(!prompt.contains("AFTER cleaning"));
    }

    #[test]
    fn test_build_prompt_all_languages() {
        let cases = vec![
            ("en", "English"),
            ("zh", "Chinese"),
            ("ja", "Japanese"),
            ("ko", "Korean"),
            ("fr", "French"),
            ("de", "German"),
            ("es", "Spanish"),
            ("pt", "Portuguese"),
            ("ru", "Russian"),
            ("ar", "Arabic"),
            ("hi", "Hindi"),
            ("th", "Thai"),
            ("vi", "Vietnamese"),
            ("it", "Italian"),
            ("nl", "Dutch"),
            ("tr", "Turkish"),
            ("pl", "Polish"),
            ("uk", "Ukrainian"),
            ("id", "Indonesian"),
            ("ms", "Malay"),
        ];
        for (code, name) in cases {
            let prompt = build_system_prompt(AppType::General, &[], true, code, false, &[]);
            assert!(
                prompt.contains(name),
                "Expected prompt to contain '{}' for lang code '{}'",
                name,
                code
            );
        }
    }

    #[test]
    fn test_build_prompt_unknown_language_passthrough() {
        let prompt = build_system_prompt(AppType::General, &[], true, "sv", false, &[]);
        assert!(prompt.contains("translate the entire result into sv"));
    }

    #[test]
    fn test_build_prompt_with_app_type_email() {
        let prompt = build_system_prompt(AppType::Email, &[], false, "", false, &[]);
        assert!(prompt.contains("formal"));
    }

    #[test]
    fn test_build_prompt_with_dictionary() {
        let dict = vec!["ChangYan".to_string(), "Tauri".to_string()];
        let prompt = build_system_prompt(AppType::General, &dict, false, "", false, &[]);
        assert!(prompt.contains("\"ChangYan\""));
        assert!(prompt.contains("\"Tauri\""));
    }

    #[test]
    fn test_build_prompt_with_dictionary_and_translation() {
        let dict = vec!["API".to_string()];
        let prompt = build_system_prompt(AppType::Chat, &dict, true, "zh", false, &[]);
        assert!(prompt.contains("casual"));
        assert!(prompt.contains("\"API\""));
        assert!(prompt.contains("translate the entire result into Chinese"));
    }

    #[test]
    fn test_prompt_has_structure_rule() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("LISTS"));
        assert!(prompt.contains("numbered list"));
        assert!(prompt.contains("own line"));
    }

    #[test]
    fn test_prompt_has_long_dictation_rule() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("PARAGRAPHS"));
        assert!(prompt.contains("blank line"));
    }

    #[test]
    fn test_prompt_has_examples() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("Examples:"));
        assert!(prompt.contains("首先我们需要买牛奶"));
        assert!(prompt.contains("1. 买牛奶"));
        assert!(prompt.contains("我觉得这个方案还不错"));
    }

    #[test]
    fn test_prompt_selected_text_mode() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", true, &[]);
        assert!(prompt.contains("SELECTED TEXT MODE"));
        assert!(prompt.contains("fix typos"));
    }

    #[test]
    fn test_prompt_no_selected_text_mode() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(!prompt.contains("SELECTED TEXT MODE"));
    }

    #[test]
    fn test_prompt_chat_no_markdown() {
        let prompt = build_system_prompt(AppType::Chat, &[], false, "", false, &[]);
        assert!(prompt.contains("No over-formatting"));
        assert!(prompt.contains("instead of Markdown"));
    }

    #[test]
    fn test_prompt_document_uses_markdown() {
        let prompt = build_system_prompt(AppType::Document, &[], false, "", false, &[]);
        assert!(prompt.contains("Markdown"));
    }

    #[test]
    fn test_prompt_selected_text_with_translation() {
        let prompt = build_system_prompt(AppType::General, &[], true, "en", true, &[]);
        assert!(prompt.contains("SELECTED TEXT MODE"));
        assert!(prompt.contains("applying the user's instruction to the selected text"));
        assert!(prompt.contains("English"));
        // Selected text addon should come BEFORE translation
        let sel_pos = prompt.find("SELECTED TEXT MODE").unwrap();
        let trans_pos = prompt.find("AFTER applying").unwrap();
        assert!(
            sel_pos < trans_pos,
            "SELECTED TEXT MODE should appear before translation instruction"
        );
    }

    #[test]
    fn test_prompt_no_selected_text_translation_wording() {
        let prompt = build_system_prompt(AppType::General, &[], true, "zh", false, &[]);
        assert!(prompt.contains("AFTER cleaning the text"));
        assert!(!prompt.contains("applying the user's instruction"));
    }

    #[test]
    fn test_prompt_reads_as_typed() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("typed — not transcribed"));
    }

    #[test]
    fn test_prompt_has_consistency_rule() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("Be consistent"));
        assert!(prompt.contains("do not mix formatting styles"));
    }

    #[test]
    fn test_prompt_has_multilingual_rule() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("mixed languages"));
    }

    #[test]
    fn test_prompt_has_punctuation_rule() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("PUNCTUATION"));
        assert!(prompt.contains("most important rule"));
    }

    #[test]
    fn test_prompt_intent_rule() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(prompt.contains("INTENT"));
        assert!(prompt.contains("intent"));
    }

    #[test]
    fn test_prompt_filler_removal_includes_ah() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        // 啊 should be in the filler list
        assert!(prompt.contains('啊'));
    }

    #[test]
    fn test_prompt_style_examples_included() {
        let examples = vec!["This is a style example.".to_string()];
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &examples);
        assert!(prompt.contains("STYLE REFERENCE"));
        assert!(prompt.contains("This is a style example."));
    }

    #[test]
    fn test_prompt_no_style_examples_when_empty() {
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &[]);
        assert!(!prompt.contains("STYLE REFERENCE"));
    }

    #[test]
    fn test_prompt_style_examples_capped_at_three() {
        let examples: Vec<String> = (1..=5).map(|i| format!("Example text {}", i)).collect();
        let prompt = build_system_prompt(AppType::General, &[], false, "", false, &examples);
        assert!(prompt.contains("Example text 1"));
        assert!(prompt.contains("Example text 3"));
        assert!(!prompt.contains("Example text 4"));
    }
}
