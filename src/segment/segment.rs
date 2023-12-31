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

    ///
    /// 打开一个segment,传递进一些参数
    ///
    pub fn open<P: AsRef<OsStr> + ?Sized>(dir: &P, sequence: u64, mut limit: usize, create: bool) -> IOResult<Segment> {
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

    ///
    /// 写入数据
    pub fn write(&mut self, entry: &[u8]) -> IOResult<bool> {
        if self.entry_number >= self.entry_limit {
            return Ok(false);
        }


        let offset = self.data_written as u64;
        // 数据写入
        fileext::fileext::write_all_at(&self.file, entry, offset)?;
        let written = entry.len();
        self.data_written += written;
        self.crc32.reset();
        self.crc32.write(entry);

        self.overhead.write_head();
        self.overhead.write_offset(offset);
        self.overhead.write_size(entry.len() as u64);
        self.overhead.write_crc32(self.crc32.sum32());

        let overhead_offset = (HEAD_SIZE + self.entry_number * OVERHEAD_SIZE) as u64;
        fileext::fileext::write_all_at(&self.file, self.overhead.bytes(), overhead_offset)?;

        self.entry_number += 1;
        Ok(true)
    }

    /// 多次调用 write
    pub fn batch_write(&mut self, mut entries: &[&[u8]]) -> IOResult<usize> {
        let size = entries.len();

        while !entries.is_empty() {
            match self.write(entries[0]) {
                Ok(false) => break,
                Ok(true) => entries = &entries[1..],
                Err(e) => return Err(e),
            }
        }

        Ok(size - entries.len())
    }

    pub fn read_into(&self, start: usize, mut limit: usize, data: &mut Vec<Vec<u8>>, check: bool) -> IOResult<usize> {
        if start >= self.entry_number {
            return Ok(0);
        }

        if start + limit > self.entry_number {
            limit = self.entry_number - start;
        }

        if limit == 0 {
            return Ok(0);
        }

        let mut buf = vec![0; limit * OVERHEAD_SIZE];
        let offset = (HEAD_SIZE + start * OVERHEAD_SIZE) as u64;
        let mut temp = Vec::with_capacity(limit);


        fileext::read_exact_at(&self.file, &mut buf, offset)?;

        let mut read: usize = 0;
        let mut overhead = Overhead::new();
        let mut digest = Digest::new(IEEE);
        while read < limit {
            overhead.copy_bytes(&buf[read * OVERHEAD_SIZE..(read + 1) * OVERHEAD_SIZE]);
            if !overhead.valid() {
                return Err(Error::new(ErrorKind::InvalidData, "invalid overhead"));
            }

            let mut entry = vec![0; overhead.size() as usize];
            fileext::read_exact_at(&self.file, &mut entry, overhead.offset())?;

            if check {
                digest.reset();
                digest.write(&entry);
                if digest.sum32() != overhead.crc32() {
                    return Err(Error::new(ErrorKind::InvalidData, "fail to check crc32"));
                }
            }
            temp.push(entry);

            read += 1;
        }

        data.append(&mut temp);

        Ok(read)
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn len(&self) -> usize {
        self.entry_number
    }

    pub fn space(&self) -> usize {
        self.entry_limit - self.entry_number
    }

    pub fn flush(&mut self) -> IOResult<()> {
        self.file.sync_all()
    }

    pub fn destory(&mut self) {
        let _ = remove_file(&self.fname);
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

fn read_info(f: &mut File) -> IOResult<(usize, usize)> {
    f.seek(SeekFrom::Start(0))?;

    let mut buf = [0; MAGIC_SIZE];
    f.read_exact(&mut buf[..])?;
    if buf != MAGIC_NUM {
        return Err(Error::new(ErrorKind::Other, "invalid magic num"));
    }

    f.read_exact(&mut buf[..ENTRY_LIMIT_SIZE])?;
    let entry_limit = BigEndian::read_u64(&buf[..ENTRY_LIMIT_SIZE]) as usize;

    let mut num = 0_usize;
    let mut oh = Overhead::new();
    while num < entry_limit {
        oh.reset();
        let _ = oh.read_from(f);

        if !oh.valid() {
            break;
        }

        num += 1;
    }

    Ok((entry_limit, num))
}