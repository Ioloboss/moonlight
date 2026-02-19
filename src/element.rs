use std::sync::{Arc, Mutex};

use tapestry::font::{Font, Pixels, font_renderer::TextBox};
use winit::dpi::PhysicalSize;

use crate::renderer::ElementRectangle;

#[derive(Clone, Copy, Debug)]
pub enum Direction {
	Vertical,
	Horizontal,
}

#[derive(Clone, Copy, Debug)]
pub enum Dimension {
	Width,
	Height,
}

impl Dimension {
	pub fn opposite(&self) -> Self {
		match self {
			Dimension::Width => Dimension::Height,
			Dimension::Height => Dimension::Width,
		}
	}
}

impl From<Direction> for Dimension {
	fn from(value: Direction) -> Self {
		match value {
			Direction::Horizontal => Dimension::Width,
			Direction::Vertical => Dimension::Height,
		}
	}
}

#[derive(Clone, Copy)]
pub struct Dimensions<T: Copy> {
	pub width: T,
	pub height: T,
}

impl<T: Copy> Dimensions<T> {
	fn get(&self, dimension: Dimension) -> T {
		match dimension {
			Dimension::Width => self.width,
			Dimension::Height => self.height,
		}
	}

	fn set(&mut self, dimension: Dimension, value: T) {
		match dimension {
			Dimension::Width => self.width = value,
			Dimension::Height => self.height = value,
		}
	}
}

impl<T: Copy> Dimensions<Option<T>> {
	pub fn none() -> Self {
		Dimensions { width: None, height:None }
	}

	pub fn unwrap_contents(self) -> Dimensions<T> {
		Dimensions { width: self.width.unwrap(), height: self.height.unwrap() }
	}
}

impl<T: Copy> From<PhysicalSize<T>> for Dimensions<T> {
	fn from(value: PhysicalSize<T>) -> Self {
		Dimensions {
			width: value.width,
			height: value.height,
		}
	}
}

#[derive(Clone, Copy)]
pub enum Axis {
	X,
	Y,
}

impl From<Dimension> for Axis {
	fn from(value: Dimension) -> Self {
		match value {
			Dimension::Width => Axis::X,
			Dimension::Height => Axis::Y,
		}
	}
}

#[derive(Clone, Copy)]
pub struct Position {
	pub x: Option<u64>,
	pub y: Option<u64>,
}

impl Position {
	fn get(&self, axis: Axis) -> Option<u64> {
		match axis {
			Axis::X => self.x,
			Axis::Y => self.y,
		}
	}

	fn set(&mut self, axis: Axis, value: Option<u64>) {
		match axis {
			Axis::X => self.x = value,
			Axis::Y => self.y = value,
		}
	}

	fn none() -> Self {
		Position { x: None, y: None }
	}
}

#[derive(Clone, Copy)]
pub enum Alignments {
	Start,
	Centre,
	End,
}

#[derive(Clone, Copy)]
pub struct Alignment {
	pub x: Alignments,
	pub y: Alignments,
}

impl Alignment {
	pub fn get(&self, axis: Axis) -> Alignments {
		match axis {
			Axis::X => self.x,
			Axis::Y => self.y,
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub enum Size {
	Fixed( u64 ),
	Fit{ minimum: Option<u64>, maximum: Option<u64> },
	Grow{ minimum: Option<u64>, maximum: Option<u64> },
	FitText { minimum: Option<u64>, maximum: Option<u64> },
}

#[derive(Clone, Copy, Debug)]
pub enum SizeError {
	CantShrinkChildren(u64),
	CantGrow(),
}

#[derive(Clone, Copy)]
pub struct Colour {
	pub r: f32,
	pub g: f32,
	pub b: f32,
}

impl Colour {
	pub fn black() -> Self {
		Colour { r: 0.0, g: 0.0, b: 0.0 }
	}

	pub fn red() -> Self {
		Colour { r: 1.0, g: 0.0, b: 0.0 }
	}

	pub fn green() -> Self {
		Colour { r: 0.0, g: 1.0, b: 0.0 }
	}

	pub fn blue() -> Self {
		Colour { r: 0.0, g: 0.0, b: 1.0 }
	}
}

pub struct Element<UserMessage> {
	direction: Direction,
	size: Dimensions<Size>,
	colour: Colour,
	children: Vec<Element<UserMessage>>,
	// text: Option<Arc<Mutex<String>>>,
	text: Option<TextBox>,
	on_click: Option<UserMessage>,
	child_gaps: u64,
	indentation: (u64, u64, u64, u64),
	alignment: Alignment,
	id: Option<u64>,
	// Working values changed by layout engine.
	calculated_fit_size: Dimensions<Option<u64>>,
	assigned_size: Dimensions<Option<u64>>,
	position: Position,
	text_minimum: Option<u64>,
	text_ideal: Option<u64>,
}

impl<UserMessage: Clone> Element<UserMessage> {
	pub fn new(direction: Direction, width: Size, height: Size, colour: Colour, children: Vec<Element<UserMessage>>) -> Self {
		Element {
			direction,
			size: Dimensions { width, height },
			colour,
			children,
			text: None,
			on_click: None,
			child_gaps: 0,
			indentation: (0, 0, 0, 0),
			alignment: Alignment {x: Alignments::Start, y: Alignments::Start},
			id: None,
			calculated_fit_size: Dimensions::none(),
			assigned_size: Dimensions::none(),
			position: Position::none(),
			text_minimum: None,
			text_ideal: None,
		}
	}

	pub fn text(mut self, text: TextBox) -> Self {
		self.text = Some(text);
		self
	}

	pub fn on_click(mut self, on_click: UserMessage) -> Self {
		self.on_click = Some(on_click);
		self
	}

	pub fn child_gaps(mut self, child_gaps: u64) -> Self {
		self.child_gaps = child_gaps;
		self
	}

	pub fn indentation(mut self, top: u64, right: u64, bottom: u64, left: u64) -> Self {
		self.indentation = (top, right, bottom, left);
		self
	}

	pub fn alignment(mut self, alignment: Alignment) -> Self {
		self.alignment = alignment;
		self
	}

	pub fn id(mut self, id: u64) -> Self {
		self.id = Some(id);
		self
	}
	
	pub fn calculate_text_data(&mut self) {
		match &self.text {
			Some(text) => {
				self.text_minimum = Some(20); // TWENTY IS THE PREDEND WIDTH OF A CHARACTER NOT SOME SPECIAL VALUE.
				self.text_ideal = Some(text.get_ideal_width() as u64 + self.indentation.1 + self.indentation.3);
			},
			None => {
				self.text_minimum = None;
				self.text_ideal = None;
			},
		}

		for child in self.children.iter_mut() {
			child.calculate_text_data();
		}
	}

	pub fn collect_text_boxes(&self, screen_size: Dimensions<u32>) -> Vec<TextBox> {
		let mut text_boxes: Vec<TextBox> = Vec::new();

		match &self.text {
			Some(text) => {
				let text_box = TextBox {
					font: Arc::clone(&text.font),
					text: Arc::clone(&text.text),
					pixels_per_em: text.pixels_per_em,
					position: ((self.position.x.unwrap() + self.indentation.3) as f32, ((screen_size.height as u64 - self.position.y.unwrap() - self.assigned_size.height.unwrap() + self.indentation.2) as f32 + text.font.typographic_descender.to_pixels(text.get_pixels_per_font_unit()).value ) as f32).into(),
					colour: text.colour,
				};
				text_boxes.push(text_box);
			},
			None => {},
		}

		for child in self.children.iter() {
			text_boxes.append(&mut child.collect_text_boxes(screen_size));
		}

		text_boxes
	}

	fn calculate_text_height(&self) -> u64 {
		match &self.text {
			Some(text) => {
				text.get_height() as u64 + self.indentation.0 + self.indentation.2
			},
			None => 0,
		}
		// self.text_ideal.unwrap().div_ceil(self.assigned_size.get(Dimension::Width).unwrap()) * 20 // TWENTY IS THE PREDENT WIDTH OF A CHARACTER NOT SOME SPECIAL VALUE.
	}

	fn get_minimum_size(&self, dimension: Dimension) -> u64 {
		match self.size.get(dimension) {
			Size::Fixed( size ) => size,
			Size::Fit { minimum, maximum: _ } => minimum.unwrap_or(0),
			Size::Grow { minimum, maximum: _ } => minimum.unwrap_or(0),
			Size::FitText { minimum, maximum: _ } => match dimension {
				Dimension::Width => if minimum.unwrap_or(0) > self.text_minimum.unwrap_or(0) { minimum.unwrap_or(0) } else { self.text_minimum.unwrap_or(0) },
				Dimension::Height => minimum.unwrap_or(0)
			},
		}
	}

	fn get_maximum_size(&self, dimension: Dimension) -> Option<u64> {
		match self.size.get(dimension) {
			Size::Fixed( size ) => Some(size),
			Size::Fit { minimum: _, maximum } => maximum,
			Size::Grow { minimum: _, maximum } => maximum,
			Size::FitText { minimum: _, maximum } => maximum,
		}
	}

	fn calculate_fit_size_along_axis(&mut self, dimension: Dimension) -> u64 {
		let mut  size: u64 = 0;
		let number_of_children: u64 = self.children.len() as u64;
		size += self.child_gaps * (if number_of_children > 1 {number_of_children - 1} else {0});
		for child in self.children.iter_mut() {
			size += child.calculate_fit_size(dimension);

		}
		size
	}

	fn calculate_fit_size_across_axis(&mut self, dimension: Dimension) -> u64 {
		let mut size: u64 = 0;
		for child in self.children.iter_mut() {
			let child_size = child.calculate_fit_size(dimension);
			if child_size > size {
				size = child_size
			};
		}
		size
	}

	pub fn calculate_fit_size(&mut self, dimension: Dimension) -> u64 {
		let (top, right, bottom, left) = self.indentation;

		let mut size = match (dimension, self.direction) {
			(Dimension::Width, Direction::Horizontal) => self.calculate_fit_size_along_axis(dimension) + right + left,
			(Dimension::Width, Direction::Vertical) => self.calculate_fit_size_across_axis(dimension) + right + left,
			(Dimension::Height, Direction::Horizontal) => self.calculate_fit_size_across_axis(dimension) + top + bottom,
			(Dimension::Height, Direction::Vertical) => self.calculate_fit_size_along_axis(dimension) + top + bottom,
		};

		if let Size::FitText { minimum: _, maximum: _ } = self.size.get(dimension) {
			if let Dimension::Width = dimension {
				if size < self.text_ideal.unwrap() {
					size = self.text_ideal.unwrap();
				}
			}
			if let Dimension::Height = dimension {
				let text_height = self.calculate_text_height();
				if size < text_height {
					size = text_height
				}
			}
		}

		self.calculated_fit_size.set(dimension, Some(size));

		let minumum = self.get_minimum_size(dimension);

		if size < minumum {
			size = minumum;
		};

		if let Some(maximum) = self.get_maximum_size(dimension) {
			if size > maximum {
				size = maximum
			}
		}

		self.assigned_size.set(dimension, Some(size));

		size
	}
	
	fn calculate_final_size_along_axis(&mut self, dimension: Dimension) -> Result<(), SizeError> {
		if self.assigned_size.get(dimension).unwrap() > self.calculated_fit_size.get(dimension).unwrap() {
			let mut growable_children:Vec<&mut Element<UserMessage>> = Vec::new();
			for child in self.children.iter_mut() {
				match child.size.get(dimension) {
					Size::Fixed( _ ) => continue,
					Size::Fit { minimum: _, maximum: _ } => continue,
					Size::FitText { minimum: _, maximum: _ } => continue,
					Size::Grow { minimum: _, maximum: _ } => growable_children.push(child),
				};
			};

			loop {
				growable_children.retain(|growable_child| {
					if let Some(maximum) = growable_child.get_maximum_size(dimension) {
						if growable_child.assigned_size.get(dimension).unwrap() >= maximum {
							false // Remove if child's size is already at the maximum.
						} else { true }
					} else { true }
				});

				if growable_children.len() < 1 { break; };

				let mut smallest: u64 = u64::MAX;
				let mut second_smallest: u64 = u64::MAX;

				for growable_child in growable_children.iter() {
					let assigned_size = growable_child.assigned_size.get(dimension).unwrap();
					if assigned_size < smallest {
						second_smallest = smallest;
						smallest = assigned_size;
					};
					if assigned_size < second_smallest && assigned_size > smallest {
						second_smallest = assigned_size;
					};

					if let Some(maximum) = growable_child.get_maximum_size(dimension) {
						if maximum < second_smallest {
							second_smallest = maximum;
						}
						if maximum < smallest {
							panic!("Maximum is less than Assigned Size; calculate_final_size_size_axis; element id = {:?}; dimension = {:?}", self.id, dimension)
						}
					}
				};

				let mut children_to_grow: Vec<&mut &mut Element<UserMessage>> = Vec::new();
				for growable_child in growable_children.iter_mut() {
					if growable_child.assigned_size.get(dimension).unwrap() == smallest {
						children_to_grow.push(growable_child);
					};
				};

				let available_growth = self.assigned_size.get(dimension).unwrap() - self.calculated_fit_size.get(dimension).unwrap();
				
				if available_growth < children_to_grow.len() as u64 {
					let number_of_excess_children = children_to_grow.len() as u64 - available_growth;
					for _ in 0..number_of_excess_children {
						let _ = children_to_grow.pop();
					};
				}; // VERY SKETCHY MIGHT NOT WORK.

				let available_growth_per_child=  available_growth / children_to_grow.len() as u64; // DID NOT WORK SHOULD FIX IT

				let ammount_children_can_be_grown = second_smallest - smallest;
				let ammount_to_grow_children_by = if available_growth_per_child < ammount_children_can_be_grown { available_growth_per_child } else { ammount_children_can_be_grown };

				for child_to_grow in children_to_grow.iter_mut() {
					child_to_grow.assigned_size.set(dimension, Some(child_to_grow.assigned_size.get(dimension).unwrap() + ammount_to_grow_children_by));
				};

				self.calculated_fit_size.set(dimension, Some(self.calculated_fit_size.get(dimension).unwrap() + ammount_to_grow_children_by * children_to_grow.len() as u64));

				if self.assigned_size.get(dimension).unwrap() <= self.calculated_fit_size.get(dimension).unwrap() { break; };
			};

		}

		if self.assigned_size.get(dimension).unwrap() < self.calculated_fit_size.get(dimension).unwrap() {
			let mut shrinkable_children: Vec<&mut Element<UserMessage>> = Vec::new();
			for child in self.children.iter_mut() {
				match child.size.get(dimension) {
					Size::Fixed( _ ) => continue,
					Size::Grow { minimum: _, maximum: _ } => continue,
					Size::Fit { minimum: _, maximum: _ } => shrinkable_children.push(child),
					Size::FitText { minimum: _, maximum: _ } => shrinkable_children.push(child),
				};
			};

			loop {
				shrinkable_children.retain(|shrinkable_child| {
					let minimum = shrinkable_child.get_minimum_size(dimension);
					if shrinkable_child.assigned_size.get(dimension).unwrap() <= minimum {
						false // Remove if child's size is already at the minimum.
					} else { true }
				});

				if shrinkable_children.len() < 1 {
					if let Dimension::Width = dimension {
						if let Size::FitText { minimum: _, maximum: _ } = self.size.get(dimension) {
							if self.assigned_size.get(dimension).unwrap() > self.get_minimum_size(dimension) {
								self.calculated_fit_size.set(dimension, self.assigned_size.get(dimension));
								return Ok(());
							}
							
						};

					};
					
					return Err(SizeError::CantShrinkChildren(1))
				};

				let mut largest: u64 = 0;
				let mut second_largest: u64 = 0;

				for shrinkable_child in shrinkable_children.iter() {
					let assigned_size = shrinkable_child.assigned_size.get(dimension).unwrap();
					if assigned_size > largest {
						second_largest = largest;
						largest = assigned_size;
					};
					if assigned_size > second_largest && assigned_size < largest {
						second_largest = assigned_size;
					};

					let minimum = shrinkable_child.get_minimum_size(dimension);
					if minimum > second_largest {
						second_largest = minimum;
					}
					if minimum > largest {
						panic!("Minimum is greater than Assigned Size; calculate_final_size_along_axis; element id = {:?}; dimension = {:?}", self.id, dimension)
					}
				};

				let mut children_to_shrink: Vec<&mut &mut Element<UserMessage>> = Vec::new();
				for shrinkable_child in shrinkable_children.iter_mut() {
					if shrinkable_child.assigned_size.get(dimension).unwrap() == largest {
						children_to_shrink.push(shrinkable_child);
					};
				};

				let available_shrinkage = self.calculated_fit_size.get(dimension).unwrap() - self.assigned_size.get(dimension).unwrap();
				let available_shrinkage_per_child = available_shrinkage / children_to_shrink.len() as u64;
				let ammount_children_can_be_shrunk = largest - second_largest;
				let ammount_to_shrink_children_by = if available_shrinkage_per_child < ammount_children_can_be_shrunk { available_shrinkage_per_child } else { ammount_children_can_be_shrunk };

				if available_shrinkage_per_child == 0 {
					let number_of_excess_children = children_to_shrink.len() as u64 - available_shrinkage;
					for _ in 0..number_of_excess_children {
						let _ = children_to_shrink.pop();
					};
				}; // VERY SKETCHY MIGHT NOT WORK.

				for child_to_shrink in children_to_shrink.iter_mut() {
					child_to_shrink.assigned_size.set(dimension, Some(child_to_shrink.assigned_size.get(dimension).unwrap() - ammount_to_shrink_children_by));
				};

				self.calculated_fit_size.set(dimension, Some(self.calculated_fit_size.get(dimension).unwrap() - ammount_to_shrink_children_by * children_to_shrink.len() as u64));

				if self.assigned_size.get(dimension).unwrap() >= self.calculated_fit_size.get(dimension).unwrap() { break; }; 
			};
		}

		Ok(())
	}

	fn calculate_final_size_across_axis(&mut self, dimension: Dimension) -> Result<(), SizeError> {
		let (top, right, bottom, left) = self.indentation;
		let available_size = match dimension {
			Dimension::Width => self.assigned_size.get(dimension).unwrap() - right -left,
			Dimension::Height => self.assigned_size.get(dimension).unwrap() - top - bottom,
		};
		for child in self.children.iter_mut() {
			if child.assigned_size.get(dimension).unwrap() < available_size {
				match child.size.get(dimension) {
					Size::Fixed( _ ) => continue,
					Size::Fit{ minimum: _, maximum: _ } => continue,
					Size::FitText { minimum: _, maximum: _ } => continue,
					Size::Grow { minimum: _, maximum: _ } => {
						child.assigned_size.set(dimension, if let Some(maximum) = child.get_maximum_size(dimension) {
							if maximum < available_size { Some(maximum) } else { Some(available_size) }
						} else { Some(available_size) });
					},
				};
			};
			if child.assigned_size.get(dimension).unwrap() > available_size {
				match child.size.get(dimension) {
					Size::Fixed( _ ) => return Err(SizeError::CantShrinkChildren(2)),
					Size::Grow{ minimum: _, maximum: _ } => return  Err(SizeError::CantShrinkChildren(3)),
					Size::Fit { minimum: _, maximum: _ } => {
						child.assigned_size.set(dimension, if child.get_minimum_size(dimension) > available_size { return Err(SizeError::CantShrinkChildren(4)) } else { Some(available_size) });
					},
					Size::FitText { minimum: _, maximum: _ } => {
						child.assigned_size.set(dimension, if child.get_minimum_size(dimension) > available_size { return Err(SizeError::CantShrinkChildren(5)) } else { Some(available_size) });

					},
				};
			};
		}

		Ok(())
	}

	pub fn calculate_final_size(&mut self, dimension: Dimension) -> Result<(), SizeError> {
		match (dimension, self.direction) {
			(Dimension::Width, Direction::Horizontal) => self.calculate_final_size_along_axis(dimension)?,
			(Dimension::Width, Direction::Vertical) => self.calculate_final_size_across_axis(dimension)?,
			(Dimension::Height, Direction::Horizontal) => self.calculate_final_size_across_axis(dimension)?,
			(Dimension::Height, Direction::Vertical) => self.calculate_final_size_along_axis(dimension)?,
		};

		for child in self.children.iter_mut() {
			child.calculate_final_size(dimension)?;
		}

		Ok(())
	}

	fn calculate_children_position_dimensioned(&mut self, position: Position, dimension: Dimension) {
		let (top, right, bottom, left) = self.indentation;

		let along_start_indentation = match dimension { Dimension::Width => right, Dimension::Height => top };
		let along_end_indentation = match dimension { Dimension::Width => left, Dimension::Height => bottom };
		let across_start_indentation = match dimension { Dimension::Width => top, Dimension::Height => right };
		let across_end_indentation = match dimension { Dimension::Width => bottom, Dimension::Height => left };

		let along = position.get(dimension.into()).unwrap();
		let across = position.get(dimension.opposite().into()).unwrap();
		let excess_along = self.assigned_size.get(dimension).unwrap() - self.calculated_fit_size.get(dimension).unwrap();
		let mut cummulative_along = match self.alignment.get(dimension.into()) {
			Alignments::Start => along_start_indentation,
			Alignments::Centre => along_start_indentation + excess_along / 2,
			Alignments::End => along_start_indentation + excess_along,
		};
		for child in self.children.iter_mut() {
			let child_excess_across = self.assigned_size.get(dimension.opposite()).unwrap() - child.assigned_size.get(dimension.opposite()).unwrap() - across_start_indentation - across_end_indentation;
			let mut child_position = Position::none();
			child_position.set(dimension.into(), Some(cummulative_along + along));
			child_position.set(dimension.opposite().into(), match self.alignment.get(dimension.opposite().into()) {
				Alignments::Start => Some(across_start_indentation + across),
				Alignments::Centre => Some(across_start_indentation + child_excess_across / 2 + across),
				Alignments::End => Some(across_start_indentation + child_excess_across + across),
			});
			cummulative_along += child.assigned_size.get(dimension).unwrap() + self.child_gaps;
			child.calculate_children_position(child_position);
		}
	}

	pub fn calculate_children_position(&mut self, position: Position) {
		self.position = position;
		self.calculate_children_position_dimensioned(position, self.direction.into());

	}

	pub fn to_rectangles(&self, element_rectangles: &mut Vec<ElementRectangle>) {
		element_rectangles.push(
			ElementRectangle {
				position: self.position,
				size: self.assigned_size.unwrap_contents(),
				colour: self.colour,
			}
		);

		for child in self.children.iter() {
			child.to_rectangles(element_rectangles);
		}
	}

	pub fn get_on_click(&self, x: Pixels<f64>, y: Pixels<f64>) -> Option<UserMessage> {
		let mut on_click: Option<UserMessage> = None;
		if let Some(on_click_self) = &self.on_click {
			on_click = Some(on_click_self.clone());
		}

		for child in self.children.iter() {
			let x_difference = x.value - child.position.x.unwrap() as f64;
			let y_difference = y.value - child.position.y.unwrap() as f64;

			if (0.0 <= x_difference) && (x_difference <= child.assigned_size.width.unwrap() as f64) {
				if (0.0 <= y_difference) && (y_difference <= child.assigned_size.height.unwrap() as f64) {
					if let Some(on_click_child) = child.get_on_click(x, y) {
						on_click = Some(on_click_child);
					}
				}
			}
		}

		on_click
	}
}

// FOR TESTING
impl<T> Element<T> {
	pub fn print_tree(&self) {
		println!("Instance {{ position: [{}, {}], size: [{}, {}], color: [{}f32, {}f32, {}f32]}},", self.position.get(Axis::X).unwrap(), self.position.get(Axis::Y).unwrap(), self.assigned_size.get(Dimension::Width).unwrap(), self.assigned_size.get(Dimension::Height).unwrap(), self.colour.r, self.colour.g, self.colour.b);
		for child in self.children.iter() {
			child.print_tree();
		}
	}
}