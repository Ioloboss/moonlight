use mircalla_types::units::Pixels;
use mircalla_types::vectors::{Colour, Dimension, Direction, Position, Size};
use tapestry::font::{Font};
use tapestry::font::font_renderer::{FontRenderer, TextBox};
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::event_loop::EventLoopProxy;
use winit::keyboard::{Key, SmolStr};

use crate::renderer::{ElementRectangle, RendererState};
use crate::window::{open_window, MessageFromMainThread, Window};
use crate::element::{Element, SizingError, Sizing};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::path::Path;
use std::{sync::{mpsc::{self, Receiver}, Arc, Mutex}, thread};

pub enum InternalMessage {
	Window(Arc<Window>),
	Resumed,
	Resized(Size<Pixels<f32>>),
	RedrawRequested,
	Close,
	KeyPressed(KeyEvent),
	MouseEvent(ElementState, MouseButton, Position<Pixels<f32>>),
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

pub enum UpdateResponse {
	Nothing,
	Recalculate,
	Render,
	Close,
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
	root: Option<Element<UserMessage>>, // MAYBE CHANGE THIS?
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
			root: None,
		}
	}

	fn recalculate(&mut self) -> Result<(), SizingError> {
		let user_root = self.assemble.assemble(&self.user_state);
		let size = self.renderer_state.window.inner_size();
		let mut root = Element::new(Direction::Horizontal, (Sizing::Fixed(size.width), Sizing::Fixed(size.height)).into(), Colour::black(), vec![user_root]);

		root.calculate_text_data();

		root.calculate_fit_size(Dimension::Width);
		root.calculate_final_size(Dimension::Width)?;


		root.calculate_fit_size(Dimension::Height);
		root.calculate_final_size(Dimension::Height)?;

		root.calculate_children_position(Position {x: Some(0.0.into()), y: Some(0.0.into())});

		self.renderer_state.font_renderer.text_boxes = root.collect_text_boxes(size);

		let mut element_rectangles = Vec::new();
		root.to_rectangles(&mut element_rectangles);
		
		self.renderer_state.update_element_rectangles_buffer(element_rectangles);

		self.renderer_state.font_renderer.update();
		self.root = Some(root);

		Ok(())
	}

	fn close(&self) {
		self.renderer_state.window.close();
	}

	pub fn run(mut self) {
		loop {
			match self.reciever.recv() {
				Ok(message) => match message {
					InternalMessage::Window(window) => panic!("InternalMessage::Window should only be sent once."),
					InternalMessage::Resumed => panic!("InternalMessage::Resumed should only be sent once."),
					InternalMessage::Resized(size) => {
						self.renderer_state.resize(size);
						self.renderer_state.font_renderer.resize(size);
						self.recalculate().unwrap();
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
								match response {
									UpdateResponse::Nothing => {},
									UpdateResponse::Recalculate => {
										self.recalculate().unwrap();
										self.renderer_state.render().unwrap();
									},
									UpdateResponse::Render => {
										self.renderer_state.render().unwrap();
										todo!("THIS IS NOT CURRENTLY SUPPORTED");
									},
									UpdateResponse::Close => {
										self.close();
										break;
									}
								}
							},
							None => {},
						}
					}
					InternalMessage::MouseEvent(state, button, mouse_position) => {
						match (button, state) {
							(MouseButton::Left, ElementState::Pressed) => {
								let user_message = match &self.root {
									Some(root_element) => root_element.get_on_click(mouse_position),
									None => None,
								};
								if let Some(user_message) = user_message {
									let response = self.update.update(&mut self.user_state, user_message);
									match response {
										UpdateResponse::Nothing => {},
										UpdateResponse::Recalculate => {
											self.recalculate().unwrap();
											self.renderer_state.render().unwrap();
										},
										UpdateResponse::Render => {
											self.renderer_state.render().unwrap();
											todo!("THIS IS NOT CURRENTLY SUPPORTED");
										},
										UpdateResponse::Close => {
											self.close();
											break;
										}
									}
								}
							},
							_ => {},
						}
					},
					_ => todo!("MoonlightApplication::Run reached InternalMessage not implemented yet.")
				},
				Err(e) => panic!("Error when running MoonlightApplication: {e}"),
			}
		}
	}

	pub fn set_keyboard_input<NewKeyboardInput: KeyboardInputFn<UserMessage> + 'static>(&mut self, keyboard_input: NewKeyboardInput) {
		self.keyboard_input = Box::new(keyboard_input);
	}
}