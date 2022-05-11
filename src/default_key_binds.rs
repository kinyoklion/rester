use crate::key_bind::KeyBind;
use crate::Operation;
use crossterm::event::{KeyCode, KeyModifiers};

pub fn default_key_binds() -> Vec<KeyBind> {
    vec![
        KeyBind {
            operation: Operation::GotoUrl,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('u'),
        },
        KeyBind {
            operation: Operation::GotoRequestBody,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('b'),
        },
        KeyBind {
            operation: Operation::GotoRequestHeaders,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('h'),
        },
        KeyBind {
            operation: Operation::GotoResponseBody,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('j'),
        },
        KeyBind {
            operation: Operation::GotoResponseHeaders,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('n'),
        },
        KeyBind {
            operation: Operation::LoadRequest,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('r'),
        },
        KeyBind {
            operation: Operation::SaveRequest,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('s'),
        },
        KeyBind {
            operation: Operation::SaveResponse,
            modifiers: KeyModifiers::ALT,
            key: KeyCode::Char('s'),
        },
        KeyBind {
            operation: Operation::NextMethod,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('p'),
        },
        KeyBind {
            operation: Operation::GotoRequestView,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('a'),
        },
        KeyBind {
            operation: Operation::GotoResponseView,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('q'),
        },
        KeyBind {
            operation: Operation::SendRequest,
            modifiers: KeyModifiers::ALT,
            key: KeyCode::Enter,
        },
        KeyBind {
            operation: Operation::Quit,
            modifiers: KeyModifiers::CONTROL,
            key: KeyCode::Char('w'),
        },
    ]
}
