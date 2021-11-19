use core::fmt::{Display, Formatter, Result};

#[derive(Default)]
pub struct Indent(usize);

impl Indent {
    pub fn push(&mut self) {
        self.0 += 1
    }

    pub fn pop(&mut self) {
        self.0 -= 1
    }
}

impl Display for Indent {
    fn fmt(&self, f: &mut Formatter) -> Result {
        for _ in 0..self.0 {
            write!(f, "  ")?;
        }
        Ok(())
    }
}
