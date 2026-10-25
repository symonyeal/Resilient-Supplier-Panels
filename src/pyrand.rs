//! Python's `random.Random` (CPython 3.14): MT19937 seeded by `init_by_array` on the 32-bit
//! words of |seed|, with `_randbelow`, `choice`, `shuffle` and `sample` drawing in the same
//! order, so a seeded basket suite reproduces the Python one exactly.

const N: usize = 624;
const M: usize = 397;

pub struct Random {
    mt: [u32; N],
    i: usize,
}

impl Random {
    pub fn new(seed: u64) -> Self {
        let mut key: Vec<u32> = Vec::new();
        let mut s = seed;
        while s > 0 {
            key.push(s as u32);
            s >>= 32;
        }
        if key.is_empty() {
            key.push(0);
        }
        let mut r = Random { mt: [0; N], i: N };
        r.init_genrand(19650218);
        let (mut i, mut j) = (1usize, 0usize);
        for _ in 0..N.max(key.len()) {
            let x = r.mt[i - 1] ^ (r.mt[i - 1] >> 30);
            r.mt[i] = (r.mt[i] ^ x.wrapping_mul(1664525)).wrapping_add(key[j]).wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                r.mt[0] = r.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..N - 1 {
            let x = r.mt[i - 1] ^ (r.mt[i - 1] >> 30);
            r.mt[i] = (r.mt[i] ^ x.wrapping_mul(1566083941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                r.mt[0] = r.mt[N - 1];
                i = 1;
            }
        }
        r.mt[0] = 0x8000_0000;
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            let x = self.mt[i - 1] ^ (self.mt[i - 1] >> 30);
            self.mt[i] = 1812433253u32.wrapping_mul(x).wrapping_add(i as u32);
        }
        self.i = N;
    }

    fn genrand(&mut self) -> u32 {
        if self.i >= N {
            for k in 0..N {
                let y = (self.mt[k] & 0x8000_0000) | (self.mt[(k + 1) % N] & 0x7fff_ffff);
                let mag = if y & 1 == 1 { 0x9908_b0df } else { 0 };
                self.mt[k] = self.mt[(k + M) % N] ^ (y >> 1) ^ mag;
            }
            self.i = 0;
        }
        let mut y = self.mt[self.i];
        self.i += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// k ≤ 32 random bits.
    pub fn getrandbits(&mut self, k: u32) -> u32 {
        if k == 0 { 0 } else { self.genrand() >> (32 - k) }
    }

    /// Uniform on 0..n, n > 0, by rejection on n.bit_length() bits.
    pub fn below(&mut self, n: usize) -> usize {
        let k = usize::BITS - n.leading_zeros();
        loop {
            let r = self.getrandbits(k) as usize;
            if r < n {
                return r;
            }
        }
    }

    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        a + self.below((b - a + 1) as usize) as i64
    }

    pub fn choice<'a, T>(&mut self, s: &'a [T]) -> &'a T {
        &s[self.below(s.len())]
    }

    pub fn shuffle<T>(&mut self, x: &mut [T]) {
        for i in (1..x.len()).rev() {
            let j = self.below(i + 1);
            x.swap(i, j);
        }
    }

    /// k distinct members in draw order: a shrinking pool when n is small against the
    /// table a k-element set would need, otherwise rejection of repeats.
    pub fn sample<T: Clone>(&mut self, pop: &[T], k: usize) -> Vec<T> {
        let n = pop.len();
        assert!(k <= n, "sample larger than population");
        let mut setsize = 21usize;
        if k > 5 {
            setsize += 4usize.pow((((k * 3) as f64).ln() / 4f64.ln()).ceil() as u32);
        }
        if n <= setsize {
            let mut pool: Vec<T> = pop.to_vec();
            (0..k).map(|i| {
                let j = self.below(n - i);
                let x = pool[j].clone();
                pool[j] = pool[n - i - 1].clone();
                x
            }).collect()
        } else {
            let mut seen = std::collections::HashSet::new();
            (0..k).map(|_| {
                let mut j = self.below(n);
                while !seen.insert(j) {
                    j = self.below(n);
                }
                pop[j].clone()
            }).collect()
        }
    }
}
