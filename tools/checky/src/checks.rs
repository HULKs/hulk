use std::ops::Range;

use crate::rust_file::RustFile;

pub mod empty_lines;
pub mod mod_use_order;

pub type Report<'a> = ariadne::Report<'a, (&'a str, Range<usize>)>;

pub fn check(file: &RustFile) -> Vec<Report> {
    let mut reports = Vec::new();
    reports.extend(empty_lines::check(file));
    reports.extend(mod_use_order::check(file));
    reports
}
