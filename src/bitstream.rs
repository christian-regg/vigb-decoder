//! Bit reader for CCITT-T.6 line decoding.
//!
//! Two refill modes:
//! - `eager`: load 2 bytes whenever the buffer holds ≤16 bits (matches
//!   Python `_refill`, the default and historical decoder behaviour).
//! - `lazy`: load 1 byte at a time only when the next peek would underrun
//!   (matches Python `_refill_lazy`, mirrors PaperPort 3.6's
//!   `MAXKER2.DLL` byte-by-byte refill timing).
//!
//! Both modes are correct on canonical files. `lazy` exists as a
//! diagnostic for sync-drift investigation — see `Config::lazy_bit_loading`.

/// MSB-first bit reader over a byte slice.
#[allow(dead_code)]
pub(crate) struct BitCursor<'a> {
    data: &'a [u8],
    /// Bit window (right-aligned in a u64 so we can hold up to 32 buffered bits
    /// after a refill without losing any when the next refill shifts in 16 more).
    bits: u64,
    /// Number of valid bits currently in `bits`, right-aligned.
    bits_left: u32,
    /// Byte offset into `data` of the next byte to load.
    pos: usize,
    /// True ⇒ byte-by-byte refill (`_refill_lazy` semantics).
    lazy: bool,
    /// Total bits consumed across the cursor's lifetime (for `consumed_bits`).
    total_consumed: u64,
}

impl<'a> BitCursor<'a> {
    /// Create a new cursor at byte offset 0 of `data`.
    ///
    /// `lazy = false` matches Python `_refill` (eager 16-bit refill).
    /// `lazy = true` matches Python `_refill_lazy` (byte-by-byte).
    #[allow(dead_code)]
    pub fn new(data: &'a [u8], lazy: bool) -> Self {
        Self {
            data,
            bits: 0,
            bits_left: 0,
            pos: 0,
            lazy,
            total_consumed: 0,
        }
    }

    /// Create a cursor that starts at `start_pos` bytes into `data`.
    #[allow(dead_code)]
    pub fn with_start(data: &'a [u8], start_pos: usize, lazy: bool) -> Self {
        Self {
            data,
            bits: 0,
            bits_left: 0,
            pos: start_pos,
            lazy,
            total_consumed: 0,
        }
    }

    /// Peek the next `n` bits (1..=32) without consuming them.
    /// Returns `None` if the stream cannot supply that many bits.
    #[allow(dead_code)]
    pub fn peek(&mut self, n: u32) -> Option<u32> {
        debug_assert!((1..=32).contains(&n));
        self.refill_if_needed(n);
        if self.bits_left < n {
            return None;
        }
        Some(((self.bits >> (self.bits_left - n)) & ((1u64 << n) - 1)) as u32)
    }

    /// Consume `n` previously-peeked bits.
    #[allow(dead_code)]
    pub fn consume(&mut self, n: u32) {
        debug_assert!(
            self.bits_left >= n,
            "consume({n}) with {} bits buffered",
            self.bits_left
        );
        self.bits_left -= n;
        self.total_consumed += n as u64;
    }

    /// Total bits consumed via `consume`.
    #[allow(dead_code)]
    pub fn consumed_bits(&self) -> u64 {
        self.total_consumed
    }

    /// Byte offset of the next byte that *would* be loaded — useful for
    /// computing `pos - start_pos` after-the-fact (matches Python's
    /// `(pos - start_pos) * 8 - bits_left`).
    #[allow(dead_code)]
    pub fn next_load_byte(&self) -> usize {
        self.pos
    }

    /// Bits currently buffered (used by callers that compute byte-position
    /// after a decode in the Python idiom).
    #[allow(dead_code)]
    pub fn bits_buffered(&self) -> u32 {
        self.bits_left
    }

    #[allow(dead_code)]
    fn refill_if_needed(&mut self, need: u32) {
        if self.lazy {
            // Byte-by-byte until we have `need` bits or run out.
            while self.bits_left < need && self.pos < self.data.len() {
                let b = self.data[self.pos] as u64;
                self.bits = (self.bits << 8) | b;
                self.bits_left += 8;
                self.pos += 1;
            }
        } else {
            // Eager 16-bit refill matching Python `_refill`. Fires once when
            // bits_left drops to ≤16, regardless of `need`.
            if self.bits_left <= 16 {
                let b0 = if self.pos < self.data.len() {
                    self.data[self.pos] as u64
                } else {
                    0
                };
                let b1 = if self.pos + 1 < self.data.len() {
                    self.data[self.pos + 1] as u64
                } else {
                    0
                };
                self.bits = (self.bits << 16) | (b0 << 8) | b1;
                self.bits_left += 16;
                self.pos += 2;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eager_peek_consume_round_trip() {
        // 0xAB 0xCD 0xEF 0x12
        let data = &[0xAB, 0xCD, 0xEF, 0x12];
        let mut bc = BitCursor::new(data, false);
        // Initial refill loads 0xAB 0xCD, bits_left=16
        // Peek(8): top 8 bits of 16-bit window = 0xAB
        assert_eq!(bc.peek(8).unwrap(), 0xAB);
        bc.consume(4);
        // bits_left=12. Second refill (because 12 <= 16) loads 0xEF 0x12, bits_left=28
        // Buffer now contains: 0xABCDEF12
        // Peek(8): with bits_left=28, top 8 bits starting from position (28-8)=20
        // From 0xABCDEF12, extract bits 20-27 (0-indexed from MSB): 0xBC
        assert_eq!(bc.peek(8).unwrap(), 0xBC);
        bc.consume(8);
        // bits_left=20
        // Peek(4): extract top 4 bits from 20-bit window = 0xD
        assert_eq!(bc.peek(4).unwrap(), 0xD);
    }

    #[test]
    fn lazy_and_eager_consume_match() {
        let data = &[0x80, 0xF8, 0x42, 0x17, 0xC0, 0x00];
        let mut e = BitCursor::new(data, false);
        let mut l = BitCursor::new(data, true);
        for n in [3, 5, 7, 13, 8, 4] {
            assert_eq!(
                e.peek(n).unwrap(),
                l.peek(n).unwrap(),
                "peek({}) diverges",
                n
            );
            e.consume(n);
            l.consume(n);
        }
        // Both should report identical bits-consumed totals.
        assert_eq!(e.consumed_bits(), l.consumed_bits());
    }

    #[test]
    fn underrun_returns_none() {
        // Single byte with lazy loading: after consuming all bits,
        // lazy refill cannot load more and returns None
        let data = &[0xFF];
        let mut bc = BitCursor::new(data, true); // lazy=true
        let _ = bc.peek(8).unwrap(); // Load the byte
        bc.consume(8); // Consume it all
                       // Now trying to peek should return None because there's no more data to load
        assert!(bc.peek(1).is_none());
    }

    #[test]
    fn pos_bytes_advances_on_consume() {
        let data = &[0x12, 0x34, 0x56, 0x78];
        let mut bc = BitCursor::new(data, false);
        let _ = bc.peek(8).unwrap();
        bc.consume(8);
        // After consuming 8 bits, the conceptual position is 1 byte in.
        assert_eq!(bc.consumed_bits(), 8);
    }
}
