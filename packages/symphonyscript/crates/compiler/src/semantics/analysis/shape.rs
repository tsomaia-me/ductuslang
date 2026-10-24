use std::fmt;
use std::rc::Rc;

#[derive(Clone, Default)]
pub(crate) struct Shape(Option<Rc<Dimension>>);

struct Dimension {
    length: u32,
    inner: Shape,
}

impl Shape {
    pub fn scalar() -> Shape {
        Shape(None)
    }

    pub fn is_scalar(&self) -> bool {
        self.0.is_none()
    }

    pub fn outer(&self) -> Option<u32> {
        self.0.as_ref().map(|dimension| dimension.length)
    }

    pub fn inner(&self) -> Shape {
        self.0
            .as_ref()
            .map_or_else(Shape::scalar, |dimension| dimension.inner.clone())
    }

    pub fn wrap(&self, length: u32) -> Shape {
        Shape(Some(Rc::new(Dimension {
            length,
            inner: self.clone(),
        })))
    }

    pub fn from_lengths(lengths: &[u32]) -> Shape {
        lengths
            .iter()
            .fold(Shape::scalar(), |shape, length| shape.wrap(*length))
    }

    pub fn lengths(&self) -> Vec<u32> {
        let mut lengths = Vec::new();
        let mut current = self.0.clone();
        while let Some(dimension) = current {
            lengths.push(dimension.length);
            current = dimension.inner.0.clone();
        }
        lengths.reverse();
        lengths
    }

    pub fn depth(&self) -> usize {
        let mut depth = 0;
        let mut current = self.0.as_ref();
        while let Some(dimension) = current {
            depth += 1;
            current = dimension.inner.0.as_ref();
        }
        depth
    }

    pub fn count(&self) -> usize {
        let mut count: usize = 1;
        let mut current = self.0.as_ref();
        while let Some(dimension) = current {
            count = count.saturating_mul(dimension.length as usize);
            current = dimension.inner.0.as_ref();
        }
        count
    }
}

impl PartialEq for Shape {
    fn eq(&self, other: &Self) -> bool {
        let mut left = self.0.as_ref();
        let mut right = other.0.as_ref();
        loop {
            match (left, right) {
                (None, None) => return true,
                (Some(a), Some(b)) => {
                    if Rc::ptr_eq(a, b) {
                        return true;
                    }
                    if a.length != b.length {
                        return false;
                    }
                    left = a.inner.0.as_ref();
                    right = b.inner.0.as_ref();
                }
                _ => return false,
            }
        }
    }
}

impl Eq for Shape {}

impl fmt::Debug for Shape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Shape(depth {})", self.depth())
    }
}

impl Drop for Shape {
    fn drop(&mut self) {
        let mut current = self.0.take();
        while let Some(dimension) = current {
            current = match Rc::try_unwrap(dimension) {
                Ok(mut dimension) => dimension.inner.0.take(),
                Err(_) => None,
            };
        }
    }
}
