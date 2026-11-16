use iced::keyboard::{self, key};

pub(crate) fn key_press(named: key::Named) -> iced::Event {
    let pressed = keyboard::Key::Named(named);
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: pressed.clone(),
        modified_key: pressed,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::empty(),
        text: None,
        repeat: false,
    })
}
