use byteorder::{BigEndian, ByteOrder};
use crate::segment::overhead::{Overhead, OVERHEAD_SIZE};
use crc::crc32::{Digest, Hasher32, IEEE};
use hex::encode;
use std::ffi::OsStr;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{Error, ErrorKind, Read, Result as IOResult, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use crate::fileext;

const MAGIC_NUM: [u8; 16] = [
    17, 116, 239, 237, 171, 24, 96, 0, 17, 116, 239, 237, 171, 24, 96, 117,
];
const MAGIC_SIZE: usize = 16;
const ENTRY_LIMIT_SIZE: usize = 8;
const DEFAULT_ENTRY_LIMIT: usize = 10 << 10;
const HEAD_SIZE: usize = MAGIC_SIZE + ENTRY_LIMIT_SIZE;


pub struct Segment {
    sequence: u64,

    fname: PathBuf,
    file: File,

    entry_number: usize,
    entry_limit: usize,
    data_written: usize,

    overhead: Overhead,
    crc32: Digest,
}


impl Segment {
    pub fn filename(sequence: u64) -> String {
        u64_to_hex(sequence)
    }

    pub fn open<P: AsRef<OsStr> + ?Sized>(
        dir: &P,
        sequence: u64,
        mut limit: usize,
        create: bool,
    ) -> IOResult<Segment> {
        if limit == 0 {
            limit = DEFAULT_ENTRY_LIMIT;
        }

        let fname = Path::new(dir).join(Segment::filename(sequence));
        let mut file = OpenOptions::new()
            .create(create)
            .read(true)
            .write(true)
            .open(&fname)?;

        let meta = file.metadata()?;
        if meta.len() == 0 {
            prepare(&mut file, limit)?;
        }

        let (entry_limit, entry_number) = read_info(&mut file)?;

        let data_written = file.seek(SeekFrom::End(0))?;

        Ok(Segment {
            sequence: sequence,
            fname: fname,
            file: file,
            entry_limit: entry_limit,
            entry_number: entry_number,
            data_written: data_written as usize,
            overhead: Overhead::new(),
            crc32: Digest::new(IEEE),
        })


    }
}

fn u64_to_hex(n: u64) -> String {
    let mut buf = vec![0; 8];
    BigEndian::write_u64(&mut buf, n);
    encode(buf)
}

fn prepare(f: &mut File, entry_limit: usize) -> IOResult<()> {
    fileext::allocate::allocate(f, HEAD_SIZE + entry_limit * OVERHEAD_SIZE)?;
    fileext::fileext::write_all_at(f, &MAGIC_NUM[..], 0)?;

    let mut size_buf: [u8; 8] = [0; 8];
    BigEndian::write_u64(&mut size_buf, entry_limit as u64);
    fileext::fileext::write_all_at(f, &size_buf, MAGIC_SIZE as u64)
}