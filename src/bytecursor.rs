use core::{
  cmp,
  convert::TryInto,
};

use alloc::{
  borrow::ToOwned,
  string::String,
  vec::Vec,
};

pub enum SeekFrom {
  Start(u64),
  End(i64),
  Current(i64),
}

#[derive(Clone, Debug)]
pub struct ByteCursor {
  inner: Vec<u8>,
  pos: u64,
}

impl ByteCursor {
  /// Creates a new `Bytecursor` from the inner bytes it will contains.
  /// Sets the position to 0 initially.
  #[must_use]
  pub const fn new(inner: Vec<u8>) -> Self { Self { pos: 0, inner } }

  /// Consumes the `Bytecursor`, returning the inner bytes.
  #[must_use]
  pub fn into_inner(self) -> Vec<u8> { self.inner }

  /// Returns an immutable reference to the inner bytes of the `Bytecursor`
  #[must_use]
  pub const fn get_ref(&self) -> &Vec<u8> { &self.inner }

  /// Returns a mutable reference to the inner bytes of the `Bytecursor`
  pub fn get_mut(&mut self) -> &mut Vec<u8> { &mut self.inner }

  /// Returns the current position of the `Bytecursor`
  #[must_use]
  pub const fn position(&self) -> u64 { self.pos }

  /// Sets the position of the `Bytecursor` to `pos`
  pub fn set_position(&mut self, pos: u64) { self.pos = pos }

  /// Reads `buf.len()` bytes into `buf` from `read`, advancing
  /// the `Bytecursor`'s position. It returns the number of bytes
  /// actually read.
  pub fn read(&mut self, buf: &mut [u8]) -> usize {
    let from = &mut self.fill_buf();
    let amt = cmp::min(buf.len(), from.len());
    let (a, b) = from.split_at(amt);
    if amt == 1 {
      buf[0] = a[0];
    }
    else {
      buf[..amt].copy_from_slice(a);
    }
    *from = b;
    self.pos += amt as u64;
    amt
  }

  /// Reads exactly `buf.len()` bytes into `buf`, throwing an error if
  /// that number of bytes was not able to be read.
  /// # Errors
  ///
  /// Will return `Err` if the buffer is longer than the available bytes to read
  pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<(), String> {
    let n = buf.len();
    let from = &mut self.fill_buf();
    if buf.len() > from.len() {
      return Err("failed to fill whole buffer".to_owned());
    }
    let (a, b) = from.split_at(buf.len());

    if buf.len() == 1 {
      buf[0] = a[0];
    }
    else {
      buf.copy_from_slice(a);
    }

    *from = b;
    self.pos += n as u64;
    Ok(())
  }

  /// Returns a byte slice containing all remaining bytes
  /// in the inner bytes after the current position of the
  /// `Bytecursor`.
  pub fn fill_buf(&mut self) -> &[u8] {
    let amt = cmp::min(self.pos, self.inner.len() as u64);
    &self.inner[(amt as usize)..] // may truncate
  }

  /// Seeks to the position referenced by `style`, returning the new position
  /// of the `Bytecursor` and throwing an error if the new position would be
  /// invalid. # Errors
  ///
  /// Will return `Err` if one tries to seek to a negative or overflowing
  /// position
  pub fn seek(&mut self, style: &SeekFrom) -> Result<u64, String> {
    let (base_pos, offset) = match style {
      SeekFrom::Start(n) => {
        self.pos = *n;
        return Ok(*n);
      }
      SeekFrom::End(n) => {
        let x: &[u8] = self.inner.as_ref();
        (x.len() as u64, n)
      }
      SeekFrom::Current(n) => (self.pos, n),
    };
    let new_pos = if *offset >= 0 {
      base_pos.checked_add(*offset as u64) // may lose sign
    }
    else {
      base_pos.checked_sub((offset.wrapping_neg()) as u64) // may lose sign
    };
    match new_pos {
      Some(n) => {
        self.pos = n;
        Ok(self.pos)
      }
      None => {
        Err("invalid seek to a negative or overflowing position".to_owned())
      }
    }
  }

  /// Writes `buf.len()` bytes into `buf`. Returns the number of bytes actually
  /// read if successful, and throws an error if there aren't enough bytes to
  /// read. # Errors
  ///
  /// Will return `Err` if the cursor position exceeds maximum possible vector
  /// length
  pub fn write(&mut self, buf: &[u8]) -> Result<usize, String> {
    let vec = &mut self.inner;
    let pos: usize = self.pos.try_into().map_err(|_| {
      "cursor position exceeds maximum possible vector length".to_owned()
    })?;
    let len = vec.len();
    if len < pos {
      vec.resize(pos, 0);
    }
    {
      let space = vec.len() - pos;
      let (left, right) = buf.split_at(cmp::min(space, buf.len()));
      vec[pos..pos + left.len()].copy_from_slice(left);
      vec.extend_from_slice(right);
    }
    self.pos = (pos + buf.len()) as u64;
    Ok(buf.len())
  }

  /// Writes all of `buf` to the `Bytecursor` until `buf` is empty.
  /// # Errors
  ///
  /// Will return `Err` if the cursor position exceeds maximum possible vector
  /// length or we failed to write whole buffer
  pub fn write_all(&mut self, mut buf: &[u8]) -> Result<(), String> {
    while !buf.is_empty() {
      match self.write(buf) {
        Ok(0) => {
          return Err("failed to write whole buffer".to_owned());
        }
        Ok(n) => buf = &buf[n..],
        Err(e) => return Err(e),
      }
    }
    Ok(())
  }
}
