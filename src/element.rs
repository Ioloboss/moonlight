use std::sync::{Arc, Mutex};

use mircalla_types::{units::Pixels, vectors::{Axis, Colour, Dimension, Direction, Position, Size}};
use tapestry::font::{Font, font_renderer::TextBox};

use crate::renderer::ElementRectangle;

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
pub enum Sizing {
	Fixed( Pixels<f32> ),
	Fit{ minimum: Option<Pixels<f32>>, maximum: Option<Pixels<f32>> },
	Grow{ minimum: Option<Pixels<f32>>, maximum: Option<Pixels<f32>> },
	FitText { minimum: Option<Pixels<f32>>, maximum: Option<Pixels<f32>> },
}

#[derive(Clone, Copy, Debug)]
pub enum SizingError {
	CantShrinkChildren(u64),
	CantGrow(),
}

pub struct Indentation {
	top: Pixels<f32>,
	right: Pixels<f32>,
	bottom: Pixels<f32>,
	left: Pixels<f32>,
}

impl From<(Pixels<f32>, Pixels<f32>, Pixels<f32>, Pixels<f32>)> for Indentation {
	fn from(value: (Pixels<f32>, Pixels<f32>, Pixels<f32>, Pixels<f32>)) -> Self {
		Self {
			top: value.0,
			right: value.1,
			bottom: value.2,
			left: value.3,
		}
	}
}

pub struct Element<UserMessage> {
	direction: Direction,
	sizing: Size<Sizing>,
	colour: Colour,
	children: Vec<Element<UserMessage>>,
	// text: Option<Arc<Mutex<String>>>,
	text: Option<TextBox>,
	on_click: Option<UserMessage>,
	child_gaps: Pixels<f32>,
	indentation: Indentation,
	alignment: Alignment,
	id: Option<u64>,
	// Working values changed by layout engine.
	calculated_fit_size: Size<Option<Pixels<f32>>>,
	assigned_size: Size<Option<Pixels<f32>>>,
	position: Position<Option<Pixels<f32>>>,
	text_minimum: Option<Pixels<f32>>,
	text_ideal: Option<Pixels<f32>>,
}

impl<UserMessage: Clone> Element<UserMessage> {
	pub fn new(direction: Direction, sizing: Size<Sizing>, colour: Colour, children: Vec<Element<UserMessage>>) -> Self {
		Element {
			direction,
			sizing,
			colour,
			children,
			text: None,
			on_click: None,
			child_gaps: 0.0.into(),
			indentation: (0.0.into(), 0.0.into(), 0.0.into(), 0.0.into()).into(),
			alignment: Alignment {x: Alignments::Start, y: Alignments::Start},
			id: None,
			calculated_fit_size: Size::none(),
			assigned_size: Size::none(),
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

	pub fn child_gaps(mut self, child_gaps: Pixels<f32>) -> Self {
		self.child_gaps = child_gaps;
		self
	}

	pub fn indentation(mut self, top: Pixels<f32>, right: Pixels<f32>, bottom: Pixels<f32>, left: Pixels<f32>) -> Self {
		self.indentation = (top, right, bottom, left).into();
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
				self.text_minimum = Some(20.0.into()); // TWENTY IS THE PREDEND WIDTH OF A CHARACTER NOT SOME SPECIAL VALUE.
				self.text_ideal = Some(text.get_ideal_width() + self.indentation.right + self.indentation.left);
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

	pub fn collect_text_boxes(&self, screen_size: Size<Pixels<f32>>) -> Vec<TextBox> {
		let mut text_boxes: Vec<TextBox> = Vec::new();

		match &self.text {
			Some(text) => {
				let text_box = TextBox {
					font: Arc::clone(&text.font),
					text: Arc::clone(&text.text),
					pixels_per_em: text.pixels_per_em,
					position: ((self.position.x.unwrap() + self.indentation.left), ((screen_size.height - self.position.y.unwrap() - self.assigned_size.height.unwrap() + self.indentation.bottom) + text.font.typographic_descender.to_pixels(text.get_pixels_per_font_unit()))).into(),
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

	fn calculate_text_height(&self) -> Pixels<f32> {
		match &self.text {
			Some(text) => {
				text.get_height() + self.indentation.top + self.indentation.bottom
			},
			None => 0.0.into(),
		}
		// self.text_ideal.unwrap().div_ceil(self.assigned_size.get(Dimension::Width).unwrap()) * 20 // TWENTY IS THE PREDENT WIDTH OF A CHARACTER NOT SOME SPECIAL VALUE.
	}

	fn get_minimum_size(&self, dimension: Dimension) -> Pixels<f32> {
		match self.sizing.get(dimension) {
			Sizing::Fixed( size ) => size,
			Sizing::Fit { minimum, maximum: _ } => minimum.unwrap_or(0.0.into()),
			Sizing::Grow { minimum, maximum: _ } => minimum.unwrap_or(0.0.into()),
			Sizing::FitText { minimum, maximum: _ } => match dimension {
				Dimension::Width => if minimum.unwrap_or(0.0.into()) > self.text_minimum.unwrap_or(0.0.into()) { minimum.unwrap_or(0.0.into()) } else { self.text_minimum.unwrap_or(0.0.into()) },
				Dimension::Height => minimum.unwrap_or(0.0.into())
			},
		}
	}

	fn get_maximum_size(&self, dimension: Dimension) -> Option<Pixels<f32>> {
		match self.sizing.get(dimension) {
			Sizing::Fixed( size ) => Some(size),
			Sizing::Fit { minimum: _, maximum } => maximum,
			Sizing::Grow { minimum: _, maximum } => maximum,
			Sizing::FitText { minimum: _, maximum } => maximum,
		}
	}

	fn calculate_fit_size_along_axis(&mut self, dimension: Dimension) -> Pixels<f32> {
		let mut  size = 0.0.into();
		let number_of_children = self.children.len();
		size += self.child_gaps * (if number_of_children > 1 {number_of_children - 1} else {0});
		for child in self.children.iter_mut() {
			size += child.calculate_fit_size(dimension);

		}
		size
	}

	fn calculate_fit_size_across_axis(&mut self, dimension: Dimension) -> Pixels<f32> {
		let mut size = 0.0.into();
		for child in self.children.iter_mut() {
			let child_size = child.calculate_fit_size(dimension);
			if child_size > size {
				size = child_size
			};
		}
		size
	}

	pub fn calculate_fit_size(&mut self, dimension: Dimension) -> Pixels<f32> {
		let Indentation {top, right, bottom, left} = self.indentation;

		let mut size = match (dimension, self.direction) {
			(Dimension::Width, Direction::Horizontal) => self.calculate_fit_size_along_axis(dimension) + right + left,
			(Dimension::Width, Direction::Vertical) => self.calculate_fit_size_across_axis(dimension) + right + left,
			(Dimension::Height, Direction::Horizontal) => self.calculate_fit_size_across_axis(dimension) + top + bottom,
			(Dimension::Height, Direction::Vertical) => self.calculate_fit_size_along_axis(dimension) + top + bottom,
		};

		if let Sizing::FitText { minimum: _, maximum: _ } = self.sizing.get(dimension) {
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
	
	fn calculate_final_size_along_axis(&mut self, dimension: Dimension) -> Result<(), SizingError> {
		if self.assigned_size.get(dimension).unwrap() > self.calculated_fit_size.get(dimension).unwrap() {
			let mut growable_children:Vec<&mut Element<UserMessage>> = Vec::new();
			for child in self.children.iter_mut() {
				match child.sizing.get(dimension) {
					Sizing::Fixed( _ ) => continue,
					Sizing::Fit { minimum: _, maximum: _ } => continue,
					Sizing::FitText { minimum: _, maximum: _ } => continue,
					Sizing::Grow { minimum: _, maximum: _ } => growable_children.push(child),
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

				let mut smallest = f32::MAX.into();
				let mut second_smallest = f32::MAX.into();

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
				
				if (available_growth.value as usize) < children_to_grow.len() {
					let number_of_excess_children = children_to_grow.len() - available_growth.value as usize;
					for _ in 0..number_of_excess_children {
						let _ = children_to_grow.pop();
					};
				}; // VERY SKETCHY MIGHT NOT WORK.

				let available_growth_per_child=  available_growth / children_to_grow.len(); // DID NOT WORK SHOULD FIX IT

				let ammount_children_can_be_grown = second_smallest - smallest;
				let ammount_to_grow_children_by = if available_growth_per_child < ammount_children_can_be_grown { available_growth_per_child } else { ammount_children_can_be_grown };

				for child_to_grow in children_to_grow.iter_mut() {
					child_to_grow.assigned_size.set(dimension, Some(child_to_grow.assigned_size.get(dimension).unwrap() + ammount_to_grow_children_by));
				};

				self.calculated_fit_size.set(dimension, Some(self.calculated_fit_size.get(dimension).unwrap() + ammount_to_grow_children_by * children_to_grow.len()));

				if self.assigned_size.get(dimension).unwrap() <= self.calculated_fit_size.get(dimension).unwrap() { break; };
			};

		}

		if self.assigned_size.get(dimension).unwrap() < self.calculated_fit_size.get(dimension).unwrap() {
			let mut shrinkable_children: Vec<&mut Element<UserMessage>> = Vec::new();
			for child in self.children.iter_mut() {
				match child.sizing.get(dimension) {
					Sizing::Fixed( _ ) => continue,
					Sizing::Grow { minimum: _, maximum: _ } => continue,
					Sizing::Fit { minimum: _, maximum: _ } => shrinkable_children.push(child),
					Sizing::FitText { minimum: _, maximum: _ } => shrinkable_children.push(child),
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
						if let Sizing::FitText { minimum: _, maximum: _ } = self.sizing.get(dimension) {
							if self.assigned_size.get(dimension).unwrap() > self.get_minimum_size(dimension) {
								self.calculated_fit_size.set(dimension, self.assigned_size.get(dimension));
								return Ok(());
							}
							
						};

					};
					
					return Err(SizingError::CantShrinkChildren(1))
				};

				let mut largest = 0.0.into();
				let mut second_largest = 0.0.into();

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
				
				if (available_shrinkage.value as usize) < children_to_shrink.len() {
					let number_of_excess_children = children_to_shrink.len() - available_shrinkage.value as usize;
					for _ in 0..number_of_excess_children {
						let _ = children_to_shrink.pop();
					};
				}; // VERY SKETCHY MIGHT NOT WORK.

				let available_shrinkage_per_child = available_shrinkage / children_to_shrink.len();
				
				let ammount_children_can_be_shrunk = largest - second_largest;
				let ammount_to_shrink_children_by = if available_shrinkage_per_child < ammount_children_can_be_shrunk { available_shrinkage_per_child } else { ammount_children_can_be_shrunk };

				for child_to_shrink in children_to_shrink.iter_mut() {
					child_to_shrink.assigned_size.set(dimension, Some(child_to_shrink.assigned_size.get(dimension).unwrap() - ammount_to_shrink_children_by));
				};

				self.calculated_fit_size.set(dimension, Some(self.calculated_fit_size.get(dimension).unwrap() - ammount_to_shrink_children_by * children_to_shrink.len()));

				if self.assigned_size.get(dimension).unwrap() >= self.calculated_fit_size.get(dimension).unwrap() { break; }; 
			};
		}

		Ok(())
	}

	fn calculate_final_size_across_axis(&mut self, dimension: Dimension) -> Result<(), SizingError> {
		let Indentation {top, right, bottom, left} = self.indentation;
		let available_size = match dimension {
			Dimension::Width => self.assigned_size.get(dimension).unwrap() - right - left,
			Dimension::Height => self.assigned_size.get(dimension).unwrap() - top - bottom,
		};
		for child in self.children.iter_mut() {
			if child.assigned_size.get(dimension).unwrap() < available_size {
				match child.sizing.get(dimension) {
					Sizing::Fixed( _ ) => continue,
					Sizing::Fit{ minimum: _, maximum: _ } => continue,
					Sizing::FitText { minimum: _, maximum: _ } => continue,
					Sizing::Grow { minimum: _, maximum: _ } => {
						child.assigned_size.set(dimension, if let Some(maximum) = child.get_maximum_size(dimension) {
							if maximum < available_size { Some(maximum) } else { Some(available_size) }
						} else { Some(available_size) });
					},
				};
			};
			if child.assigned_size.get(dimension).unwrap() > available_size {
				match child.sizing.get(dimension) {
					Sizing::Fixed( _ ) => return Err(SizingError::CantShrinkChildren(2)),
					Sizing::Grow{ minimum: _, maximum: _ } => return  Err(SizingError::CantShrinkChildren(3)),
					Sizing::Fit { minimum: _, maximum: _ } => {
						child.assigned_size.set(dimension, if child.get_minimum_size(dimension) > available_size { return Err(SizingError::CantShrinkChildren(4)) } else { Some(available_size) });
					},
					Sizing::FitText { minimum: _, maximum: _ } => {
						child.assigned_size.set(dimension, if child.get_minimum_size(dimension) > available_size { return Err(SizingError::CantShrinkChildren(5)) } else { Some(available_size) });

					},
				};
			};
		}

		Ok(())
	}

	pub fn calculate_final_size(&mut self, dimension: Dimension) -> Result<(), SizingError> {
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

	fn calculate_children_position_dimensioned(&mut self, position: Position<Option<Pixels<f32>>>, dimension: Dimension) {
		let Indentation {top, right, bottom, left} = self.indentation;

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

	pub fn calculate_children_position(&mut self, position: Position<Option<Pixels<f32>>>) {
		self.position = position;
		self.calculate_children_position_dimensioned(position, self.direction.into());

	}

	pub fn to_rectangles(&self, element_rectangles: &mut Vec<ElementRectangle>) {
		element_rectangles.push(
			ElementRectangle {
				position: Position { x: self.position.x.unwrap(), y: self.position.y.unwrap() },
				size: Size { width: self.assigned_size.width.unwrap(), height: self.assigned_size.height.unwrap() },
				colour: self.colour,
			}
		);

		for child in self.children.iter() {
			child.to_rectangles(element_rectangles);
		}
	}

	pub fn get_on_click(&self, position: Position<Pixels<f32>>) -> Option<UserMessage> {
		let mut on_click: Option<UserMessage> = None;
		if let Some(on_click_self) = &self.on_click {
			on_click = Some(on_click_self.clone());
		}

		for child in self.children.iter() {
			let x_difference = position.x - child.position.x.unwrap();
			let y_difference = position.y - child.position.y.unwrap();

			if (Pixels { value: 0.0 } <= x_difference) && (x_difference <= child.assigned_size.width.unwrap()) {
				if (Pixels { value: 0.0 } <= y_difference) && (y_difference <= child.assigned_size.height.unwrap()) {
					if let Some(on_click_child) = child.get_on_click(position) {
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
		println!("Instance {{ position: [{}, {}], size: [{}, {}], color: [{}f32, {}f32, {}f32]}},", self.position.get(Axis::X).unwrap().value, self.position.get(Axis::Y).unwrap().value, self.assigned_size.get(Dimension::Width).unwrap().value, self.assigned_size.get(Dimension::Height).unwrap().value, self.colour.r, self.colour.g, self.colour.b);
		for child in self.children.iter() {
			child.print_tree();
		}
	}
}