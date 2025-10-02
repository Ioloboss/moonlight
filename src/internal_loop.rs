use winit::event_loop::EventLoopProxy;

use crate::renderer::{ElementRectangle, RendererState};
use crate::window::{open_window, MessageFromMainThread, Window};
use crate::element::{Colour, Dimensions, Dimension, Direction, Element, Position, Size, SizeError};
use std::marker::PhantomData;
use std::{sync::{mpsc::{self, Receiver}, Arc, Mutex}, thread};

pub enum InternalMessage {
	Window(Arc<Window>),
	Resumed,
	Resized(Dimensions<u32>),
	RedrawRequested,
	Close,
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

pub struct MoonlightApplication<UserState, UserMessage, Assemble: AssembleFn<UserState, UserMessage>> {
	user_state: UserState,
	renderer_state: RendererState,
	reciever: Receiver<InternalMessage>,
	assemble: Assemble,
	what: PhantomData<UserMessage>, // REMOVE THIS IF POSSIBLE
}

impl<UserState, UserMessage, Assemble: AssembleFn<UserState, UserMessage>> MoonlightApplication<UserState, UserMessage, Assemble> {
	pub fn new(user_state: UserState, assemble: Assemble) -> Self {
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
			what: PhantomData,
		}
	}


	fn recalculate(&mut self) -> Result<(), SizeError> {
		let user_root = self.assemble.assemble(&self.user_state);
		let size = self.renderer_state.window.inner_size();
		let mut root = Element::new(Direction::Horizontal, Size::Fixed(size.width as u64), Size::Fixed(size.height as u64), Colour::black(), vec![user_root]);

		root.calculate_text_data();

		root.calculate_fit_size(Dimension::Width);
		root.calculate_final_size(Dimension::Width)?;

		root.calculate_fit_size(Dimension::Height);
		root.calculate_final_size(Dimension::Height)?;

		root.calculate_children_position(Position {x: Some(0), y: Some(0)});

		let mut element_rectangles = Vec::new();
		root.to_rectangles(&mut element_rectangles);
		
		self.renderer_state.update_element_rectangles_buffer(element_rectangles);

		Ok(())
	}

	pub fn run(mut self) {
		loop {
			match self.reciever.recv() {
				Ok(message) => match message {
					InternalMessage::Window(window) => panic!("InternalMessage::Window should only be sent once."),
					InternalMessage::Resumed => panic!("InternalMessage::Resumed should only be sent once."),
					InternalMessage::Resized(size) => {
						self.renderer_state.resize(size);
						self.recalculate().unwrap();
						self.renderer_state.render();
					},
					InternalMessage::RedrawRequested => {
						match self.renderer_state.render() {
							Ok(_) => {},
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
						self.renderer_state.window.close();
						break;
					},
					_ => todo!("MoonlightApplication::Run reached InternalMessage not implemented yet.")
				},
				Err(e) => panic!("Error when running MoonlightApplication: {e}"),
			}
		}
	}
}