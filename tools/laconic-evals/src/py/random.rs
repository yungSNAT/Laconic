// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

/// Python's `random.Random(seed)` for a non-negative seed below 2**32: MT19937 seeded by
/// `init_by_array([seed])`, with `getrandbits`, `choice` and `shuffle` as CPython draws them.
pub struct Random {
    state: [u32; 624],
    index: usize,
}

impl Random {
    pub fn new(seed: u32) -> Self {
        let mut state = [0u32; 624];
        state[0] = 19_650_218;
        for i in 1..624 {
            let previous = state[i - 1];
            state[i] = 1_812_433_253u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(u32::try_from(i).expect("below 624"));
        }
        // The key is the one word `seed`, so its index j is always 0 and adds nothing.
        let mut i = 1usize;
        for _ in 0..624 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(seed);
            i += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
        }
        for _ in 0..623 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(u32::try_from(i).expect("below 624"));
            i += 1;
            if i >= 624 {
                state[0] = state[623];
                i = 1;
            }
        }
        state[0] = 0x8000_0000;
        Random { state, index: 624 }
    }

    pub(super) fn next_u32(&mut self) -> u32 {
        if self.index >= 624 {
            for k in 0..624 {
                let y = (self.state[k] & 0x8000_0000) | (self.state[(k + 1) % 624] & 0x7fff_ffff);
                let mut next = self.state[(k + 397) % 624] ^ (y >> 1);
                if y & 1 != 0 {
                    next ^= 0x9908_b0df;
                }
                self.state[k] = next;
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// `_randbelow(n)` for 0 < n < 2**32: rejection sampling on `getrandbits(n.bit_length())`.
    fn below(&mut self, n: usize) -> usize {
        let n = u32::try_from(n).expect("fewer than 2**32 items");
        let bits = 32 - n.leading_zeros();
        loop {
            let r = self.next_u32() >> (32 - bits);
            if r < n {
                return r as usize;
            }
        }
    }

    /// `random.choice(items)`.
    pub fn choice<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len())]
    }

    /// `random.shuffle(items)`.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}
