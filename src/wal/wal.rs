use crate::config::Config;
use crate::segment::segment::Segment;
use std::ffi::OsStr;
use std::fs;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use super::cursor::Cursor;


pub struct WAL {
    cfg: Config,
    dir: PathBuf,
    cursor: Cursor,
    next_sequence: u64,
    segments: Vec<Segment>,
}

impl WAL {
    pub fn open<S: AsRef<OsStr> + ?Sized>(dir: &S, cfg: Config) -> Result<WAL> {
        let p = Path::new(dir);
        if !p.exists() {
            fs::create_dir_all(&p)?;
        }

        if !p.is_dir() {
            return Err(Error::new(ErrorKind::Other, "expecting a directory"));
        }

        let dir = p.to_path_buf();

        let mut cursor = Cursor::open(&dir)?;

        let mut read_sequence = cursor.position.sequence;
        let mut segments: Vec<Segment> = Vec::with_capacity(10);
        loop {
            match Segment::open(&dir, read_sequence, cfg.entry_per_segment, false) {
                Ok(s) => segments.push(s),
                Err(ref e) if e.kind() == ErrorKind::NotFound => break,
                Err(e) => return Err(e),
            }

            read_sequence += 1;
        }

        match segments.first() {
            Some(s) => if s.len() < cursor.position.read as usize {
                cursor.position.read = 0;
            },
            None => {
                cursor.position.sequence = 0;
                cursor.position.read = 0;
            }
        }

        Ok(WAL {
            cfg: cfg,
            dir: dir,
            cursor: cursor,
            next_sequence: read_sequence,
            segments: segments,
        })
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        self.try_allocate(1)?;
        let segment = self.segments.last_mut().unwrap();
        segment.write(data)?;

        Ok(())
    }

    pub fn batch_write(&mut self, mut data: &[&[u8]]) -> Result<()> {
        while !data.is_empty() {
            let space = self.try_allocate(data.len())?;

            let segment = self.segments.last_mut().unwrap();
            let written = segment.batch_write(&data[0..space])?;
            data = &data[written..];
        }

        Ok(())
    }

    fn try_allocate(&mut self, n: usize) -> Result<(usize)> {
        match self.segments.last_mut() {
            Some(ref s) if s.space() > 0 => {
                let space = s.space();

                return if space > n { Ok(n) } else { Ok(space) };
            }
            Some(ss) => {
                let _ = ss.flush();
            }
            None => {}
        }

        let new_seg = Segment::open(
            &self.dir,
            self.next_sequence,
            self.cfg.entry_per_segment,
            true,
        )?;
        let space = new_seg.space();
        self.next_sequence += 1;
        self.segments.push(new_seg);

        if space > n {
            Ok(n)
        } else {
            Ok(space)
        }
    }

    pub fn len(&self) -> usize {
        let mut size: usize = 0;

        for segment in &self.segments {
            let num = if segment.sequence() == self.cursor.position.sequence {
                segment.len() - self.cursor.position.read as usize
            } else {
                segment.len()
            };

            size += num;
        }

        size
    }
}
