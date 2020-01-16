//! The ChaCha20 block function. Defined in RFC 8439 Section 2.3.
//!
//! <https://tools.ietf.org/html/rfc8439#section-2.3>
//!
//! Portable implementation which does not rely on architecture-specific
//! intrinsics.

use crate::{BLOCK_SIZE, CONSTANTS, IV_SIZE, KEY_SIZE, STATE_WORDS};
use core::{convert::TryInto, mem};

/// Size of buffers passed to `generate` and `apply_keystream` for this backend
pub(crate) const BUFFER_SIZE: usize = BLOCK_SIZE;

/// The ChaCha20 block function (portable software implementation)
// TODO(tarcieri): zeroize?
#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct Block {
    /// Internal state of the block function
    state: [u32; STATE_WORDS],

    /// Number of rounds to perform
    rounds: usize,
}

#[allow(dead_code)]
impl Block {
    /// Initialize block function with the given key size, IV, and number of rounds
    pub(crate) fn new(key: &[u8; KEY_SIZE], iv: [u8; IV_SIZE], rounds: usize) -> Self {
        assert!(
            rounds == 8 || rounds == 12 || rounds == 20,
            "rounds must be 8, 12, or 20"
        );

        let mut state: [u32; STATE_WORDS] = unsafe { mem::zeroed() };
        state[..4].copy_from_slice(&CONSTANTS);

        for (i, chunk) in key.chunks(4).enumerate() {
            state[4 + i] = u32::from_le_bytes(chunk.try_into().unwrap());
        }

        state[12] = 0;
        state[13] = 0;
        state[14] = u32::from_le_bytes(iv[0..4].try_into().unwrap());
        state[15] = u32::from_le_bytes(iv[4..].try_into().unwrap());

        Self { state, rounds }
    }

    /// Generate output, overwriting data already in the buffer
    pub(crate) fn generate(&mut self, counter: u64, output: &mut [u8]) {
        debug_assert_eq!(output.len(), BUFFER_SIZE);
        self.counter_setup(counter);

        let mut state = self.state;
        self.rounds(&mut state);

        for (i, chunk) in output.chunks_mut(4).enumerate() {
            chunk.copy_from_slice(&state[i].to_le_bytes());
        }
    }

    /// Apply generated keystream to the output buffer
    pub(crate) fn apply_keystream(&mut self, counter: u64, output: &mut [u8]) {
        debug_assert_eq!(output.len(), BUFFER_SIZE);
        self.counter_setup(counter);

        let mut state = self.state;
        self.rounds(&mut state);

        for (i, chunk) in output.chunks_mut(4).enumerate() {
            for (a, b) in chunk.iter_mut().zip(&state[i].to_le_bytes()) {
                *a ^= *b;
            }
        }
    }

    #[inline]
    fn counter_setup(&mut self, counter: u64) {
        self.state[12] = (counter & 0xffff_ffff) as u32;
        self.state[13] = ((counter >> 32) & 0xffff_ffff) as u32;
    }

    #[inline]
    fn rounds(&mut self, state: &mut [u32; STATE_WORDS]) {
        for _ in 0..(self.rounds / 2) {
            // column rounds
            quarter_round(0, 4, 8, 12, state);
            quarter_round(1, 5, 9, 13, state);
            quarter_round(2, 6, 10, 14, state);
            quarter_round(3, 7, 11, 15, state);

            // diagonal rounds
            quarter_round(0, 5, 10, 15, state);
            quarter_round(1, 6, 11, 12, state);
            quarter_round(2, 7, 8, 13, state);
            quarter_round(3, 4, 9, 14, state);
        }

        for (s1, s0) in state.iter_mut().zip(&self.state) {
            *s1 = s1.wrapping_add(*s0);
        }
    }
}

/// The ChaCha20 quarter round function
#[inline]
pub(crate) fn quarter_round(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    state: &mut [u32; STATE_WORDS],
) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(16);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(12);

    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(8);

    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(7);
}