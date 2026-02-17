use std::path::Path;
use std::sync::{Arc, Mutex};

use moonlight::element::{Alignment, Alignments, Colour, Dimension, Direction, Element, Position, Size};
use moonlight::internal_loop::{MoonlightApplication, UpdateResponse};
use tapestry::font::Font;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey, SmolStr};

#[derive(Debug)]
enum UserMessageTest {
	Test,
	KeyPressed(Key<SmolStr>),
}

fn from_keyboard_input(key_event: KeyEvent) -> Option<UserMessageTest> {
	if key_event.state == ElementState::Pressed {
		Some(UserMessageTest::KeyPressed(key_event.logical_key))
	} else {
		None
	}
}

struct UserStateTest {
	text: Arc<Mutex<String>>,
	font: Arc<Font>,
}

fn assemble(user_state: &UserStateTest) -> Element<UserMessageTest> {
	Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Grow { minimum: None, maximum: None }, Colour::red(), vec!(
		Element::new(Direction::Vertical, Size::Fit { minimum: None, maximum: None }, Size::Grow { minimum: None, maximum: None }, Colour::green(), vec!(
			Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Fixed(50), Colour::blue(), Vec::new()),
			Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Fixed(50), Colour::blue(), Vec::new()),
			Element::new(Direction::Horizontal, Size::FitText { minimum: None, maximum: None }, Size::FitText { minimum: None, maximum: None }, Colour::blue(), Vec::new()).text(tapestry::font::font_renderer::TextBox { font: Arc::clone(&user_state.font), text: Arc::clone(&user_state.text), pixels_per_em: 50.0.into(), position: (0.0, 0.0).into() })
		)).child_gaps(5),
		Element::new(Direction::Vertical, Size::Grow { minimum: None, maximum: None }, Size::Grow { minimum: None, maximum: None }, Colour::green(), vec!(
			Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Fixed(50), Colour::blue(), Vec::new()),
			Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Fixed(50), Colour::blue(), Vec::new()),
		)).child_gaps(5).alignment(Alignment { x: Alignments::Centre, y: Alignments::Centre }),
		Element::new(Direction::Vertical, Size::Grow { minimum: None, maximum: None }, Size::Grow { minimum: None, maximum: None }, Colour::green(), vec!(
			Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Fixed(50), Colour::blue(), Vec::new()),
			Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Fixed(50), Colour::blue(), Vec::new()),
		)).child_gaps(5).alignment(Alignment { x: Alignments::End, y: Alignments::End }),
	)).indentation(5, 5, 5, 5).child_gaps(5)
}

fn update(user_state: &mut UserStateTest, user_message: UserMessageTest) -> UpdateResponse {
	match user_message {
		UserMessageTest::Test => {},
		UserMessageTest::KeyPressed(key) => {
			match key {
				Key::Named(NamedKey::Escape) => {
					return UpdateResponse::Close;
				},
				Key::Character(character) => {
					let mut text_lock = user_state.text.lock().unwrap();
					*text_lock += &character;
					drop(text_lock);
				},
				Key::Named(NamedKey::Backspace) => {
					let mut text_lock = user_state.text.lock().unwrap();
					text_lock.pop();
					drop(text_lock);
				},
				Key::Named(NamedKey::Space) => {
					let mut text_lock = user_state.text.lock().unwrap();
					text_lock.push(' ');
					drop(text_lock);
				},
				_ => println!("Key Pressed: {key:?}"),
			}
		},
	}
	UpdateResponse::Recalculate
}

fn main() {
	// let font_filename = Path::new("../tapestry/resources/fonts/Geist_Mono/static/GeistMono-Regular.ttf");
	let font_filename = Path::new("../tapestry/resources/fonts/NotoJP/static/NotoSansJP-Regular.ttf");

	let font = Arc::new(Font::new(font_filename));

	let state = UserStateTest { text: Arc::new(Mutex::new("Harrison Jones".into())), font };

	let mut application = MoonlightApplication::new(state, assemble, update);

	application.set_keyboard_input(from_keyboard_input);

	application.run();
}