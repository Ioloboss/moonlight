use std::sync::{Arc, Mutex, mpsc};

use crate::element::{Colour, Dimensions, Element, Position};
use crate::internal_loop::{InternalMessage};
use winit::event_loop::EventLoopProxy;

use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::{
	application::ApplicationHandler, error::EventLoopError, event::*, event_loop::{ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey},
};

pub struct Window {
	internal: winit::window::Window,
	event_loop_proxy: EventLoopProxy<MessageFromMainThread>,
}

impl Window {
	pub fn inner_size(&self) -> Dimensions<u32> {
		self.internal.inner_size().into()
	}

	pub fn pre_present_notify(&self) {
		self.internal.pre_present_notify();
	}

	pub fn close(&self) {
		self.event_loop_proxy.send_event(MessageFromMainThread::Close);
	}
}

impl HasWindowHandle for Window{
	fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
		self.internal.window_handle()
	}
}

impl HasDisplayHandle for Window {
	fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
	    self.internal.display_handle()
	}
}

pub enum MessageFromMainThread {
	Close,
}

pub struct App {
	transmitter: mpsc::Sender<InternalMessage>,
	event_loop_proxy: EventLoopProxy<MessageFromMainThread>,
}

impl App {
	pub fn new(transmitter: mpsc::Sender<InternalMessage>, event_loop_proxy: EventLoopProxy<MessageFromMainThread>) -> Self {
		Self {
			transmitter,
			event_loop_proxy,
		}
	}
}

impl ApplicationHandler<MessageFromMainThread> for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let window_attributes = winit::window::Window::default_attributes()
			.with_title("Moonlight Test"); // CHANGE TO RECIEVED FROM APPLICATION

		let window = Arc::new(Window{ 
			internal: event_loop.create_window(window_attributes).unwrap(),
			event_loop_proxy: { self.event_loop_proxy.clone() },
		});
		self.transmitter.send(InternalMessage::Window(window)).unwrap();
		self.transmitter.send(InternalMessage::Resumed).unwrap();
	}

	fn window_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		window_id: winit::window::WindowId,
		event: WindowEvent,
	) {
		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(size) => {
				self.transmitter.send(InternalMessage::Resized(size.into())).unwrap(); // HANDLE THIS PROPERLY
			},
			WindowEvent::RedrawRequested => {
				self.transmitter.send(InternalMessage::RedrawRequested).unwrap() // HANDLE THIS PROPERLY
			},
			WindowEvent::KeyboardInput {
				event:
					KeyEvent {
						physical_key: PhysicalKey::Code(KeyCode::Escape),
						state: ElementState::Pressed,
						..
					},
				..
			} => self.transmitter.send(InternalMessage::Close).unwrap(),
			_ => {},
		}
	}

	fn user_event(&mut self, event_loop: &ActiveEventLoop, event: MessageFromMainThread) {
		match event {
			MessageFromMainThread::Close => event_loop.exit(),
		}
	}
}

pub fn open_window(transmitter: mpsc::Sender<InternalMessage>) {
	let event_loop = EventLoop::<MessageFromMainThread>::with_user_event().with_any_thread(true).build().unwrap();
	let event_loop_proxy = event_loop.create_proxy();
	let mut app = App::new(transmitter, event_loop_proxy);
	event_loop.run_app(&mut app).unwrap();
}