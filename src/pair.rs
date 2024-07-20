pub struct Pair<T> {
    left: T,
    right: T,
}

impl<T> std::ops::Index<usize> for Pair<T> {
    type Output = T;
    fn index(&self, i: usize) -> &T {
        if i == 0 { &self.left } else { &self.right }
    }
}

impl<T> std::ops::IndexMut<usize> for Pair<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        if i == 0 { &mut self.left } else { &mut self.right }
    }
}
