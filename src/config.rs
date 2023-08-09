#[derive ! (Debug, Copy, Clone, Eq, PartialEq)]
pub struct Config {
    // 单个 segment的条目数量
    pub entry_per_segment: usize,

    pub check_crc32: bool,
}