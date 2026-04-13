use mircalla_types::units::Pixels;
use mircalla_types::vectors::{Colour, Dimension, Direction, Position, Size};
use winit::event::{ElementState, KeyEvent, MouseButton};

use crate::renderer::{RendererState};
use crate::window::{open_window, Window};
use crate::element::{Element, SizingError, Sizing};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::{sync::{mpsc::{self, Receiver}, Arc}, thread};

pub enum InternalMessage {
	Window(Arc<Window>),
	Resumed,
	Resized(Size<Pixels<i32>>),
	RedrawRequested,
	Close,
	KeyPressed(KeyEvent),
	MouseEvent(ElementState, MouseButton, Position<Pixels<i32>>),
	Scroll(Pixels<i32>, Position<Pixels<i32>>, ScrollType),
}

pub enum ScrollType {
	Pixel,
	Line,
}

trait Update<UserState, UserMessage> {
	fn update(&self, user_state: &mut UserState, user_message: UserMessage);
}

impl<UserState, UserMessage, T> Update<UserState, UserMessage> for T
where
	T: Fn(&mut UserState, UserMessage),
{
	fn update(&self, user_state: &mut UserState, user_message: UserMessage) {
		self(user_state, user_message)
	}
}

trait AssembleFn<UserState, UserMessage> {
	fn assemble(&self, user_state: &UserState) -> Element<UserMessage>;
}

impl<UserState, UserMessage, T> AssembleFn<UserState, UserMessage> for T
where
	T: Fn(&UserState) -> Element<UserMessage>
{
	fn assemble(&self, user_state: &UserState) -> Element<UserMessage> {
		self(user_state)
	}
}

trait KeyboardInputFn<UserMessage> {
	fn keyboard_input(&self, key_event: KeyEvent) -> Option<UserMessage>;
}

impl<UserMessage, T> KeyboardInputFn<UserMessage> for T
where
	T: Fn(KeyEvent) -> Option<UserMessage>
{
	fn keyboard_input(&self, key_event: KeyEvent) -> Option<UserMessage> {
		self(key_event)
	}
}

pub struct NoKeyboardInput;

impl<UserMessage> KeyboardInputFn<UserMessage> for NoKeyboardInput {
	fn keyboard_input(&self, _: KeyEvent) -> Option<UserMessage> {
		None
	}
}

trait UpdateFn<UserState, UserMessage> {
	fn update(&self, user_state: &mut UserState, user_message: UserMessage) -> UpdateResponse;
}

impl<UserState, UserMessage, T> UpdateFn<UserState, UserMessage> for T
where
	T: Fn(&mut UserState, UserMessage) -> UpdateResponse
{
	fn update(&self, user_state: &mut UserState, user_message: UserMessage) -> UpdateResponse {
		self(user_state, user_message)
	}
}

pub struct UpdateResponse {
	pub reassemble: bool,
	pub recalculate_size: bool,
	pub recalculate_position: bool,
	pub update_render_state: bool,
	pub render: bool,
	pub close: bool,
}

impl UpdateResponse {
	pub fn close() -> UpdateResponse {
		UpdateResponse { reassemble: false, recalculate_size: false, recalculate_position: false, update_render_state: false, render: false, close: true }
	}

	pub fn recalculate() -> UpdateResponse {
		UpdateResponse { reassemble: true, recalculate_size: true, recalculate_position: true, update_render_state: true, render: true, close: false }
	}

	pub fn nothing() -> UpdateResponse {
		UpdateResponse { reassemble: false, recalculate_size: false, recalculate_position: false, update_render_state: false, render: false, close: false }
	}

	pub fn reposition() -> UpdateResponse {
		UpdateResponse { reassemble: false, recalculate_size: false, recalculate_position: true, update_render_state: true, render: true, close: false }
	}
}

pub struct MoonlightApplication<UserState, UserMessage, Assemble, Update>
where
	UserMessage: Debug + Clone,
	Assemble: AssembleFn<UserState, UserMessage>,
	Update: UpdateFn<UserState, UserMessage>,
{
	user_state: UserState,
	renderer_state: RendererState,
	reciever: Receiver<InternalMessage>,
	assemble: Assemble,
	update: Update,
	keyboard_input: Box<dyn KeyboardInputFn<UserMessage>>,
	what: PhantomData<UserMessage>, // REMOVE THIS IF POSSIBLE
	root: Element<UserMessage>, // MAYBE CHANGE THIS?
	line_scroll_distance: Pixels<i32>,
}

impl<UserState, UserMessage: Debug + Clone, Assemble: AssembleFn<UserState, UserMessage>, Update: UpdateFn<UserState, UserMessage>> MoonlightApplication<UserState, UserMessage, Assemble, Update> {
	pub fn new(user_state: UserState, assemble: Assemble, update: Update) -> Self {
		env_logger::init();
		let (transmitter, reciever) = mpsc::channel::<InternalMessage>();
		let transmitter_renderer = transmitter.clone();
		thread::spawn(move || open_window(transmitter_renderer));

		let window = match reciever.recv().unwrap() {
			InternalMessage::Window(window) => window,
			_ => panic!("First message should be InternalState::Window"),
		};

		match reciever.recv().unwrap() {
			InternalMessage::Resumed => {},
			_ => panic!("Second message should be InternalState::Resumed"),
		};

		let renderer_state = pollster::block_on(RendererState::new(window)).unwrap();

		Self {
			user_state,
			renderer_state,
			reciever,
			assemble,
			update,
			keyboard_input: Box::new(NoKeyboardInput),
			what: PhantomData,
			root: Element::new(Direction::Horizontal, Size { width: Sizing::Grow { minimum: None, maximum: None }, height: Sizing::Grow { minimum: None, maximum: None } }, Colour { r: 0.0, g: 0.0, b: 0.0 }, Vec::new()),
			line_scroll_distance: 20.into(),
		}
	}

	fn recalculate(&mut self, update_response: UpdateResponse) -> Result<(), SizingError> {

		let size = self.renderer_state.window.inner_size();

		if update_response.reassemble {
			let user_root = self.assemble.assemble(&self.user_state);
			self.root = Element::new(Direction::Horizontal, (Sizing::Fixed(size.width), Sizing::Fixed(size.height)).into(), Colour::black(), vec![user_root]);
		}

		if update_response.recalculate_size {
			self.root.calculate_text_data();

			self.root.calculate_fit_size(Dimension::Width);
			self.root.calculate_final_size(Dimension::Width)?;

			//root.wrap_text();

			self.root.calculate_fit_size(Dimension::Height);
			self.root.calculate_final_size(Dimension::Height)?;
		}



		if update_response.recalculate_position {
			self.root.calculate_children_position(Position {x: Some(0.into()), y: Some(0.into())}, size);
		}	

		if update_response.update_render_state {
			self.renderer_state.font_renderer.text_boxes = self.root.collect_text_boxes(size);

			let mut element_rectangles = Vec::new();
			self.root.to_rectangles(size, &mut element_rectangles);
			
			self.renderer_state.update_element_rectangles_buffer(element_rectangles);

			self.renderer_state.font_renderer.update();
		}

		if update_response.render {
			self.renderer_state.render().unwrap();
		}

		Ok(())
	}

	fn close(&self) {
		self.renderer_state.window.close();
	}

	pub fn run(mut self) {
		println!("Running MoonlightApplication");
		loop {
			match self.reciever.recv() {
				Ok(message) => match message {
					InternalMessage::Window(_window) => panic!("InternalMessage::Window should only be sent once."),
					InternalMessage::Resumed => panic!("InternalMessage::Resumed should only be sent once."),
					InternalMessage::Resized(size) => {
						self.renderer_state.resize(size);
						self.recalculate(UpdateResponse::recalculate()).unwrap();
						self.renderer_state.render().unwrap();
					},
					InternalMessage::RedrawRequested => {
						match self.renderer_state.render() {
							Ok(_) => {
							},
							Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
								let size = self.renderer_state.window.inner_size();
								self.renderer_state.resize(size);
							},
							Err(e) => {
								log::error!("Unable to render. Error: {e}");
							},
						}
					},
					InternalMessage::Close => {
						self.close();
						break;
					},
					InternalMessage::KeyPressed(key) => {
						match self.keyboard_input.keyboard_input(key) {
							Some(keyboard_input) => {
								let response = self.update.update(&mut self.user_state, keyboard_input);
								if response.close {
									self.close();
									break;
								} else {
									self.recalculate(response).unwrap();
								}
							},
							None => {},
						}
					}
					InternalMessage::MouseEvent(state, button, mouse_position) => {
						match (button, state) {
							(MouseButton::Left, ElementState::Pressed) => {
								let user_message = self.root.get_on_click(mouse_position);
								if let Some(user_message) = user_message {
									let response = self.update.update(&mut self.user_state, user_message);
									if response.close {
										self.close();
										break;
									} else {
										self.recalculate(response).unwrap();
									}
								}
							},
							_ => {},
						}
					},
					InternalMessage::Scroll(distance, mouse_position, scroll_type) => {
						let user_message = {
							let on_scroll = self.root.get_on_scroll(mouse_position);
							match on_scroll {
								Some(on_scroll) => {
									on_scroll.on_scroll(
										match scroll_type {
											ScrollType::Line => distance * self.line_scroll_distance,
											ScrollType::Pixel => distance,
										}
									)
								},
								None => None,
							}
						};
						if let Some(user_message) = user_message {
							let response = self.update.update(&mut self.user_state, user_message);
							if response.close {
								self.close();
								break;
							} else {
								self.recalculate(response).unwrap();
							}
						}
					}
					_ => todo!("MoonlightApplication::Run reached InternalMessage not implemented yet.")
				},
				Err(e) => panic!("Error when running MoonlightApplication: {e}"),
			}
		}
	}

	pub fn set_keyboard_input<NewKeyboardInput: KeyboardInputFn<UserMessage> + 'static>(&mut self, keyboard_input: NewKeyboardInput) {
		self.keyboard_input = Box::new(keyboard_input);
	}

	pub fn set_line_scroll_distance(&mut self, scroll_distance: Pixels<i32>) {
		self.line_scroll_distance = scroll_distance;
	}

	pub fn set_title(&self, title: &str) {
		self.renderer_state.window.set_title(title);
	}
}