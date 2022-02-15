use crate::Operation;
use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Clone, Debug)]
pub struct KeyBind {
    pub operation: Operation,
    pub modifiers: KeyModifiers,
    pub key: KeyCode,
}

pub fn get_modifier_symbol(modifier: KeyModifiers) -> String {
    let mut res = String::new();

    if modifier.contains(KeyModifiers::ALT) {
        res.push('⎇');
    }
    if modifier.contains(KeyModifiers::CONTROL) {
        res.push('^');
    }
    if modifier.contains(KeyModifiers::SHIFT) {
        res.push('⇧');
    }
    res
}

pub fn get_key_symbol(key: KeyCode) -> String {
    match key {
        KeyCode::Backspace => "⌫".to_string(),
        KeyCode::Enter => "⏎".to_string(),
        KeyCode::Left => "←".to_string(),
        KeyCode::Right => "→".to_string(),
        KeyCode::Up => "↑".to_string(),
        KeyCode::Down => "↓".to_string(),
        KeyCode::Home => "⇱".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PgUp".to_string(),
        KeyCode::PageDown => "PgDn".to_string(),
        KeyCode::Tab => "⇥".to_string(),
        KeyCode::BackTab => "⇤".to_string(),
        KeyCode::Delete => "⌦".to_string(),
        KeyCode::Insert => "Ins".to_string(),
        KeyCode::F(_) => "".to_string(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Null => "".to_string(),
        KeyCode::Esc => "Esc".to_string(),
    }
}

pub fn get_help(label: &str, operation: Operation, key_binds: &Vec<KeyBind>) -> String {
    let key_bind = key_binds
        .iter()
        .find(|key_bind| key_bind.operation == operation);

    if let Some(key_bind) = key_bind {
        return format!(
            "{:} {:}{:}",
            label,
            get_modifier_symbol(key_bind.modifiers),
            get_key_symbol(key_bind.key)
        );
    }
    label.to_string()
}
