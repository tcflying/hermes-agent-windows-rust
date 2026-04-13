use crate::chat::Message;

pub struct ContextCompressor {
    pub threshold: f32,
    pub target_ratio: f32,
    pub protect_head: usize,
    pub protect_tail: usize,
}

impl ContextCompressor {
    pub fn new() -> Self {
        Self {
            threshold: 0.6,
            target_ratio: 0.3,
            protect_head: 3,
            protect_tail: 20,
        }
    }

    pub fn should_compress(&self, messages: &[Message], context_limit: usize) -> bool {
        let total_tokens: usize = messages.iter().map(|m| message_tokens(m)).sum();
        total_tokens > (context_limit as f32 * self.threshold) as usize
    }

    pub fn compress(&self, messages: &[Message]) -> Vec<Message> {
        if messages.len() <= self.protect_head + self.protect_tail {
            return messages.to_vec();
        }

        let total_tokens: usize = messages.iter().map(|m| message_tokens(m)).sum();
        let target_tokens = (total_tokens as f32 * self.target_ratio) as usize;

        let mut result = Vec::new();

        for msg in messages.iter().take(self.protect_head) {
            result.push(msg.clone());
        }

        let middle_start = self.protect_head;
        let middle_end = messages.len().saturating_sub(self.protect_tail);

        if middle_end > middle_start {
            let middle_messages = &messages[middle_start..middle_end];
            let middle_tokens: usize = middle_messages.iter().map(|m| message_tokens(m)).sum();
            let middle_target = (target_tokens as f32 * 0.5) as usize;

            let skip_ratio = if middle_tokens > middle_target * 3 {
                3
            } else {
                2
            };

            for (i, msg) in middle_messages.iter().enumerate() {
                if i % skip_ratio == 0 {
                    let mut compressed = msg.clone();
                    if msg.role == "tool" {
                        if let Some(ref content) = msg.content {
                            if content.len() > 500 {
                                compressed.content =
                                    Some(format!("{}... [compressed]", &content[..500]));
                            }
                        }
                    }
                    result.push(compressed);
                }
            }
        }

        for msg in messages
            .iter()
            .skip(messages.len().saturating_sub(self.protect_tail))
        {
            result.push(msg.clone());
        }

        result
    }
}

impl Default for ContextCompressor {
    fn default() -> Self {
        Self::new()
    }
}

fn message_tokens(msg: &Message) -> usize {
    let mut total = 0;

    if let Some(ref content) = msg.content {
        total += estimate_tokens(content);
    }

    if let Some(ref calls) = msg.tool_calls {
        for call in calls {
            total += estimate_tokens(&call.function.name);
            total += estimate_tokens(&call.function.arguments);
        }
    }

    total
}

fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    let chinese_chars = text.chars().filter(|c| c.len_utf8() > 1).count();
    (words * 4 + chinese_chars / 2) / 4
}
