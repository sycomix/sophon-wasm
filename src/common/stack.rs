
use std::collections::VecDeque;
use std::fmt;

#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Stack with limit.
#[derive(Debug)]
pub struct StackWithLimit<T> where T: Clone {
	/// Stack values.
	values: VecDeque<T>,
	/// Stack limit (maximal stack len).
	limit: usize,
}

impl<T> StackWithLimit<T> where T: Clone {
	pub fn with_data(data: Vec<T>, limit: usize) -> Self {
		StackWithLimit {
			values: data.into_iter().collect(),
			limit: limit
		}
	}

	pub fn with_limit(limit: usize) -> Self {
		StackWithLimit {
			values: VecDeque::new(),
			limit: limit
		}
	}

	pub fn is_empty(&self) -> bool {
		self.values.is_empty()
	}

	pub fn len(&self) -> usize {
		self.values.len()
	}

	pub fn limit(&self) -> usize {
		self.limit
	}

	pub fn values(&self) -> &VecDeque<T> {
		&self.values
	}

	pub fn top(&self) -> Result<&T, Error> {
		self.values
			.back()
			.ok_or(Error("non-empty stack expected".into()))
	}

	pub fn top_mut(&mut self) -> Result<&mut T, Error> {
		self.values
			.back_mut()
			.ok_or(Error("non-empty stack expected".into()))
	}

	pub fn get(&self, index: usize) -> Result<&T, Error> {
		if index >= self.values.len() {
			return Err(Error(format!("trying to get value at position {} on stack of size {}", index, self.values.len())));
		}

		Ok(self.values.get(self.values.len() - 1 - index).expect("checked couple of lines above"))
	}

	pub fn push(&mut self, value: T) -> Result<(), Error> {
		if self.values.len() >= self.limit {
			return Err(Error(format!("exceeded stack limit {}", self.limit)));
		}

		self.values.push_back(value);
		Ok(())
	}

	pub fn push_penultimate(&mut self, value: T) -> Result<(), Error> {
		if self.values.is_empty() {
			return Err(Error("trying to insert penultimate element into empty stack".into()));
		}
		self.push(value)?;

		let last_index = self.values.len() - 1;
		let penultimate_index = last_index - 1;
		self.values.swap(last_index, penultimate_index);

		Ok(())
	}

	pub fn pop(&mut self) -> Result<T, Error> {
		self.values
			.pop_back()
			.ok_or(Error("non-empty stack expected".into()))
	}

	pub fn resize(&mut self, new_size: usize, dummy: T) {
		debug_assert!(new_size <= self.values.len());
		self.values.resize(new_size, dummy);
	}
}
