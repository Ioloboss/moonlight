use std::path::Path;
use std::sync::{Arc, Mutex};

use mircalla_types::units::Pixels;
use moonlight::TextBox;
use moonlight::element::{Element, Sizing};
use moonlight::internal_loop::{MoonlightApplication, UpdateResponse};
use tapestry::font::Font;
use tapestry::font::font_renderer::{WrapOn, WrapOptions};
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key, NamedKey, SmolStr};
use mircalla_types::vectors::{Colour, Direction, Size};

#[derive(Debug, Clone)]
enum UserMessageTest {
	Test,
	KeyPressed(Key<SmolStr>),
	ScrollList(Pixels<i32>),
}

fn from_keyboard_input(key_event: KeyEvent) -> Option<UserMessageTest> {
	if key_event.state == ElementState::Pressed {
		Some(UserMessageTest::KeyPressed(key_event.logical_key))
	} else {
		None
	}
}

fn on_scroll(distance: Pixels<i32>) -> Option<UserMessageTest> {
	Some(UserMessageTest::ScrollList(distance))
}

struct UserStateTest {
	text: Arc<Mutex<String>>,
	font: Arc<Font>,
	scrolled: Arc<Mutex<Pixels<i32>>>,
}

fn assemble(user_state: &UserStateTest) -> Element<UserMessageTest> {
	let scroll_children = std::iter::repeat(Element::<UserMessageTest>::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::FitText { minimum: None, maximum: None }).into(), Colour::blue(), Vec::new()).text(TextBox::new(Arc::clone(&user_state.font), Arc::new(Mutex::new("Test Scroll".into())), 30.0.into(), Colour::black(), WrapOptions { wrap_on: WrapOn::Whitespace })).id(12))
		.take(50).collect();

	let scroll_child = Element::new(
		Direction::Vertical,
		Size { width: Sizing::Grow { minimum: None, maximum: None }, height: Sizing::Grow { minimum: None, maximum: None } },
		Colour::green(),
		scroll_children,
	).child_gaps(5.into());

	Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Grow { minimum: None, maximum: None }).into(), Colour::red(), vec!(
		Element::new(Direction::Vertical, (Sizing::Fit { minimum: None, maximum: None }, Sizing::Grow { minimum: None, maximum: None }).into(), Colour::green(), vec!(
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.into())).into(), Colour::blue(), Vec::new()).id(2),
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.into())).into(), Colour::blue(), Vec::new()).on_click(UserMessageTest::Test).id(3),
			Element::new(Direction::Horizontal, (Sizing::FitText { minimum: None, maximum: None }, Sizing::FitText { minimum: None, maximum: None }).into(), Colour::blue(), Vec::new()).text(TextBox::new(Arc::clone(&user_state.font), Arc::clone(&user_state.text), 50.0.into(), Colour::black(), WrapOptions { wrap_on: WrapOn::Whitespace }) ).indentation(0.into(), 0.into(), 0.into(), 0.into()).id(4),
			Element::new(Direction::Horizontal, (Sizing::FitText { minimum: None, maximum: None }, Sizing::FitText { minimum: None, maximum: None }).into(), Colour::blue(), Vec::new()).text(TextBox::new(Arc::clone(&user_state.font), Arc::new(Mutex::new("The reading for this word is quite strange. If you know 二つ's reading, you can use that to remember the 二 part (ふた). But, the り that is the 人 is a total exception, something you won't see too often (though you may have seen it in 一人). If you can use the reading of 一人 (aka if you've learned it already) then definitely use that. If not, do your best to remember the reading on your own. It's a strange one that doesn't connect to much else.".into())), 50.0.into(), Colour::black(), WrapOptions { wrap_on: WrapOn::Whitespace })).indentation(5.into(), 20.into(), 5.into(), 20.into()).id(11),
		)).child_gaps(5.into()).id(1),
		/* Element::new(Direction::Vertical, (Sizing::Grow { minimum: Some(100.into()), maximum: None }, Sizing::Grow { minimum: None, maximum: None }).into(), Colour::green(), vec!(
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.into())).into(), Colour::blue(), Vec::new()).id(6),
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::Fixed(50.into())).into(), Colour::blue(), Vec::new()).id(7),
			Element::new(Direction::Horizontal, (Sizing::Grow { minimum: None, maximum: None }, Sizing::FitText { minimum: None, maximum: None }).into(), Colour::blue(), Vec::new()).text(TextBox::new(Arc::clone(&user_state.font), Arc::new(Mutex::new("Test Alignment".into())), 50.0.into(), Colour::black(), WrapOptions { wrap_on: WrapOn::Whitespace }).alignment(Alignment { x: Alignments::End, y: Alignments::Start }) ).indentation(5.into(), 20.into(), 5.into(), 20.into()).id(11),
		)).child_gaps(5.into()).alignment(Alignment { x: Alignments::Centre, y: Alignments::Centre }).id(5),
		*/
		Element::new(Direction::Vertical, (Sizing::Grow { minimum: Some(100.into()), maximum: None }, Sizing::Scroll { minimum: None, maximum: None, scrolled: user_state.scrolled.clone() }).into(), Colour::green(), vec![scroll_child])
			.id(8)
			.on_scroll(on_scroll)
			.indentation(5.into(), 5.into(), 5.into(), 5.into())
	)).indentation(5.into(), 5.into(), 5.into(), 5.into()).child_gaps(5.into()).id(0)
}

fn update(user_state: &mut UserStateTest, user_message: UserMessageTest) -> UpdateResponse {
	match user_message {
		UserMessageTest::Test => {
			println!("Tested");
			UpdateResponse::nothing()
		},
		UserMessageTest::KeyPressed(key) => {
			match key {
				Key::Named(NamedKey::Escape) => {
					return UpdateResponse::close();
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
				Key::Named(NamedKey::Enter) => {
					let mut text_lock = user_state.text.lock().unwrap();
					text_lock.push('\n');
					drop(text_lock);
				},
				Key::Named(NamedKey::ArrowUp) => {
					let mut scroll_lock = user_state.scrolled.lock().unwrap();
					*scroll_lock += 10.into();
				},
				Key::Named(NamedKey::ArrowDown) => {
					let mut scroll_lock = user_state.scrolled.lock().unwrap();
					*scroll_lock -= 10.into();
				},
				_ => println!("Key Pressed: {key:?}"),
			};
				UpdateResponse::recalculate()

		},
		UserMessageTest::ScrollList(distance) => {
			let mut scroll_lock = user_state.scrolled.lock().unwrap();
			*scroll_lock += distance;
			UpdateResponse::reposition()
		}
	}
}

fn main() {
	// let font_filename = Path::new("../tapestry/resources/fonts/Geist_Mono/static/GeistMono-Regular.ttf");
	let font_filename = Path::new("../tapestry/resources/fonts/NotoJP/static/NotoSansJP-Regular.ttf");

	let font = Arc::new(Font::new(font_filename));

	let state = UserStateTest { text: Arc::new(Mutex::new("Harrison Jones".into())), font, scrolled: Arc::new(Mutex::new(0.into())) };

	let mut application = MoonlightApplication::new(state, assemble, update);

	application.set_keyboard_input(from_keyboard_input);

	application.run();
}