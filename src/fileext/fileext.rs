#[cfg(unix)]
pub use super::fileext_unix::*;

#[cfg(windows)]
mod fileext_win;

#[cfg(windows)]
pub use super::fileext_win::*;

use std::fs::File;
use std::io::{Error, ErrorKind, Result};

///
/// 这段代码是一个函数 `read_exact_at`，它接受一个文件 `f`，一个可变的字节数组 `buf` 和一个偏移量 `offset` 作为参数，并返回一个 `Result` 类型的结果。
//
// 函数使用一个 `while` 循环来读取文件中的数据，直到 `buf` 数组为空。在每次循环中，它调用 `read_at` 函数来读取文件中的数据，并根据返回值进行不同的处理：
//
// - 如果返回值是 `Ok(0)`，表示已经读取到文件末尾，循环结束。
// - 如果返回值是 `Ok(n)`，表示成功读取了 `n` 个字节的数据。函数会更新偏移量 `offset`，然后将 `buf` 数组的引用更新为剩余未读取的部分。
// - 如果返回值是 `Err`，函数会检查错误类型是否为 `ErrorKind::Interrupted`，如果是，则继续循环。否则，将错误返回给调用者。
//
// 循环结束后，函数会检查 `buf` 数组是否为空。如果不为空，表示没有读取到足够的数据，函数会返回一个 `UnexpectedEof` 错误。否则，函数返回一个空的 `Ok` 值，表示读取操作成功完成。
//
// 这段代码的作用是从指定偏移量开始，连续读取文件中的数据，直到读取到文件末尾或者读取到足够的数据填满 `buf` 数组。
pub fn read_exact_at(f: &File, mut buf: &mut [u8], mut offset: u64) -> Result<()> {
    while !buf.is_empty() {
        match read_at(f, &mut buf, offset) {
            Ok(0) => break,
            Ok(n) => {
                offset += n as u64;
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    if !buf.is_empty() {
        return Err(Error::from(ErrorKind::UnexpectedEof));
    }

    Ok(())
}


///
/// 这段代码是一个函数 `write_all_at`，它接受一个文件 `f`，一个字节数组 `buf` 和一个偏移量 `offset` 作为参数，并返回一个 `Result` 类型的结果。
//
// 函数使用一个 `while` 循环来写入数据到文件，直到 `buf` 数组为空。在每次循环中，它调用 `write_at` 函数来将数据写入文件，并根据返回值进行不同的处理：
//
// - 如果返回值是 `Ok(0)`，表示写入操作没有写入任何数据，即写入了零个字节。函数会返回一个 `WriteZero` 错误，表示写入整个缓冲区失败。
// - 如果返回值是 `Ok(n)`，表示成功写入了 `n` 个字节的数据。函数会更新 `buf` 数组的引用，将已经写入的部分从数组中移除，并更新偏移量 `offset`。
// - 如果返回值是 `Err`，函数会检查错误类型是否为 `ErrorKind::Interrupted`，如果是，则继续循环。否则，将错误返回给调用者。
//
// 循环结束后，函数会返回一个空的 `Ok` 值，表示写入操作成功完成。
//
// 这段代码的作用是从指定偏移量开始，连续将数据写入文件，直到写入完整的 `buf` 数组或者写入操作失败。如果写入操作没有写入任何数据，函数会返回一个 `WriteZero` 错误。
pub fn write_all_at(f: &File, mut buf: &[u8], mut offset: u64) -> Result<()> {
    while !buf.is_empty() {
        match write_at(f, buf, offset) {
            Ok(0) => {
                return Err(Error::new(
                    ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            Ok(n) => {
                buf = &buf[n..];
                offset += n as u64
            }
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}