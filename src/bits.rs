//! Fixed-width bitsets over a support 0..n−1, n ≤ 64 W.
//!
//! A set is W machine words; subset, meet and difference are word operations. Algorithms are
//! generic in W, and `with_width!` picks the narrowest width for a support (the QBL has 981
//! suppliers, so W ≤ 16).

use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bits<const W: usize>(pub [u64; W]);

impl<const W: usize> Default for Bits<W> {
    fn default() -> Self {
        Bits([0; W])
    }
}

impl<const W: usize> Bits<W> {
    pub const EMPTY: Self = Bits([0; W]);

    pub fn unit(i: usize) -> Self {
        let mut b = Self::EMPTY;
        b.insert(i);
        b
    }

    /// {0, …, n−1}.
    pub fn full(n: usize) -> Self {
        let mut b = Self::EMPTY;
        for (j, w) in b.0.iter_mut().enumerate() {
            let lo = 64 * j;
            *w = if n >= lo + 64 { !0 } else if n > lo { (1u64 << (n - lo)) - 1 } else { 0 };
        }
        b
    }

    pub fn of<I: IntoIterator<Item = usize>>(it: I) -> Self {
        let mut b = Self::EMPTY;
        for i in it {
            b.insert(i);
        }
        b
    }

    pub fn has(&self, i: usize) -> bool {
        self.0[i / 64] >> (i % 64) & 1 == 1
    }

    pub fn insert(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }

    pub fn remove(&mut self, i: usize) {
        self.0[i / 64] &= !(1 << (i % 64));
    }

    pub fn with(mut self, i: usize) -> Self {
        self.insert(i);
        self
    }

    pub fn without(mut self, i: usize) -> Self {
        self.remove(i);
        self
    }

    pub fn and(&self, o: &Self) -> Self {
        let mut r = *self;
        for j in 0..W {
            r.0[j] &= o.0[j];
        }
        r
    }

    pub fn or(&self, o: &Self) -> Self {
        let mut r = *self;
        for j in 0..W {
            r.0[j] |= o.0[j];
        }
        r
    }

    /// self \ o.
    pub fn minus(&self, o: &Self) -> Self {
        let mut r = *self;
        for j in 0..W {
            r.0[j] &= !o.0[j];
        }
        r
    }

    pub fn xor(&self, o: &Self) -> Self {
        let mut r = *self;
        for j in 0..W {
            r.0[j] ^= o.0[j];
        }
        r
    }

    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }

    pub fn meets(&self, o: &Self) -> bool {
        (0..W).any(|j| self.0[j] & o.0[j] != 0)
    }

    /// self ⊆ o.
    pub fn sub(&self, o: &Self) -> bool {
        (0..W).all(|j| self.0[j] & !o.0[j] == 0)
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// |self ∩ o|.
    pub fn meet_len(&self, o: &Self) -> usize {
        (0..W).map(|j| (self.0[j] & o.0[j]).count_ones() as usize).sum()
    }

    pub fn lowest(&self) -> Option<usize> {
        (0..W).find(|&j| self.0[j] != 0).map(|j| 64 * j + self.0[j].trailing_zeros() as usize)
    }

    pub fn highest(&self) -> Option<usize> {
        (0..W).rev().find(|&j| self.0[j] != 0).map(|j| 64 * j + 63 - self.0[j].leading_zeros() as usize)
    }

    /// Members in ascending order (fka `verts`).
    pub fn iter(&self) -> Members<W> {
        Members { b: *self, j: 0 }
    }

    pub fn to_vec(&self) -> Vec<usize> {
        self.iter().collect()
    }

    /// Order of the ascending member tuples, the canonical term order of the thesis
    /// (`Hypergraph.from_bitsets` sorts by `verts`). With i the least element of a △ b:
    /// the set holding i is smaller iff the other set has an element above i.
    pub fn lex(&self, o: &Self) -> Ordering {
        let Some(i) = self.xor(o).lowest() else { return Ordering::Equal };
        let (a_has, other) = if self.has(i) { (true, o) } else { (false, self) };
        let above = other.highest().is_some_and(|h| h > i);
        match (a_has, above) {
            (true, true) | (false, false) => Ordering::Less,
            _ => Ordering::Greater,
        }
    }

    /// Order of Python ints: the highest differing bit decides.
    pub fn num(&self, o: &Self) -> Ordering {
        for j in (0..W).rev() {
            match self.0[j].cmp(&o.0[j]) {
                Ordering::Equal => continue,
                c => return c,
            }
        }
        Ordering::Equal
    }
}

pub struct Members<const W: usize> {
    b: Bits<W>,
    j: usize,
}

impl<const W: usize> Iterator for Members<W> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        while self.j < W {
            let w = self.b.0[self.j];
            if w != 0 {
                self.b.0[self.j] = w & (w - 1);
                return Some(64 * self.j + w.trailing_zeros() as usize);
            }
            self.j += 1;
        }
        None
    }
}

impl<const W: usize> std::fmt::Debug for Bits<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// Runs `$body` with `$W` bound to the narrowest word count holding `$n` elements.
#[macro_export]
macro_rules! with_width {
    ($n:expr, $W:ident => $body:expr) => {{
        match ($n as usize).div_ceil(64) {
            0 | 1 => {
                const $W: usize = 1;
                $body
            }
            2 => {
                const $W: usize = 2;
                $body
            }
            3 | 4 => {
                const $W: usize = 4;
                $body
            }
            5..=8 => {
                const $W: usize = 8;
                $body
            }
            9..=16 => {
                const $W: usize = 16;
                $body
            }
            _ => panic!("a support of {} exceeds 1024 suppliers", $n),
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(v: &[usize]) -> Bits<2> {
        Bits::of(v.iter().copied())
    }

    #[test]
    fn lex_is_tuple_order() {
        let sets: Vec<Vec<usize>> = vec![vec![], vec![0], vec![0, 1], vec![0, 5], vec![1], vec![1, 70],
                                         vec![4, 5], vec![64], vec![64, 65]];
        for x in &sets {
            for y in &sets {
                assert_eq!(b(x).lex(&b(y)), x.cmp(y), "{x:?} {y:?}");
            }
        }
    }

    #[test]
    fn full_and_members() {
        assert_eq!(Bits::<2>::full(70).len(), 70);
        assert_eq!(b(&[3, 64, 100]).to_vec(), vec![3, 64, 100]);
        assert_eq!(b(&[3, 64, 100]).highest(), Some(100));
    }
}
