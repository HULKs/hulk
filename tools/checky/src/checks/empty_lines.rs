use ariadne::{Label, ReportKind};
use itertools::Itertools;
use syn::{spanned::Spanned, ImplItem, Item};

use crate::rust_file::RustFile;

use super::Report;

pub fn check(file: &RustFile) -> Vec<Report> {
    let mut reports = Vec::new();
    for (item_before, item_after) in file
        .file
        .items
        .iter()
        .filter(|item| {
            matches!(
                item,
                Item::Enum(_) | Item::Fn(_) | Item::Impl(_) | Item::Struct(_) | Item::Trait(_)
            )
        })
        .tuple_windows()
    {
        if let Some(report) = check_item_spacing(
            file,
            item_before,
            item_after,
            "`enum`, `fn`, `impl`, `struct`, and `trait` must be separated by one empty line",
        ) {
            reports.push(report);
        }

        if let Item::Impl(item) = item_before {
            for (item_before, item_after) in item
                .items
                .iter()
                .filter(|item| matches!(item, ImplItem::Fn(_)))
                .tuple_windows()
            {
                if let Some(report) = check_item_spacing(
                    file,
                    item_before,
                    item_after,
                    "items in `impl` must be separated by one empty line",
                ) {
                    reports.push(report);
                }
            }
        }
    }
    reports
}

fn check_item_spacing<'a>(
    file: &'a RustFile,
    before: impl Spanned,
    after: impl Spanned,
    message: &str,
) -> Option<Report<'a>> {
    let line_before = before.span().end().line;
    let line_after = after.span().start().line;
    if line_before + 1 == line_after {
        let Some(line_before) = file.source.line(line_before - 1) else {
            panic!("line {line_before} does not exist");
        };
        let Some(line_after) = file.source.line(line_after - 1) else {
            panic!("line {line_after} does not exist");
        };

        let span = line_before.offset()..(line_after.offset() + line_after.len());

        Some(
            Report::build(
                ReportKind::Error,
                file.source_id.as_str(),
                line_after.offset(),
            )
            .with_message(message)
            .with_label(
                Label::new((file.source_id.as_str(), span)).with_message("missing empty line"),
            )
            .finish(),
        )
    } else {
        None
    }
}
