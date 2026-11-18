//! The reading pane's primitives: its column, its day breaks and
//! rows, and the panel's heads, tracks and cards.

use gpui::{AnyElement, Div, div, prelude::*, px};

use super::READING_WIDTH;
use crate::model;
use crate::theme::Palette;

/// A row of the reading column: full width, its content centred in
/// the reading width.
///
/// The width stands on two divs, not one: a div that is `w_full` with
/// a `max_w` keeps the height its text has at full width once the max
/// clamps it, so the row below starts inside the text above wherever
/// the text wraps. With the max on the inner div alone the row stands
/// as tall as its text wraps.
pub(crate) fn reading_column(row: AnyElement) -> Div {
    div()
        .w_full()
        .child(div().max_w(px(READING_WIDTH)).mx_auto().child(row))
}

/// The day's separator: its name centred between two hairlines, in the
/// reading column.
pub(crate) fn day_break(day: String, palette: Palette) -> AnyElement {
    reading_column(
        div()
            .debug_selector({
                let day = day.clone();
                move || format!("day {day}")
            })
            .mt_2()
            .flex()
            .items_center()
            .gap_2()
            .child(div().h(px(1.)).flex_1().bg(palette.line))
            .child(div().text_sm().text_color(palette.faint).child(day))
            .child(div().h(px(1.)).flex_1().bg(palette.line))
            .into_any_element(),
    )
    // What the rows' gap carried under a row with no body.
    .mb_1()
    .into_any_element()
}

/// A conversation row in the reading column. Group heads stand farther
/// from the mail before them; bodies in one group stand close together.
pub(crate) fn reading_row(row: AnyElement, apart: bool) -> AnyElement {
    let row = reading_column(row);
    if apart {
        row.mt_2().into_any_element()
    } else {
        row.into_any_element()
    }
}

/// What the board's banner says waits on the person reading, on a yard
/// from before `Waiting`: `1 waits on you`, `2 wait on you`. One that
/// serves the call counts the open asks with the beads to move after
/// them instead.
pub(crate) fn waits_head(waiting: usize) -> String {
    match waiting {
        1 => "1 waits on you".to_string(),
        waiting => format!("{waiting} wait on you"),
    }
}

/// The track through a stage's cell: a 2-pixel rail stopping short of
/// each stage boundary for the insulated joint, dashed where people
/// handle the stage. It runs in the border colour through an empty
/// stage and in the faint text colour through one a bead stands on, so
/// a lit section reads along the track as it does on a panel.
pub(crate) fn panel_track_line(human: bool, lit: bool, palette: Palette) -> Div {
    let color = match lit {
        true => palette.faint,
        false => palette.line,
    };
    match human {
        false => div().h(px(2.)).rounded_full().bg(color).mx(px(2.)).my_1(),
        true => {
            let mut dashes = div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .mx(px(2.))
                .my_1();
            for _ in 0..14 {
                dashes = dashes.child(
                    div()
                        .h(px(2.))
                        .w(px(6.))
                        .flex_none()
                        .rounded_full()
                        .bg(color),
                );
            }
            dashes
        }
    }
}

/// A stage name in the panel: GPUI can break an overlong hyphenated
/// name between its letters, so give it an invisible break after each
/// hyphen. The hyphenated words then stay whole when the cell wraps.
pub(crate) fn panel_stage_name(stage: &str, human: bool) -> String {
    let name = stage.replace('-', "-\u{200b}");
    match human {
        true => format!("{name} · people"),
        false => name,
    }
}

/// What the Lines pane says of a yard from before the Lines view, in
/// place of the arrivals or the flows it would draw: plain words in the
/// faint colour, never a notice.
pub(crate) fn lines_too_old(palette: Palette) -> Div {
    div()
        .debug_selector(|| "lines too old".to_string())
        .text_color(palette.faint)
        .child(model::LINES_TOO_OLD)
}

/// A track's card: a surface of its own under its head and its
/// stations, on the raised ground with the border's hairline and the
/// kit's 8-pixel radius, as wide as the pane, so the tracks stand as
/// bands one under another. The card never shrinks: squeezed below its
/// content it would spill its chips over the next row, so the panel
/// scrolls instead.
pub(crate) fn panel_card(palette: Palette) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .flex_none()
        .px_3()
        .py_3()
        .rounded_lg()
        .border_1()
        .border_color(palette.line)
        .bg(palette.raised)
}

/// The buffer stop that closes a terminal stage's track: a short
/// vertical bar where the yard serves the stage terminal, on the
/// panel's cells and in the line view's terminal column.
pub(crate) fn buffer_stop(selector: String, palette: Palette) -> Div {
    div()
        .debug_selector(move || selector.clone())
        .w(px(3.))
        .h(px(14.))
        .flex_none()
        .bg(palette.faint)
}

/// The rule under a stage column's head: the track, 1 pixel in the
/// border colour, in the lamp's colour where one burns, dashed where
/// people handle the stage.
pub(crate) fn column_rule(lamp: &str, human: bool, palette: Palette) -> Div {
    let color = match lamp {
        "fault" => palette.warn,
        "turn" => palette.accent,
        _ => palette.line,
    };
    match human {
        false => div().h_px().flex_none().bg(color),
        true => {
            let mut dashes = div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .flex_none();
            for _ in 0..16 {
                dashes = dashes.child(div().h_px().w(px(6.)).flex_none().bg(color));
            }
            dashes
        }
    }
}
