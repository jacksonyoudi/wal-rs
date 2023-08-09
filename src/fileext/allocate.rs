use fs2::FileExt;
use std::fs::File;
use std::io::Result;

/// 这段代码定义了一个函数 `allocate`，它接受一个可变的文件 `f` 和一个大小 `size` 作为参数，并返回一个 `Result` 类型的结果。
//
// 函数使用 `fs2` 模块中的 `FileExt` trait 来扩展 `File` 类型的功能。这个 trait 提供了一个 `allocate` 方法，用于在文件中分配指定大小的空间。
//
// 函数调用 `f.allocate(size as u64)` 来调用 `allocate` 方法，将 `size` 转换为 `u64` 类型作为参数传递给方法。`allocate` 方法会尝试在文件中分配指定大小的空间。
//
// 函数将 `allocate` 方法的返回值直接返回，即返回一个 `Result` 类型的结果。如果分配空间成功，返回一个空的 `Ok` 值，表示操作成功完成。如果分配空间失败，返回一个包含错误信息的 `Err` 值。
//
// 这段代码的作用是在给定的文件中分配指定大小的空间。
pub fn allocate(f: &mut File, size: usize) -> Result<()> {
    f.allocate(size as u64)
}