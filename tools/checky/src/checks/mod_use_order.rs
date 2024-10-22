use ariadne::{Label, Line, ReportKind};
use syn::{spanned::Spanned, Item, ItemMod, ItemUse, Visibility};

use crate::rust_file::RustFile;

use super::Report;

pub fn check(file: &RustFile) -> Vec<Report> {
    let mut reports = Vec::new();

    let mut last_order_number = 0;
    let mut last_end_line: Option<Line> = None;

    for item in file.file.items.iter() {
        if let Some(order_number) = item_into_order_number(item) {
            let start_line = item.span().start().line;
            let Some(start_line) = file.source.line(start_line - 1) else {
                panic!("line {start_line} does not exist");
            };
            let end_line = item.span().end().line;
            let Some(end_line) = file.source.line(end_line - 1) else {
                panic!("line {end_line} does not exist");
            };

            let is_different_order_group = order_number != last_order_number;
            if let Some(last_end_line) = last_end_line {
                let is_separated_with_empty_line =
                    last_end_line.span().end != start_line.span().start;
                if is_different_order_group && !is_separated_with_empty_line {
                    let span = last_end_line.offset()..(start_line.offset() + start_line.len());

                    reports.push(
                        Report::build(
                            ReportKind::Error,
                            file.source_id.as_str(),
                            start_line.offset(),
                        )
                        .with_message("`mod` and `use` groups must be separated by one empty line")
                        .with_label(
                            Label::new((file.source_id.as_str(), span))
                                .with_message("missing empty line"),
                        )
                        .finish(),
                    );
                }
            }

            let is_too_late = order_number < last_order_number;
            if is_too_late {
                let span = start_line.offset()..(end_line.offset() + end_line.len());

                reports.push(
                    Report::build(
                        ReportKind::Error,
                        file.source_id.as_str(),
                        start_line.offset(),
                    )
                    .with_message("`mod` and `use` not ordered correctly")
                    .with_label(
                        Label::new((file.source_id.as_str(), span))
                            .with_message("must be placed earlier"),
                    )
                    .with_note(
                        "Expected order: pub use, pub(...) use, use, pub mod, pub(...) mod, mod",
                    )
                    .with_help("see https://doc.rust-lang.org/beta/style-guide/items.html")
                    .finish(),
                );
            }

            last_order_number = order_number;
            last_end_line = Some(end_line);
        }
    }

    reports
}

fn item_into_order_number(item: &Item) -> Option<usize> {
    match item {
        Item::Mod(item) => Some(item_mod_into_order_number(item)),
        Item::Use(item) => Some(item_use_into_order_number(item)),
        _ => None,
    }
}

fn item_mod_into_order_number(item: &ItemMod) -> usize {
    200 + match item.vis {
        Visibility::Public(_) => 0,
        Visibility::Restricted(_) => 1,
        Visibility::Inherited => 2,
    }
}

fn item_use_into_order_number(item: &ItemUse) -> usize {
    100 + match item.vis {
        Visibility::Public(_) => 0,
        Visibility::Restricted(_) => 1,
        Visibility::Inherited => 2,
    }
}
