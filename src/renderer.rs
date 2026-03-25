use std::sync::Arc;

use crate::window::Window;

use mircalla_types::units::Pixels;
use mircalla_types::vectors::{Colour, Position, Size};
use tapestry::font::font_renderer::FontRenderer;
use wgpu::util::DeviceExt;

#[derive(Clone, Debug)]
pub enum NewRendererStateError {
	RequestAdapterError(wgpu::RequestAdapterError),
	RequestDeviceError(wgpu::RequestDeviceError),
}

impl From<wgpu::RequestAdapterError> for NewRendererStateError {
	fn from(value: wgpu::RequestAdapterError) -> Self {
		Self::RequestAdapterError(value)
	}
}

impl From<wgpu::RequestDeviceError> for NewRendererStateError {
	fn from(value: wgpu::RequestDeviceError) -> Self {
	    Self::RequestDeviceError(value)
	}
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
	position: [f32; 2],
}

impl Vertex {
	fn desc() -> wgpu::VertexBufferLayout<'static> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x2,
				},
			],
		}
	}
}

pub struct ElementRectangle {
	pub position: Position<Pixels<i32>>,
	pub size: Size<Pixels<i32>>,
	pub colour: Colour,
	pub bounds: Option<(Position<Pixels<i32>>, Position<Pixels<i32>>)>,
	pub id: Option<u64>,
}

impl ElementRectangle {
	fn to_raw(&self, screen_size: Size<Pixels<i32>>) -> ElementRectangleRaw {
		let mut x = self.position.x;
		let mut y = screen_size.height - self.position.y - self.size.height;

		let mut width= self.size.width;
		let mut height= self.size.height;

		if let Some((lower_left_corner, upper_right_corner)) = self.bounds {
			if x < lower_left_corner.x {
				let difference = lower_left_corner.x - x;
				x = lower_left_corner.x;
				width -= difference;
			}

			if x + width > upper_right_corner.x {
				let difference = (x + width) - lower_left_corner.x;
				width -= difference;
			}

			if y < lower_left_corner.y.into() {
				let difference = lower_left_corner.y - y;
				y = lower_left_corner.y.into();
				height -= difference;
			}

			if y + height > upper_right_corner.y.into() {
				let difference = (y + height) - upper_right_corner.y;
				height -= difference;
			}
		}

		let normalised_x = x.to_screen_space(screen_size.width);
		let normalised_y = y.to_screen_space(screen_size.height);
		let normalised_width = width.to_screen_space_length(screen_size.width);
		let normalised_height = height.to_screen_space_length(screen_size.height);
		ElementRectangleRaw { position: [normalised_x.value, normalised_y.value],
			size: [normalised_width.value, normalised_height.value],
			colour: self.colour.into(),
		}
	}
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ElementRectangleRaw {
	position: [f32; 2],
	size: [f32; 2],
	colour: [f32; 3],
}

impl ElementRectangleRaw {
	fn dec() -> wgpu::VertexBufferLayout<'static> {
		use std::mem;
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<ElementRectangleRaw>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Instance,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x2,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x2,
				},
				wgpu::VertexAttribute {
					offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
					shader_location: 3,
					format: wgpu::VertexFormat::Float32x3,
				},
			],
		}
	}
}

const ELEMENT_RECTANGLE_VERTICES: &[Vertex] = &[
	Vertex { position: [0.0, 1.0] },
	Vertex { position: [0.0, 0.0] },
	Vertex { position: [1.0, 1.0] },
	Vertex { position: [1.0, 0.0] },
];

const ELEMENT_RECTANGLE_INDICES: &[u16] = &[
	0, 1, 2, 3,
];

pub struct RendererState {
	surface: wgpu::Surface<'static>,
	device: Arc<wgpu::Device>,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	is_surface_configured: bool,
	render_pipeline: wgpu::RenderPipeline,
	element_rectangle_vertex_buffer: wgpu::Buffer,
	element_rectangle_index_buffer: wgpu::Buffer,
	pub element_rectangles: Vec<ElementRectangle>,
	number_of_elements: u32,
	element_rectangle_buffer: wgpu::Buffer,
	pub window: Arc<Window>,
	pub font_renderer: FontRenderer,
}

impl RendererState {
	pub async fn new(window: Arc<Window>) -> Result<Self, NewRendererStateError> {
		println!("New RenderState");

		let size: Size<Pixels<i32>> = window.inner_size();

		let element_rectangles = vec![
			ElementRectangle { position: Position { x: 0.into(), y: 0.into() }, size, colour: Colour::black(), bounds: None, id: None, }
		];

		let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			backends: wgpu::Backends::PRIMARY,
			..Default::default()
		});

		let surface = instance.create_surface(window.clone()).unwrap();

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(), // MAY WANT TO SWITCH TO LOW POWER
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await?;

		let (device, queue) = adapter
			.request_device(&wgpu::DeviceDescriptor {
				label: None, // MAY WANT TO GIVE LABEL
				required_features: wgpu::Features::empty(),
				required_limits: wgpu::Limits::defaults(),
				memory_hints: Default::default(),
				trace: wgpu::Trace::Off,
				experimental_features: wgpu::ExperimentalFeatures::disabled(),
			})
			.await?;

		let device = Arc::new(device);

		let surface_capabilities = surface.get_capabilities(&adapter);

		let surface_format = surface_capabilities.formats.iter()
			.find(|f| f.is_srgb())
			.copied()
			.unwrap_or(surface_capabilities.formats[0]);

		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width.value as u32,
			height: size.height.value as u32,
			present_mode: surface_capabilities.present_modes[0],
			alpha_mode: surface_capabilities.alpha_modes[0],
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};


		let font_renderer = pollster::block_on(FontRenderer::new(Arc::clone(&(window.internal)), Arc::clone(&device), &config)).unwrap(); // NEED TO FIX THIS

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("Moonlight Shader"),
			source: wgpu::ShaderSource::Wgsl(include_str!("../resources/shaders/element_rectangle.wgsl").into()),
		});

		let element_rectangle_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Element Rectangle Vertex Buffer"),
			contents: bytemuck::cast_slice(ELEMENT_RECTANGLE_VERTICES),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let number_of_elements = element_rectangles.len() as u32;
		let element_rectangles_data = element_rectangles.iter().map(|element_rectangle| element_rectangle.to_raw(size.into())).collect::<Vec<_>>();

		let element_rectangle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Element Rectangle Buffer"),
			contents: bytemuck::cast_slice(&element_rectangles_data),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let element_rectangle_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Element Rectangle Index Buffer"),
			contents: bytemuck::cast_slice(ELEMENT_RECTANGLE_INDICES),
			usage: wgpu::BufferUsages::INDEX,
		});

		let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Render Pipeline Layout"),
			bind_group_layouts: &[],
			push_constant_ranges: &[],
		});

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Render Pipeline"),
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: Some("vs_main"),
				buffers: &[
					Vertex::desc(),
					ElementRectangleRaw::dec(),
				],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: Some("fs_main"),
				targets: &[Some(wgpu::ColorTargetState {
					format: config.format,
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleStrip,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: Some(wgpu::Face::Back),
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false,
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState {
				count: 1,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			multiview: None,
			cache: None,
		});

		println!("New RenderState Created");

		Ok(Self {
			surface,
			device,
			queue,
			config,
			is_surface_configured: false,
			render_pipeline,
			element_rectangle_vertex_buffer,
			element_rectangle_index_buffer,
			element_rectangles,
			number_of_elements,
			element_rectangle_buffer,
			window,
			font_renderer,
		})
	}

	pub fn resize(&mut self, size: Size<Pixels<i32>>) {
		if size.width.value > 0 && size.height.value > 0 {
			self.config.width = size.width.value as u32;
			self.config.height = size.height.value as u32;
			self.surface.configure(&self.device, &self.config);
			self.font_renderer.resize(size);
			self.is_surface_configured = true;
		}
	}

	pub fn update_element_rectangles_buffer(&mut self, element_rectangles: Vec<ElementRectangle>) {
		self.number_of_elements = element_rectangles.len() as u32;
		let element_rectangles_data = element_rectangles.iter().map(|element_rectangle| element_rectangle.to_raw(self.window.inner_size().into())).collect::<Vec<_>>();

		self.element_rectangle_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Element Rectangle Buffer"),
			contents: bytemuck::cast_slice(&element_rectangles_data),
			usage: wgpu::BufferUsages::VERTEX,
		});
	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		
		if !self.is_surface_configured {
			return  Ok(());
		}

		let output = self.surface.get_current_texture()?;

		let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
			label: Some("Moonlight TextureView"),
			..Default::default()
		});

		// println!("\nMoonlight TextureView: {view:?}");

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Render Pass"),
				color_attachments: &[
					Some(wgpu::RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(
								wgpu::Color {
									r: 0.0,
									g: 0.0,
									b: 1.0,
									a: 1.0,
								}
							),
							store: wgpu::StoreOp::Store,
						},
						depth_slice: None,
					})
				],
				depth_stencil_attachment: None,
				occlusion_query_set: None,
				timestamp_writes: None,
			});

			render_pass.set_pipeline(&self.render_pipeline);
			render_pass.set_vertex_buffer(0, self.element_rectangle_vertex_buffer.slice(..));
			render_pass.set_vertex_buffer(1, self.element_rectangle_buffer.slice(..));
			render_pass.set_index_buffer(self.element_rectangle_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
			render_pass.draw_indexed(0..4, 0, 0..self.number_of_elements);
		}

		self.queue.submit(std::iter::once(encoder.finish()));

		self.font_renderer.draw_text(&self.queue, &view);

		self.window.pre_present_notify();
		output.present();

		Ok(())
	}
}
