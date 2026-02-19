use std::path::Path;
use std::sync::{Arc, Mutex};

use moonlight::element::{Alignment, Alignments, Element, Sizing};
use moonlight::internal_loop::{MoonlightApplication, UpdateResponse};
use tapestry::font::Font;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey, SmolStr};
use mircalla_types::vectors::{Colour, Direction};

#[derive(Debug, Clone)]
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
	Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Grow { minimum: None, maximum: None }).into(), Colour::red(), vec!(
		Element::new(Direction::Vertical, (Sizing::Fit { minimum: None, maximum: None }, Sizing::Grow { minimum: None, maximum: None }).into(), Colour::green(), vec!(
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.0.into())).into(), Colour::blue(), Vec::new()),
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.0.into())).into(), Colour::blue(), Vec::new()).on_click(UserMessageTest::Test),
			Element::new(Direction::Horizontal, (Sizing::FitText { minimum: None, maximum: None }, Sizing::FitText { minimum: None, maximum: None }).into(), Colour::blue(), Vec::new()).text(tapestry::font::font_renderer::TextBox { font: Arc::clone(&user_state.font), text: Arc::clone(&user_state.text), pixels_per_em: 50.0.into(), position: (0.0, 0.0).into(), colour: Colour::black() })
		)).child_gaps(5.0.into()),
		Element::new(Direction::Vertical, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Grow { minimum: None, maximum: None }).into(), Colour::green(), vec!(
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.0.into())).into(), Colour::blue(), Vec::new()),
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.0.into())).into(), Colour::blue(), Vec::new()),
		)).child_gaps(5.0.into()).alignment(Alignment { x: Alignments::Centre, y: Alignments::Centre }),
		Element::new(Direction::Vertical, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Grow { minimum: None, maximum: None }).into(), Colour::green(), vec!(
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.0.into())).into(), Colour::blue(), Vec::new()),
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.0.into())).into(), Colour::blue(), Vec::new()),
		)).child_gaps(5.0.into()).alignment(Alignment { x: Alignments::End, y: Alignments::End }),
	)).indentation(5.0.into(), 5.0.into(), 5.0.into(), 5.0.into()).child_gaps(5.0.into())
}

fn update(user_state: &mut UserStateTest, user_message: UserMessageTest) -> UpdateResponse {
	match user_message {
		UserMessageTest::Test => {
			println!("Tested");
		},
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