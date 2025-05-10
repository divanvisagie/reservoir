use crate::clients::openai::types::Message;

fn message_to_string(msg: &Message) -> String {
    match msg.role.as_str() {
        "user" => format!("User: {}", msg.content),
        "assistant" => format!("Assistant: {}", msg.content),
        "system" => format!("System Note: {}", msg.content),
        _ => format!("{}: {}", msg.role, msg.content),
    }
}

pub fn compress_system_context(messages: &Vec<Message>) -> Vec<Message> {
    let first_index = messages.iter().position(|m| m.role == "system");
    let last_index = messages.iter().rposition(|m| m.role == "system");

    if let (Some(first), Some(last)) = (first_index, last_index) {
        if first != 0 || first == last {
            return messages.clone(); // return original if invalid or nothing to compress
        }

        let mut compressed = vec![messages[0].clone()];

        for i in first + 1..=last {
            let msg = &messages[i];
            let line = format!("\n{}", message_to_string(msg));
            compressed[0].content += &line;
        }

        // Add the remaining messages (after the last system prompt)
        compressed.extend_from_slice(&messages[last + 1..]);

        compressed
    } else {
        messages.clone()
    }
}
