mod element;
mod renderer;
mod internal_loop;
mod window;

#[cfg(test)]
mod tests {
	use crate::element::{Alignment, Alignments, Colour, Dimension, Direction, Element, Position, Size};
	use crate::internal_loop::MoonlightApplication;

	use super::*;

	enum UserMessageTest {
		Test,
	}

	struct UserStateTest {
		text: String,
	}

	fn assemble(user_state: &UserStateTest) -> Element<UserMessageTest>{
		Element::new(Direction::Horizontal, Size::Grow { minimum: None, maximum: None }, Size::Grow { minimum: None, maximum: None }, Colour::red(), vec!(
			Element::new(Direction::Vertical, Size::Fixed(100), Size::Grow { minimum: None, maximum: None }, Colour::green(), vec!(
				Element::new(Direction::Horizontal, Size::Fixed(100), Size::Fixed(50), Colour::blue(), Vec::new()),
				Element::new(Direction::Horizontal, Size::Fixed(100), Size::Fixed(50), Colour::blue(), Vec::new()),
				Element::new(Direction::Horizontal, Size::FitText { minimum: None, maximum: None }, Size::FitText { minimum: None, maximum: None }, Colour::blue(), Vec::new()).text(user_state.text.clone())
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

	#[test]
	fn layout_engine_basic() {
		let state = UserStateTest { text: "Harrison".into() };

		MoonlightApplication::new(state, assemble).run();

	}
}
