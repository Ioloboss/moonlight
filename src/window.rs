use std::sync::{Arc, mpsc};

use crate::internal_loop::{InternalMessage, ScrollType};
use mircalla_types::units::Pixels;
use mircalla_types::vectors::{Position, Size};
use tapestry::font::ToPixelsSize;
use winit::dpi::PhysicalPosition;
use winit::event_loop::EventLoopProxy;

use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
use winit::platform::wayland::EventLoopBuilderExtWayland;
use winit::{
	application::ApplicationHandler, event::*, event_loop::{ActiveEventLoop, EventLoop},
};

pub struct Window {
	pub internal: Arc<winit::window::Window>,
	event_loop_proxy: EventLoopProxy<MessageFromMainThread>,
}

impl Window {
	pub fn inner_size(&self) -> Size<Pixels<i32>> {
		self.internal.inner_size().to_pixels_size()
	}

	pub fn pre_present_notify(&self) {
		self.internal.pre_present_notify();
	}

	pub fn close(&self) {
		self.event_loop_proxy.send_event(MessageFromMainThread::Close).unwrap();
	}

	pub fn set_title(&self, title: &str) {
		self.internal.set_title(title);
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

#[derive(Debug)]
pub enum MessageFromMainThread {
	Close,
}

pub struct App {
	transmitter: mpsc::Sender<InternalMessage>,
	event_loop_proxy: EventLoopProxy<MessageFromMainThread>,
	mouse_position: Position<Pixels<i32>>,
}

impl App {
	pub fn new(transmitter: mpsc::Sender<InternalMessage>, event_loop_proxy: EventLoopProxy<MessageFromMainThread>) -> Self {
		Self {
			transmitter,
			event_loop_proxy,
			mouse_position: (0, 0).into(),
		}
	}
}

impl ApplicationHandler<MessageFromMainThread> for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let window_attributes = winit::window::Window::default_attributes()
			.with_title("Moonlight Application"); // CHANGE TO RECIEVED FROM APPLICATION

		let window = Arc::new(Window{ 
			internal: Arc::new(event_loop.create_window(window_attributes).unwrap()),
			event_loop_proxy: { self.event_loop_proxy.clone() },
		});
		self.transmitter.send(InternalMessage::Window(window)).unwrap();
		self.transmitter.send(InternalMessage::Resumed).unwrap();
	}

	fn window_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		_window_id: winit::window::WindowId,
		event: WindowEvent,
	) {
		match event {
			WindowEvent::CloseRequested => event_loop.exit(),
			WindowEvent::Resized(size) => {
				self.transmitter.send(InternalMessage::Resized(size.to_pixels_size())).unwrap(); // HANDLE THIS PROPERLY
			},
			WindowEvent::RedrawRequested => {
				self.transmitter.send(InternalMessage::RedrawRequested).unwrap(); // HANDLE THIS PROPERLY
			},
			WindowEvent::KeyboardInput { event, .. } => {
				self.transmitter.send(InternalMessage::KeyPressed(event)).unwrap(); // HANDLE THIS PROPERLY
			},
			WindowEvent::MouseInput { device_id, state, button } => {
				self.transmitter.send(InternalMessage::MouseEvent(state, button, self.mouse_position)).unwrap(); // HANDLE THIS PROPERLY
			},
			WindowEvent::CursorMoved { device_id, position } => {
				self.mouse_position = (position.x as i32, position.y as i32).into()
			},
			WindowEvent::MouseWheel { device_id, delta, phase } => {
				let (distance, scroll_type) = match delta {
					MouseScrollDelta::LineDelta(horizontal, vertical) => {
						((vertical as i32).into(), ScrollType::Line)
					},
					MouseScrollDelta::PixelDelta(PhysicalPosition {x, y}) => {
						((y as i32).into(), ScrollType::Pixel)
					}
				};
				self.transmitter.send(InternalMessage::Scroll(distance, self.mouse_position, scroll_type)).unwrap();
			}
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