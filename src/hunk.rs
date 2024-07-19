type Line = Vec<Vec<u8>>;

pub struct Hunk {
    pub left: Line,
    pub right: Line,
}

impl Hunk {
    pub fn new() -> Self {
        Self{
            left: vec![],
            right: vec![],
        }
    }

    pub fn get(&self, i: usize) -> &Line {
        if i == 0 { &self.left } else { &self.right }
    }

    pub fn get_mut(&mut self, i: usize) -> &mut Line {
        if i == 0 { &mut self.left } else { &mut self.right }
    }

    pub fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
}
