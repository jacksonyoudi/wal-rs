#[cfg(unix)]
mod fileext_unix;
#[cfg(unix)]
pub use self::fileext_unix::*;

#[cfg(windows)]
mod fileext_win;
#[cfg(windows)]
pub use self::fileext_win::*;

mod allocate;

use std::fs::File;
use std::io::{Error, ErrorKind, Result};


// 指定文件中读取数据
pub fn read_exact_at(f: &File, mut buf: &mut [u8], mut offset: u64) -> Result<()> {
    while !buf.is_empty() {
        match read_at(f, &mut buf, offset){
            Ok(0) => break,
            Ok(n) => {

            }
        }
    }
}


