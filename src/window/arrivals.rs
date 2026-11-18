//! The timetable's views: the arrivals pane with its panel of
//! tracks and departures board, the lines' flows, and the beads that
//! travel with no line.

use gpui::{
    AnyElement, Context, FontWeight, MouseButton, SharedString, Window, div, prelude::*, px, rems,
    rgba,
};

use super::Footplate;
use super::ask::section_head;
use super::metrics::{
    BACKLOG_TITLES, BOARD_BEAD, BOARD_EXPECTED, BOARD_LINE, BOARD_PLATFORM, BOARD_REMARK, FLOW_ID,
    LINE_COLUMN, LINE_HEAD, PANEL_HEAD_NAME_FLOOR, QUIET_NAME, QUIET_NAME_CHARS, ROW_MARK,
    TRACK_STATION, flow_empty_budgets, flow_empty_stage_budget, flow_group_budgets,
    flow_quiet_path_budget, flow_row_budgets, flow_strip_overflows, grown, panel_head_name_budget,
};
use super::panel::{
    buffer_stop, column_rule, lines_too_old, panel_card, panel_stage_name, waits_head,
};
use super::rail::RailTip;
use super::screen::screen_control;
use super::ui::{fold_chevron, outcome_glyph, segment, segmented, shorten};
use super::words::{
    bead_words, chip_you, decide_move_words, depot_lamp, flow_path, flow_status, flow_words,
    headcode_color, lamp_dot, landed_age, line_counts, moving_quiet_words, next_stage,
    on_you_words, remark_color, stage_group_words, stage_lamp, stage_state, station_node,
    waiting_words, waits_for_seat, with_beads_words, worked_first, worker_words, working_tip,
    working_words,
};
use crate::api::{Arrival, Flow, FlowBead, FlowDepot, FlowLanding, FlowStage};
use crate::model::{self, Mode, Model, Selected, short_bead, stage_age, you_first};
use crate::select;
use crate::theme::{
    Palette, TYPE_BODY, TYPE_CODE, TYPE_LABEL, TYPE_META, TYPE_SECTION, TYPE_TITLE,
};

/// Lines' waiting words, worded: the accent run with the quiet run after
/// it, each where any are. A yard from before `Waiting` keeps its one
/// accent line from the remarks of the beads it shows, worded by `old`,
/// which the panel's summary and the board's banner word apart; one that
/// serves the call counts the open asks with the beads to move after
/// them. The panel and the banner draw the same pair, so the two never
/// disagree.
fn lines_waits_words(
    model: &Model,
    old: impl Fn(usize) -> String,
) -> (Option<String>, Option<String>) {
    match model.waiting_too_old() {
        true => {
            let waiting = model.waiting_on_you().len();
            match waiting > 0 {
                true => (Some(old(waiting)), None),
                false => (None, None),
            }
        }
        false => decide_move_words(model.decisions_count(), model.parked_count()),
    }
}

impl Footplate {
    pub(crate) fn toggle_quiet(&mut self, cx: &mut Context<Self>) {
        self.change(cx, |m| {
            m.toggle_quiet();
            Vec::new()
        });
    }

    /// The arrivals head's toggle sets which view stands: the
    /// departures board where true, the panel where false.
    pub(crate) fn set_departures(&mut self, departures: bool, cx: &mut Context<Self>) {
        self.change(cx, |m| {
            m.set_departures(departures);
            Vec::new()
        });
    }

    /// The flow head's toggle sets which view stands: the board where
    /// true, the list where false.
    pub(crate) fn set_flow_board(&mut self, board: bool, cx: &mut Context<Self>) {
        self.change(cx, |m| {
            m.set_flow_board(board);
            Vec::new()
        });
    }

    /// The depot's quiet fold sets whether its quiet flows stand
    /// unfolded under it in the list.
    pub(crate) fn toggle_quiet_flows(&mut self, depot: &str, cx: &mut Context<Self>) {
        let depot = depot.to_string();
        self.change(cx, |m| {
            m.toggle_quiet_flows(&depot);
            Vec::new()
        });
    }

    /// The flow list's empty fold sets whether its empty stages stand
    /// unfolded under it.
    pub(crate) fn toggle_empty_stages(
        &mut self,
        depot: &str,
        name: &str,
        flow_type: &str,
        cx: &mut Context<Self>,
    ) {
        let (depot, name, flow_type) = (depot.to_string(), name.to_string(), flow_type.to_string());
        self.change(cx, |m| {
            m.toggle_empty_stages(&depot, &name, &flow_type);
            Vec::new()
        });
    }

    /// The timetable's list: the arrivals row with the yard-wide
    /// travelling total, then the depots with beads — each active depot
    /// with its flows nested under it, No flow while beads travel with
    /// none — and the quiet depots folded under Quiet depots.
    pub(crate) fn list_timetable(
        &self,
        model: &Model,
        palette: Palette,
        top: f32,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut body = self.list_body();
        let arrivals = model.arrivals_row();
        body = body.child(section_head("Timetable", arrivals.line.clone(), palette));
        body = body.child(self.arrivals_tree_row(model, palette, cx));
        let active: Vec<&FlowDepot> = model
            .depots()
            .iter()
            .filter(|depot| depot.travelling > 0)
            .collect();
        if !active.is_empty() || !model.unassigned().is_empty() {
            let rows = active.len() + usize::from(!model.unassigned().is_empty());
            body = body.child(section_head("Depots", with_beads_words(rows), palette));
        }
        for depot in active {
            body = body.child(self.depot_tree_row(model, depot, palette, cx));
            body = body.child(self.depot_flow_rows(model, depot, palette, cx));
        }
        if !model.unassigned().is_empty() {
            body = body.child(self.no_flow_tree_row(model, palette, cx));
        }
        let quiet = model.quiet_rows();
        if !quiet.is_empty() {
            let open = model.quiet_expanded();
            body = body.child(
                div()
                    .id("quiet")
                    .debug_selector(|| "quiet".to_string())
                    .mx_1()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .gap_2()
                    .hover(|style| style.bg(palette.hover))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_quiet(cx)))
                    .child(fold_chevron(open, palette.faint))
                    .child(
                        div()
                            .text_size(rems(TYPE_SECTION))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Quiet depots"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_right()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child(format!("{} · nothing moving", quiet.len())),
                    ),
            );
            if open {
                for row in quiet {
                    let Some(depot) = model.depot(&row.name) else {
                        continue;
                    };
                    body = body.child(self.quiet_tree_row(model, depot, palette, cx));
                }
            }
        }
        self.column(body, palette, top, window, cx)
    }

    /// One tree row's shell: the list row's shape — the open row filled,
    /// a row under the pointer told by its hairline — opening `to`.
    fn tree_row(
        &self,
        id: String,
        to: Selected,
        open: bool,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        div()
            .id(SharedString::from(id.clone()))
            .debug_selector(move || id.clone())
            .mx_1()
            .px_2()
            .py_1p5()
            .rounded_md()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .border_1()
            .border_color(rgba(0x00000000))
            .when(open, |el| el.bg(palette.open))
            .when(!open, |el| {
                el.hover(|style| style.border_color(palette.line))
            })
            .on_click(cx.listener(move |this, _, window, cx| this.select(to.clone(), window, cx)))
    }

    /// Whether the depot's tree row stands open: its board, its Line
    /// window, or a travelling bead opened from either.
    fn depot_open(model: &Model, depot: &str) -> bool {
        let back = Selected::Line(depot.to_string());
        model.selected() == Some(&back)
            || matches!(model.selected(), Some(Selected::LineWindow(name)) if name == depot)
            || matches!(model.selected(), Some(Selected::Bead(_))) && model.bead_back() == back
    }

    /// Whether the flow's tree row stands open: its list, or a
    /// travelling bead opened from it.
    fn flow_open(model: &Model, depot: &str, name: &str, flow_type: &str) -> bool {
        let back = Selected::Flow(depot.to_string(), name.to_string(), flow_type.to_string());
        model.selected() == Some(&back)
            || matches!(model.selected(), Some(Selected::Bead(_))) && model.bead_back() == back
    }

    /// A flow's name, disambiguated where its depot serves that name
    /// for more than one type. An empty type is the depot's own flow.
    fn flow_label(depot: &FlowDepot, flow: &Flow) -> String {
        let same_name = depot
            .flows
            .iter()
            .filter(|other| other.name == flow.name)
            .count()
            > 1;
        if !same_name {
            return flow.name.clone();
        }
        let kind = match flow.flow_type.is_empty() {
            true => "depot flow".to_string(),
            false => format!("type {}", flow.flow_type),
        };
        format!("{} · {kind}", flow.name)
    }

    /// A flow's element id includes its type: a depot can serve two
    /// flows with the same name.
    fn flow_element_id(kind: &str, depot: &FlowDepot, flow: &Flow) -> SharedString {
        SharedString::from(format!(
            "{kind} {:?} {:?} {:?}",
            depot.name, flow.name, flow.flow_type
        ))
    }

    /// The arrivals row of the tree: its name with the yard-wide
    /// travelling total at its right. It stands open for a travelling
    /// bead from the panel or the board too, which has no row of its own.
    pub(crate) fn arrivals_tree_row(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let travelling = model.travelling();
        let right = match travelling {
            0 => "none".to_string(),
            travelling => format!("{travelling} · all depots"),
        };
        self.tree_row(
            "row arrivals".to_string(),
            Selected::Arrivals,
            model.arrivals_row().selected,
            palette,
            cx,
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(rems(TYPE_LABEL))
                .font_weight(FontWeight::MEDIUM)
                .child("Arrivals"),
        )
        .child(
            div()
                .flex_none()
                .text_size(rems(TYPE_META))
                .text_color(palette.faint)
                .child(right),
        )
    }

    /// One depot's tree row: its lamp, its name, and at the right how
    /// many wait on the reader in rust, else its travelling total; under
    /// it its transit with its moving and quiet flows. It opens the
    /// depot's board, and stands open for its Line window too.
    pub(crate) fn depot_tree_row(
        &self,
        model: &Model,
        depot: &FlowDepot,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let on_you = model.depot_on_you(&depot.name);
        let (right, color) = match on_you {
            0 => (depot.travelling.to_string(), palette.faint),
            _ => (on_you_words(on_you), palette.text_accent),
        };
        let mut sub = format!("{} in transit", depot.travelling);
        if let Some(flows) = moving_quiet_words(
            Model::moving_flows(depot).len(),
            Model::quiet_flows(depot).len(),
        ) {
            sub.push_str(&format!(" · {flows}"));
        }
        let lamp = lamp_dot(
            depot_lamp(depot),
            format!("row lamp {}", depot.name),
            palette,
        );
        self.tree_row(
            format!("row {}", depot.name),
            Selected::Line(depot.name.clone()),
            Self::depot_open(model, &depot.name),
            palette,
            cx,
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .children(lamp)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(rems(TYPE_LABEL))
                                .font_weight(FontWeight::MEDIUM)
                                .child(depot.name.clone()),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(rems(TYPE_META))
                                .text_color(color)
                                .child(right),
                        ),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child(sub),
                ),
        )
    }

    /// The depot's flows nested under its tree row: each moving flow
    /// with its lamp and its count, then the quiet fold with the quiet
    /// flows under it where it stands open. Nothing where the depot
    /// governs no flow.
    pub(crate) fn depot_flow_rows(
        &self,
        model: &Model,
        depot: &FlowDepot,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let moving = Model::moving_flows(depot);
        let quiet = Model::quiet_flows(depot);
        let mut rows = div()
            .ml_7()
            .mr_1()
            .mb_1()
            .pl_2()
            .border_l_1()
            .border_color(palette.line)
            .flex()
            .flex_col();
        for flow in moving {
            rows = rows.child(self.flow_tree_row(model, depot, flow, false, palette, cx));
        }
        if !quiet.is_empty() {
            let open = model.quiet_flows_expanded(&depot.name);
            let name = depot.name.clone();
            rows = rows.child(
                div()
                    .id(SharedString::from(format!("quiet flows {name}")))
                    .debug_selector({
                        let name = name.clone();
                        move || format!("quiet flows {name}")
                    })
                    .h_7()
                    .px_2()
                    .rounded_md()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|style| style.bg(palette.hover))
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_quiet_flows(&name, cx)))
                    .child(fold_chevron(open, palette.faint))
                    .child(
                        div()
                            .size(px(8.))
                            .rounded_full()
                            .flex_none()
                            .bg(palette.strong),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child(match quiet.len() {
                                1 => "1 quiet flow".to_string(),
                                quiet => format!("{quiet} quiet flows"),
                            }),
                    ),
            );
            if open {
                for flow in quiet {
                    rows = rows.child(self.flow_tree_row(model, depot, flow, true, palette, cx));
                }
            }
        }
        rows
    }

    /// One flow's tree row under its depot's: its lamp, its name, and at
    /// its right how many wait on the reader in rust where any do, else
    /// what is worked, else its travelling total — a quiet flow's
    /// backlog instead. It opens the flow's list.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn flow_tree_row(
        &self,
        model: &Model,
        depot: &FlowDepot,
        flow: &Flow,
        quiet: bool,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let on_you = model.flow_on_you(&depot.name, &flow.name, &flow.flow_type);
        let counts = line_counts(flow);
        let label = Self::flow_label(depot, flow);
        let (right, color) = match (quiet, on_you, counts.working) {
            (_, on_you, _) if on_you > 0 => (on_you.to_string(), palette.text_accent),
            (_, _, working) if working > 0 => (format!("{working} working"), palette.faint),
            (true, _, _) => match counts.backlog {
                0 => (String::new(), palette.faint),
                backlog => (format!("{backlog} backlog"), palette.faint),
            },
            (false, _, _) => (Model::flow_travelling(flow).to_string(), palette.faint),
        };
        let lamp = lamp_dot(
            counts.lamp,
            format!("flow lamp {} {label}", depot.name),
            palette,
        );
        let open = Self::flow_open(model, &depot.name, &flow.name, &flow.flow_type);
        let to = Selected::Flow(
            depot.name.clone(),
            flow.name.clone(),
            flow.flow_type.clone(),
        );
        div()
            .id(Self::flow_element_id("flow row", depot, flow))
            .debug_selector({
                let (depot, label) = (depot.name.clone(), label.clone());
                move || format!("flow row {depot} {label}")
            })
            .h_7()
            .px_2()
            .rounded_md()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .when(open, |el| el.bg(palette.open))
            .when(!open, |el| el.hover(|style| style.bg(palette.hover)))
            .on_click(cx.listener(move |this, _, window, cx| this.select(to.clone(), window, cx)))
            .children(lamp.or_else(|| {
                Some(
                    div()
                        .size(px(8.))
                        .rounded_full()
                        .flex_none()
                        .bg(palette.strong),
                )
            }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(rems(TYPE_META))
                    .child(label),
            )
            .when(!right.is_empty(), |el| {
                el.child(
                    div()
                        .flex_none()
                        .text_size(rems(TYPE_META))
                        .text_color(color)
                        .child(right),
                )
            })
    }

    /// The No flow row of the tree: the beads travelling with no flow,
    /// with their count and that they have no stages. It stands last,
    /// and only while any travel.
    pub(crate) fn no_flow_tree_row(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let travelling = model.unassigned().len();
        self.tree_row(
            format!("row {}", model::NO_FLOW),
            Selected::NoLine,
            model.no_line_row().is_some_and(|row| row.selected),
            palette,
            cx,
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(rems(TYPE_LABEL))
                                .font_weight(FontWeight::MEDIUM)
                                .child(model::NO_FLOW),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(rems(TYPE_META))
                                .text_color(palette.faint)
                                .child(travelling.to_string()),
                        ),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child("beads moving with no stages"),
                ),
        )
    }

    /// One quiet depot's tree row under the Quiet depots fold: its name
    /// with its backlog where any waits, else that nothing is in
    /// transit. It opens the depot's board.
    pub(crate) fn quiet_tree_row(
        &self,
        model: &Model,
        depot: &FlowDepot,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let sub = match Model::depot_backlog(depot) {
            0 => "nothing in transit".to_string(),
            backlog => format!("{backlog} backlog"),
        };
        self.tree_row(
            format!("row {}", depot.name),
            Selected::Line(depot.name.clone()),
            Self::depot_open(model, &depot.name),
            palette,
            cx,
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .size(px(8.))
                                .rounded_full()
                                .flex_none()
                                .bg(palette.strong),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(rems(TYPE_LABEL))
                                .font_weight(FontWeight::MEDIUM)
                                .child(depot.name.clone()),
                        ),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child(sub),
                ),
        )
    }

    /// One row of the departures board: the expected arrival in
    /// mono, the bead's title with its headcode faint after it, the
    /// platform as a plain word, the line, and the remark last in its
    /// tone. The rows stand flat in the served arrival order, a late
    /// mark as words among the remarks. A click opens the details.
    /// Hovering a worked row says which session works it, from the
    /// bead's flows row, which is where the yard says so. Where the
    /// window is the operator's a worked row carries Screen, which opens
    /// that session's screen over the window instead of the details.
    pub(crate) fn arrival_row(
        &self,
        model: &Model,
        row: &Arrival,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let bead = row.bead.clone();
        let sel = Selected::Bead(row.bead.clone());
        let remark = you_first(&row.remark, model.me());
        let color = remark_color(&row.remark, &row.tone, &row.held_by, model.me(), palette);
        let tip = model
            .flow_bead(&row.bead)
            .and_then(|(found, _)| worker_words(found).map(|(tip, _)| tip));
        let mut expected = grown(BOARD_EXPECTED)
            .debug_selector({
                let bead = bead.clone();
                move || format!("arrival expected {bead}")
            })
            .flex_none()
            .truncate()
            .text_xs()
            .text_color(match row.expected.as_str() {
                "—" => palette.faint,
                _ => palette.text,
            })
            .child(row.expected.clone());
        if let Some(mono) = mono.clone() {
            expected = expected.font_family(mono);
        }
        let mut headcode = div()
            .flex_none()
            .text_xs()
            .text_color(palette.faint)
            .child(short_bead(&row.bead));
        if let Some(mono) = mono {
            headcode = headcode.font_family(mono);
        }
        div()
            .id(SharedString::from(format!("arrival {bead}")))
            .debug_selector({
                let bead = bead.clone();
                move || format!("arrival {bead}")
            })
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .py_1p5()
            .border_b_1()
            .border_color(palette.line)
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.select(sel.clone(), window, cx)))
            .when_some(tip, |el, tip| {
                el.tooltip(move |_, cx| cx.new(|_| RailTip(tip.clone())).into())
            })
            .child(expected)
            .child(
                grown(BOARD_BEAD)
                    .debug_selector({
                        let bead = bead.clone();
                        move || format!("arrival bead {bead}")
                    })
                    .flex_none()
                    .flex()
                    .flex_row()
                    .items_baseline()
                    .gap_1()
                    .child(
                        div()
                            .flex_shrink()
                            .min_w_0()
                            .truncate()
                            .debug_selector({
                                let bead = bead.clone();
                                move || format!("arrival title {bead}")
                            })
                            .child(row.title.clone()),
                    )
                    .child(headcode),
            )
            .child(
                grown(BOARD_PLATFORM)
                    .flex_none()
                    .truncate()
                    .debug_selector({
                        let bead = bead.clone();
                        move || format!("arrival platform {bead}")
                    })
                    .child(row.platform.clone()),
            )
            .child(
                grown(BOARD_LINE)
                    .flex_none()
                    .truncate()
                    .debug_selector({
                        let bead = bead.clone();
                        move || format!("arrival line {bead}")
                    })
                    .child(match row.line.is_empty() {
                        true => "no flow".to_string(),
                        false => row.line.clone(),
                    }),
            )
            .child(
                grown(BOARD_REMARK)
                    .flex_none()
                    .truncate()
                    .text_color(color)
                    .when(!remark.is_empty(), |el| {
                        el.debug_selector({
                            let bead = bead.clone();
                            move || format!("arrival remark {bead}")
                        })
                    })
                    .child(remark),
            )
            .children(model.screen_for_bead(&row.bead).map(|session| {
                screen_control(session, format!("arrival screen {bead}"), palette, cx)
            }))
    }

    /// What the arrivals pane's tooltip says, in one line.
    pub(crate) const ARRIVALS_TIP: &str = "arrivals: the panel of beads still travelling";

    /// The arrivals pane: the panel of tracks, or the departures
    /// board under the head's toggle. The head carries the travelling
    /// words, the panel's summary where it stands, and the toggle.
    pub(crate) fn talk_arrivals(
        &self,
        model: &Model,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pane = div().flex_1().min_w_0().h_full().flex().flex_col();
        let head = self.arrivals_head(model, palette, cx);
        let body = match model.departures() {
            true => self.departures_board(model, palette, cx).into_any_element(),
            false => self
                .panel(model, pane_width, rem, palette, cx)
                .into_any_element(),
        };
        pane.children(self.reconnect_strip(palette))
            .child(head)
            .child(body)
            .when(!model.notice.is_empty(), |el| {
                el.child(
                    div()
                        .px_4()
                        .py_1()
                        .text_sm()
                        .text_color(palette.warn)
                        .child(format!("! {}", model.notice)),
                )
            })
    }

    /// The arrivals pane's head: the title with the Panel/Departures
    /// toggle at its right, the travelling words under it, and the
    /// head's stats where it stands on the panel. The stats stand
    /// full-width under the title's row, wrapping where they cannot
    /// hold one line down to the floor.
    pub(crate) fn arrivals_head(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let travelling = model.travelling();
        let sub = match travelling {
            0 => "none in transit".to_string(),
            travelling => format!("{travelling} in transit across all flows"),
        };
        div()
            .px_4()
            .py_2p5()
            .border_b_1()
            .border_color(palette.line)
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .id("arrivals head")
                                    .debug_selector(|| "arrivals head".to_string())
                                    .text_size(rems(TYPE_TITLE))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .tooltip(|_, cx| {
                                        cx.new(|_| RailTip(Self::ARRIVALS_TIP.to_string())).into()
                                    })
                                    .child("Arrivals"),
                            )
                            .child(
                                div()
                                    .text_size(rems(TYPE_META))
                                    .text_color(palette.faint)
                                    .child(sub),
                            ),
                    )
                    .child(self.arrivals_toggle(model, palette, cx)),
            )
            .children(self.panel_stats(model, palette, cx))
    }

    /// The head's Panel/Departures toggle: the kit's segmented control,
    /// each half opening its view.
    pub(crate) fn arrivals_toggle(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let departures = model.departures();
        div().flex_none().child(segmented(
            [
                ("Panel", false, !departures),
                ("Departures", true, departures),
            ]
            .into_iter()
            .map(|(word, to, open)| {
                segment(word.to_string(), open, palette)
                    .id(SharedString::from(format!(
                        "arrivals {}",
                        word.to_lowercase()
                    )))
                    .debug_selector(move || format!("arrivals {}", word.to_lowercase()))
                    .on_click(cx.listener(move |this, _, _, cx| this.set_departures(to, cx)))
            })
            .collect(),
            palette,
        ))
    }

    /// The head's stats over the panel: what is worked and what waits
    /// on the reader to decide and to move, each a pill where any
    /// stands. The worked pill splits by served kind
    /// where two or more kinds work, and hovering it lists each working
    /// group with its seats; the waiting pills open the decisions mode,
    /// as the board's banner does. Never on the departures board, whose
    /// banner counts the waits.
    pub(crate) fn panel_stats(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        if model.departures() {
            return None;
        }
        let working = model.working_count();
        let (accent, quiet) =
            lines_waits_words(model, |waiting| format!("{waiting} waiting on you"));
        if working == 0 && accent.is_none() && quiet.is_none() {
            return None;
        }
        let mut stats = div()
            .debug_selector(|| "panel stats".to_string())
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_1p5();
        if working > 0 {
            let split = model.working_split();
            stats = stats.child(
                Self::stat_pill(
                    "stat working",
                    working_words(working, &split.kinds),
                    false,
                    palette,
                )
                .children(lamp_dot("moving", "stat working dot".to_string(), palette))
                .when_some(working_tip(&split.groups), |el, tip| {
                    el.tooltip(move |_, cx| cx.new(|_| RailTip(tip.clone())).into())
                }),
            );
        }
        if let Some(words) = accent {
            stats = stats.child(
                Self::stat_pill("stat decide", words, true, palette).on_click(
                    cx.listener(|this, _, window, cx| this.set_mode(Mode::Decisions, window, cx)),
                ),
            );
        }
        if let Some(words) = quiet {
            stats = stats.child(
                Self::stat_pill("stat move", words, false, palette)
                    .cursor_pointer()
                    .hover(|style| style.bg(palette.hover))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.set_mode(Mode::Decisions, window, cx)
                    })),
            );
        }
        Some(stats)
    }

    /// One head stat: the numeral semibold with its words, in a pill
    /// with the strong edge — the accent edge with rust words where the
    /// stat waits on the reader. The caller gives a pill that opens
    /// Decide its press, a quiet one its pointer too.
    fn stat_pill(
        id: &'static str,
        words: String,
        you: bool,
        palette: Palette,
    ) -> gpui::Stateful<gpui::Div> {
        let (count, rest) = match words.split_once(' ') {
            Some((count, rest)) => (count.to_string(), rest.to_string()),
            None => (words, String::new()),
        };
        let mut pill = div()
            .id(id)
            .debug_selector(move || id.to_string())
            .h_7()
            .px_2p5()
            .rounded_full()
            .border_1()
            .flex()
            .flex_row()
            .items_center()
            .gap_1p5()
            .text_size(rems(TYPE_META))
            .when(you, |el| {
                el.border_color(palette.accent)
                    .text_color(palette.text_accent)
                    .cursor_pointer()
                    .hover(|style| style.bg(palette.hover))
            })
            .when(!you, |el| {
                el.border_color(palette.strong).text_color(palette.faint)
            })
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(match you {
                        true => palette.text_accent,
                        false => palette.text,
                    })
                    .child(count),
            );
        if !rest.is_empty() {
            pill = pill.child(rest);
        }
        pill
    }

    /// The departures board under the toggle: the served rows flat
    /// in the served arrival order under their column head, with the
    /// waits banner and the cap's honest line.
    pub(crate) fn departures_board(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mono = select::mono(cx);
        let mut list = div()
            .id("arrivals")
            .flex_1()
            .min_h_0()
            .overflow_x_scroll()
            .overflow_y_scroll()
            .track_scroll(&self.decision)
            .p_4()
            .flex()
            .flex_col()
            .gap_3();
        let (accent, quiet) = lines_waits_words(model, waits_head);
        if accent.is_some() || quiet.is_some() {
            list = list.child(self.waits_banner(accent, quiet, palette, cx));
        }
        let rows = model.arrival_rows();
        if rows.is_empty() {
            list = list.child(match model.arrivals_too_old() {
                true => lines_too_old(palette),
                false => div().text_color(palette.faint).child("None in transit."),
            });
        } else {
            list = list.child(self.departures_head(palette));
            for row in rows {
                list = list.child(self.arrival_row(model, row, mono.clone(), palette, cx));
            }
        }
        // The board caps at 15 rows; the total is complete.
        let travelling = model.travelling();
        let shown = model.arrival_rows().len();
        if travelling > shown {
            list = list.child(
                div()
                    .debug_selector(|| "arrivals capped".to_string())
                    .text_sm()
                    .text_color(palette.faint)
                    .child(format!("{shown} of {travelling} shown")),
            );
        }
        list
    }

    /// The departures board's column head: Expected, Bead, Platform,
    /// Line and Remark over the rows' columns.
    pub(crate) fn departures_head(&self, palette: Palette) -> impl IntoElement + use<> {
        let cell = |words: &'static str, width: f32| {
            grown(width)
                .flex_none()
                .truncate()
                .text_xs()
                .text_color(palette.faint)
                .child(words)
        };
        div()
            .debug_selector(|| "departures head".to_string())
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .pb_1p5()
            .border_b_1()
            .border_color(palette.line)
            .child(cell("Expected", BOARD_EXPECTED))
            .child(cell("Bead", BOARD_BEAD))
            .child(cell("Platform", BOARD_PLATFORM))
            .child(cell("Line", BOARD_LINE))
            .child(cell("Remark", BOARD_REMARK))
    }

    /// The arrivals panel: the legend, then one section per depot
    /// with beads in transit — each moving flow a track of stations
    /// with its beads' chips on them, each quiet flow one line of its
    /// path — and the beads with no flow in their own section. The
    /// panel scrolls down over the sections; a track scrolls sideways
    /// past the pane's edge.
    pub(crate) fn panel(
        &self,
        model: &Model,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mono = select::mono(cx);
        let mut body = div()
            .id("panel")
            .debug_selector(|| "panel".to_string())
            .flex_1()
            .min_h_0()
            .min_w_0()
            .overflow_y_scroll()
            .track_scroll(&self.decision)
            .px_4()
            .py_3()
            .flex()
            .flex_col()
            .gap_4();
        let mut empty = true;
        for depot in model
            .depots()
            .iter()
            .filter(|depot| depot.travelling > 0 || !depot.error.is_empty())
        {
            if empty {
                empty = false;
                body = body.child(self.panel_legend(palette));
            }
            body = body.child(self.panel_depot(
                model,
                depot,
                mono.clone(),
                pane_width,
                rem,
                palette,
                cx,
            ));
        }
        if !model.unassigned().is_empty() {
            empty = false;
            body = body.child(self.panel_loose(model, palette, cx));
        }
        if empty {
            body = body.child(match model.flows_too_old() {
                true => lines_too_old(palette),
                false => div().text_color(palette.faint).child("None in transit."),
            });
        }
        body
    }

    /// What the tracks' nodes say, each with its word: a crew stage, a
    /// people stage, crew working, waiting on the reader, the end of
    /// the flow.
    pub(crate) fn panel_legend(&self, palette: Palette) -> impl IntoElement + use<> {
        let mut legend = div()
            .debug_selector(|| "panel legend".to_string())
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_x_4()
            .gap_y_1();
        for (node, words) in [
            ("crew", "crew stage"),
            ("people", "people stage"),
            ("work", "crew working"),
            ("you", "waiting on you"),
            ("end", "end of the flow"),
        ] {
            legend = legend.child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1p5()
                    .child(Self::panel_node(node, palette))
                    .child(
                        div()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child(words),
                    ),
            );
        }
        legend
    }

    /// One depot's section on the panel: its head with its transit and
    /// its flows, its error where its flows could not be read, one
    /// track per moving flow in the served order, its quiet flows as
    /// one-liners, and its Elsewhere beads' line where beads stand in
    /// no stage of its flows. The head opens the depot's board. The
    /// rows never shrink: squeezed, their cards would spill over the
    /// next row, so the panel scrolls instead.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn panel_depot(
        &self,
        model: &Model,
        depot: &FlowDepot,
        mono: Option<SharedString>,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut sub = match depot.travelling {
            0 => "nothing in transit".to_string(),
            travelling => format!("{travelling} in transit"),
        };
        let flows = flow_words(depot.flows.len());
        if !flows.is_empty() {
            sub.push_str(&format!(" · {flows}"));
        }
        let to = Selected::Line(depot.name.clone());
        let mut section = div().flex().flex_col().flex_none().gap_2().child(
            div()
                .id(SharedString::from(format!("panel depot {}", depot.name)))
                .debug_selector({
                    let depot = depot.name.clone();
                    move || format!("panel depot {depot}")
                })
                .flex()
                .flex_row()
                .items_baseline()
                .gap_2()
                .cursor_pointer()
                .hover(|style| style.bg(palette.hover))
                .on_click(
                    cx.listener(move |this, _, window, cx| this.select(to.clone(), window, cx)),
                )
                .child(
                    div()
                        .text_size(rems(TYPE_SECTION))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(depot.name.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child(sub),
                ),
        );
        if !depot.error.is_empty() {
            section = section.child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .child(depot.error.clone()),
            );
        }
        for flow in Model::moving_flows(depot) {
            section = section.child(self.panel_track(
                model,
                depot,
                flow,
                mono.clone(),
                pane_width,
                rem,
                palette,
                cx,
            ));
        }
        let quiet = Model::quiet_flows(depot);
        if !quiet.is_empty() {
            section =
                section.child(self.panel_quiet_flows(depot, quiet, pane_width, rem, palette, cx));
        }
        if !depot.elsewhere.is_empty() {
            section = section.child(self.panel_elsewhere(depot, palette, cx));
        }
        section
    }

    /// One moving flow's track, a card of its own: its head above a
    /// station per stage but the backlog's, left to right in travel
    /// order, with the beads' chips on their stations. The head opens
    /// the flow's list; the card reads rust-edged where any of its
    /// beads waits on the reader.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn panel_track(
        &self,
        model: &Model,
        depot: &FlowDepot,
        flow: &Flow,
        mono: Option<SharedString>,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let on_you = model.flow_on_you(&depot.name, &flow.name, &flow.flow_type);
        let selector = format!("{} {}", depot.name, Self::flow_label(depot, flow));
        let mut stations = div()
            .id(Self::flow_element_id("panel stations", depot, flow))
            .flex()
            .flex_row()
            .w_full()
            .min_w_0()
            .overflow_x_scroll();
        let stages: Vec<&FlowStage> = flow
            .stages
            .iter()
            .filter(|stage| stage.stage != "backlog")
            .collect();
        let last = stages.len().saturating_sub(1);
        for (at, stage) in stages.iter().enumerate() {
            stations = stations.child(self.panel_station(
                model,
                depot,
                flow,
                stage,
                at == 0,
                at == last,
                mono.clone(),
                palette,
                cx,
            ));
        }
        panel_card(palette)
            .id(Self::flow_element_id("panel track", depot, flow))
            .debug_selector({
                let selector = selector.clone();
                move || format!("panel track {selector}")
            })
            .when(on_you > 0, |el| el.border_color(palette.accent))
            .child(self.panel_track_head(depot, flow, on_you, pane_width, rem, palette, cx))
            .child(stations)
    }

    /// One track's head above its stations: the flow's name with its
    /// type where typed, else its source, and at the right who the flow
    /// waits on with its backlog counted apart. It opens the flow's
    /// list. The head holds one line: the name ends in an ellipsis past
    /// its budget, and the chip with the meta stands whole at the right.
    /// Where the budget with the chip falls below the name's floor, the
    /// chip yields instead, and the name is budgeted without it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn panel_track_head(
        &self,
        depot: &FlowDepot,
        flow: &Flow,
        on_you: usize,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let chip = match flow.flow_type.is_empty() {
            false => format!("type {}", flow.flow_type),
            true => flow.source.clone(),
        };
        let (status, tone) = flow_status(flow, on_you);
        let color = match tone {
            "you" => palette.text_accent,
            "working" => palette.ok,
            _ => palette.text,
        };
        let counts = line_counts(flow);
        let backlog = format!("· {} backlog", counts.backlog);
        let name_len = flow.name.chars().count();
        let status_len = status.chars().count();
        let backlog_len = backlog.chars().count();
        let mut budget = panel_head_name_budget(
            pane_width,
            rem,
            name_len,
            chip.chars().count(),
            status_len,
            backlog_len,
        );
        // The name identifies where the chip classifies: where the budget
        // with the chip falls below the floor and the name is cut, the
        // chip yields and the name is budgeted without it. A short name
        // already whole keeps its chip.
        let mut show_chip = !chip.is_empty();
        if show_chip && budget < PANEL_HEAD_NAME_FLOOR && budget < name_len {
            budget = panel_head_name_budget(pane_width, rem, name_len, 0, status_len, backlog_len);
            show_chip = false;
        }
        let name = shorten(&flow.name, budget);
        let selector = format!("{} {}", depot.name, Self::flow_label(depot, flow));
        let to = Selected::Flow(
            depot.name.clone(),
            flow.name.clone(),
            flow.flow_type.clone(),
        );
        div()
            .id(Self::flow_element_id("panel head", depot, flow))
            .debug_selector({
                let selector = selector.clone();
                move || format!("panel head {selector}")
            })
            .w_full()
            .min_w_0()
            .flex()
            .flex_row()
            .items_baseline()
            .gap_2()
            .pb_3()
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.select(to.clone(), window, cx)))
            .child(
                div()
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("panel head {selector} name")
                    })
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(rems(TYPE_LABEL))
                    .font_weight(FontWeight::MEDIUM)
                    .child(name),
            )
            .when(show_chip, |el| {
                el.child(
                    div()
                        .debug_selector({
                            let selector = selector.clone();
                            move || format!("panel head {selector} chip")
                        })
                        .flex_none()
                        .rounded(px(4.))
                        .border_1()
                        .border_color(palette.line)
                        .px_1()
                        .text_size(rems(TYPE_CODE))
                        .text_color(palette.faint)
                        .child({
                            let mut chip = div().child(chip);
                            if let Some(mono) = select::mono(cx) {
                                chip = chip.font_family(mono);
                            }
                            chip
                        }),
                )
            })
            .child(
                div()
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("panel head {selector} status")
                    })
                    .flex_none()
                    .text_size(rems(TYPE_META))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(color)
                    .child(status),
            )
            .child(
                div()
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("panel head {selector} backlog")
                    })
                    .flex_none()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .child(backlog),
            )
    }

    /// One stage's station on the track: the node on the track's line
    /// with its name under it, and the beads' chips under that. The
    /// line runs through the node row, stopping at the track's ends;
    /// the stations share the card's width down to their narrowest,
    /// past which the track scrolls. A press on the node or the name
    /// opens the stage's context drawer; the chips keep their own taps.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn panel_station(
        &self,
        model: &Model,
        depot: &FlowDepot,
        flow: &Flow,
        stage: &FlowStage,
        first: bool,
        last: bool,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let lit = !stage.beads.is_empty();
        let selector = format!("{} {}", depot.name, Self::flow_label(depot, flow));
        let open = self.context.as_ref().is_some_and(|open| {
            open.stands_on(&depot.name, &flow.name, &flow.flow_type, &stage.stage)
        });
        let (press_depot, press_flow, press_type, press_stage) = (
            depot.name.clone(),
            flow.name.clone(),
            flow.flow_type.clone(),
            stage.stage.clone(),
        );
        let press_selector = format!("panel station {selector} {}", stage.stage);
        // The node and the name stand in the press that opens the
        // stage's context drawer; the chips stand past it, with their
        // own taps.
        let press = div()
            .id(SharedString::from(format!("{press_selector} open")))
            .debug_selector({
                let press_selector = press_selector.clone();
                move || format!("{press_selector} open")
            })
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap_1p5()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_context(
                    press_depot.clone(),
                    press_flow.clone(),
                    press_type.clone(),
                    press_stage.clone(),
                    cx,
                )
            }))
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .child(div().h(px(2.)).flex_1().rounded_full().bg(match first {
                        true => rgba(0x00000000),
                        false => palette.strong,
                    }))
                    .child(
                        Self::panel_node(station_node(stage), palette).debug_selector({
                            let (selector, stage, node) = (
                                selector.clone(),
                                stage.stage.clone(),
                                station_node(stage).to_string(),
                            );
                            move || format!("panel node {selector} {stage} {node}")
                        }),
                    )
                    .child(div().h(px(2.)).flex_1().rounded_full().bg(match last {
                        true => rgba(0x00000000),
                        false => palette.strong,
                    })),
            )
            .child(
                div()
                    .debug_selector({
                        let (selector, stage) = (selector.clone(), stage.stage.clone());
                        move || format!("panel label {selector} {stage}")
                    })
                    .text_size(rems(TYPE_META))
                    .text_color(match lit || open {
                        true => palette.text,
                        false => palette.faint,
                    })
                    .when(lit || open, |el| el.font_weight(FontWeight::MEDIUM))
                    .text_center()
                    .child(panel_stage_name(&stage.stage, false)),
            );
        // The stations share the card's width down to their narrowest,
        // past which the track scrolls: `grown`'s width, inlined, since
        // a station is no column of its own.
        let station = div()
            .debug_selector({
                let (selector, stage) = (selector.clone(), stage.stage.clone());
                move || format!("panel station {selector} {stage}")
            })
            .flex_1()
            .w(rems(TRACK_STATION / 16.))
            .min_w(px(TRACK_STATION))
            .flex()
            .flex_col()
            .items_center()
            .gap_1p5()
            .child(press);
        // Full width, so the chips wrap where the station ends and
        // nowhere sooner: a row that hugs its content wraps early.
        let mut chips = div()
            .w_full()
            .flex()
            .flex_row()
            .flex_wrap()
            .justify_center()
            .gap_1();
        let on_you = model.on_you_beads();
        let me = model.me();
        match worked_first(&stage.beads).as_slice() {
            [] => {}
            [one] => {
                chips = chips.child(self.panel_chip(one, &on_you, me, mono, palette, cx));
            }
            [one, two] => {
                chips = chips.child(self.panel_chip(one, &on_you, me, mono.clone(), palette, cx));
                chips = chips.child(self.panel_chip(two, &on_you, me, mono, palette, cx));
            }
            [one, hidden @ ..] => {
                chips = chips.child(self.panel_chip(one, &on_you, me, mono, palette, cx));
                chips = chips.child(
                    div()
                        .debug_selector({
                            let (selector, stage) = (selector.clone(), stage.stage.clone());
                            move || format!("panel more {selector} {stage}")
                        })
                        .h(px(20.))
                        .px_1p5()
                        .rounded_full()
                        .border_1()
                        .border_color(palette.strong)
                        .bg(palette.ground)
                        .text_size(rems(TYPE_CODE))
                        .text_color(palette.text)
                        .flex()
                        .flex_row()
                        .items_center()
                        .child(format!("+{}", hidden.len())),
                );
            }
        }
        station.child(chips)
    }

    /// A station's node: the plain crew dot, the hollow people ring,
    /// the working ring, the rust fill waiting on the reader, or the
    /// end square. The rings are one dot inside another, since the kit
    /// draws no border widths of its own.
    fn panel_node(node: &str, palette: Palette) -> gpui::Div {
        let dot = |fill: gpui::Rgba| {
            div()
                .size(px(14.))
                .rounded_full()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .bg(fill)
        };
        match node {
            "people" => dot(palette.faint).child(
                div()
                    .size(px(10.))
                    .rounded_full()
                    .flex_none()
                    .bg(palette.raised),
            ),
            "work" => dot(palette.ok).child(
                div()
                    .size(px(8.))
                    .rounded_full()
                    .flex_none()
                    .bg(palette.raised),
            ),
            "you" => dot(palette.accent),
            "end" => div()
                .size(px(14.))
                .rounded_sm()
                .flex_none()
                .bg(palette.faint),
            _ => dot(palette.strong),
        }
    }

    /// One bead's chip on its station: the short handle in mono,
    /// opening the bead. A bead waiting on the reader reads rust
    /// filled; a worked bead reads semibold; a held or stalled bead
    /// reads red on the plain chip. Hovering a worked chip says which
    /// session works it, with its group, kind and model.
    pub(crate) fn panel_chip(
        &self,
        bead: &FlowBead,
        asked: &std::collections::HashSet<&str>,
        me: &str,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let sel = Selected::Bead(bead.bead.clone());
        let name = bead.bead.clone();
        let tip = worker_words(bead).map(|(tip, _)| tip);
        let you = chip_you(bead, me, asked);
        let color = match you {
            true => palette.on_accent,
            false => headcode_color(
                bead.working,
                &bead.remark,
                &bead.tone,
                &bead.held_by,
                me,
                palette,
            ),
        };
        let mut text = div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(rems(TYPE_CODE))
            .text_color(color)
            .when(bead.working, |el| el.font_weight(FontWeight::SEMIBOLD))
            .child(short_bead(&bead.bead));
        if let Some(mono) = mono {
            text = text.font_family(mono);
        }
        div()
            .id(SharedString::from(format!("panel chip {name}")))
            .debug_selector({
                let name = name.clone();
                move || format!("panel chip {name}")
            })
            .h(px(20.))
            .px_1p5()
            .rounded_full()
            .border_1()
            .border_color(match you {
                true => palette.accent,
                false => palette.strong,
            })
            .bg(match you {
                true => palette.accent,
                false => palette.ground,
            })
            .flex()
            .flex_row()
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| this.select(sel.clone(), window, cx)))
            .when_some(tip, |el, tip| {
                el.tooltip(move |_, cx| cx.new(|_| RailTip(tip.clone())).into())
            })
            .child(text)
    }

    /// The depot's quiet flows as one line each: the name, the path
    /// of its stages, and its backlog. Each opens the flow's list. The
    /// path ends in an ellipsis past its budget, as the name does.
    pub(crate) fn panel_quiet_flows(
        &self,
        depot: &FlowDepot,
        quiet: Vec<&Flow>,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut rows = div()
            .debug_selector({
                let depot = depot.name.clone();
                move || format!("panel quiet flows {depot}")
            })
            .w_full()
            .min_w_0()
            .flex_none()
            .flex()
            .flex_col()
            .rounded_lg()
            .border_1()
            .border_color(palette.line)
            .bg(palette.raised)
            .overflow_hidden();
        for (at, flow) in quiet.iter().enumerate() {
            let label = Self::flow_label(depot, flow);
            let path = flow_path(flow);
            let backlog = format!("{} backlog", model::backlog_count(flow));
            let path = shorten(
                &path,
                flow_quiet_path_budget(
                    pane_width,
                    rem,
                    path.chars().count(),
                    backlog.chars().count(),
                ),
            );
            let to = Selected::Flow(
                depot.name.clone(),
                flow.name.clone(),
                flow.flow_type.clone(),
            );
            rows = rows.child(
                div()
                    .id(Self::flow_element_id("panel quiet", depot, flow))
                    .debug_selector({
                        let (depot, label) = (depot.name.clone(), label.clone());
                        move || format!("panel quiet {depot} {label}")
                    })
                    .px_3()
                    .py_2()
                    .flex()
                    .flex_row()
                    .items_baseline()
                    .gap_3()
                    .cursor_pointer()
                    .hover(|style| style.bg(palette.hover))
                    .when(at > 0, |el| el.border_t_1().border_color(palette.line))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.select(to.clone(), window, cx)),
                    )
                    .child(
                        grown(QUIET_NAME)
                            .flex_none()
                            .truncate()
                            .text_size(rems(TYPE_META))
                            .font_weight(FontWeight::MEDIUM)
                            .child(shorten(&label, QUIET_NAME_CHARS)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child(path),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child(backlog),
                    ),
            );
        }
        rows
    }

    /// The depot's Elsewhere beads' line: how many stand in no stage of
    /// its flows, opening the depot's board, where the board lists
    /// them. The boards draw no Elsewhere, but without the line beads
    /// the depot counts would stand nowhere on the panel.
    pub(crate) fn panel_elsewhere(
        &self,
        depot: &FlowDepot,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let to = Selected::Line(depot.name.clone());
        div()
            .id(SharedString::from(format!(
                "panel elsewhere {}",
                depot.name
            )))
            .debug_selector({
                let depot = depot.name.clone();
                move || format!("panel elsewhere {depot}")
            })
            .w_full()
            .min_w_0()
            .flex_none()
            .px_3()
            .py_2()
            .rounded_lg()
            .border_1()
            .border_color(palette.line)
            .bg(palette.raised)
            .flex()
            .flex_row()
            .items_baseline()
            .gap_3()
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.select(to.clone(), window, cx)))
            .child(
                grown(QUIET_NAME)
                    .flex_none()
                    .truncate()
                    .text_size(rems(TYPE_META))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Elsewhere"),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .child(match depot.elsewhere.len() {
                        1 => "1 bead stands in no stage of these flows".to_string(),
                        elsewhere => {
                            format!("{elsewhere} beads stand in no stage of these flows")
                        }
                    }),
            )
    }

    /// The beads travelling with no flow: their own section at the
    /// panel's foot, since a bead with no depot travels no flow and has
    /// no stages to stand between. Its line says how many, and opens
    /// their list.
    pub(crate) fn panel_loose(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let travelling = model.unassigned().len();
        let words = match travelling {
            1 => "1 bead is moving with no flow, so it has no stage to show.".to_string(),
            travelling => {
                format!(
                    "{travelling} beads are moving with no flow, so they have no stages to show."
                )
            }
        };
        div()
            .flex()
            .flex_col()
            .flex_none()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .debug_selector(|| "panel loose head".to_string())
                            .text_size(rems(TYPE_SECTION))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(model::NO_FLOW),
                    )
                    .child(
                        div()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child(format!("{travelling} in transit")),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "panel loose".to_string())
                    .px_3()
                    .py_2p5()
                    .rounded_lg()
                    .border_1()
                    .border_color(palette.strong)
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_baseline()
                    .gap_2()
                    .child(
                        div()
                            .text_size(rems(TYPE_BODY))
                            .text_color(palette.faint)
                            .child(words),
                    )
                    .child(
                        div()
                            .id("panel loose show")
                            .debug_selector(|| "panel loose show".to_string())
                            .text_size(rems(TYPE_META))
                            .text_color(palette.link)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.select(Selected::NoLine, window, cx)
                            }))
                            .child("Show them"),
                    ),
            )
    }

    /// The beads that wait on the person reading, above the table:
    /// the calls in the accent with the beads to move after them in
    /// quiet words, or the old yard's one accent line, as one link. A
    /// click opens the decisions mode, as Enter does with the focus on
    /// it.
    pub(crate) fn waits_banner(
        &self,
        accent: Option<String>,
        quiet: Option<String>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut banner = div()
            .id("waits banner")
            .debug_selector(|| "waits banner".to_string())
            .text_sm()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .cursor_pointer()
            .track_focus(&self.waits_focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, _| window.focus(&this.waits_focus)),
            )
            .on_click(
                cx.listener(|this, _, window, cx| this.set_mode(Mode::Decisions, window, cx)),
            );
        let both = accent.is_some() && quiet.is_some();
        if let Some(words) = accent {
            banner = banner.child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(palette.text_accent)
                    .child(words),
            );
        }
        if both {
            banner = banner.child(div().text_color(palette.faint).child("·"));
        }
        if let Some(words) = quiet {
            banner = banner.child(div().text_color(palette.faint).child(words));
        }
        banner
    }

    /// One line's flows: each flow a section with its stages in travel
    /// order as columns — the panel's track straightened — each column
    /// its head with the group words and one card per bead, and an
    /// Elsewhere column under the flows for the line's beads in no
    /// stage of them. A bead's click opens its details.
    pub(crate) fn talk_flow(
        &self,
        model: &Model,
        line: &str,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pane = div().flex_1().min_w_0().h_full().flex().flex_col();
        let Some(depot) = model.depot(line) else {
            return pane
                .items_center()
                .justify_center()
                .text_color(palette.faint)
                .child("The depot is gone.");
        };
        let head = div()
            .p_3()
            .border_b_1()
            .border_color(palette.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .debug_selector({
                        let line = line.to_string();
                        move || format!("flow head {line}")
                    })
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(line.to_string()),
            )
            .child(div().text_sm().text_color(palette.faint).child(format!(
                        "{} · {}",
                        model
                            .line_rows()
                            .iter()
                            .find(|row| row.name == line)
                            .map(|row| row.line.clone())
                            .unwrap_or_default(),
                        flow_words(depot.flows.len())
                    )))
            .when(model.line_offered(), |el| {
                let sel = Selected::LineWindow(line.to_string());
                el.child(
                    div().pt_2().child(
                        super::ui::small_button("Line window".to_string(), palette)
                            .id(SharedString::from(format!("line window open {line}")))
                            .debug_selector({
                                let line = line.to_string();
                                move || format!("line window open {line}")
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.select(sel.clone(), window, cx)
                            })),
                    ),
                )
            });
        let mono = select::mono(cx);
        let now = chrono::Utc::now();
        let mut list = div()
            .id("flow")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.decision)
            .p_4()
            .flex()
            .flex_col()
            .gap_3();
        // The list takes vertical moves alone: gpui hands a wheel move
        // to every scroller under the pointer and spends a move along
        // one axis on a scroller of the other, so the horizontal moves
        // meant for the flows' columns would scroll the list as well.
        list.style().restrict_scroll_to_axis = Some(true);
        if !depot.error.is_empty() {
            list = list.child(
                div()
                    .debug_selector({
                        let line = line.to_string();
                        move || format!("flow error {line}")
                    })
                    .text_sm()
                    .text_color(palette.faint)
                    .child(depot.error.clone()),
            );
        }
        for flow in &depot.flows {
            list = list.child(self.flow_section(flow, &now, mono.clone(), palette, cx));
        }
        if !depot.elsewhere.is_empty() {
            list = list.child(self.elsewhere_column(depot, &now, mono.clone(), palette, cx));
        }
        if depot.flows.is_empty() && depot.elsewhere.is_empty() {
            list = list.child(div().text_color(palette.faint).child("None in transit."));
        }
        pane.children(self.reconnect_strip(palette))
            .child(head)
            .child(list)
            .when(!model.notice.is_empty(), |el| {
                el.child(
                    div()
                        .px_4()
                        .py_1()
                        .text_sm()
                        .text_color(palette.warn)
                        .child(format!("! {}", model.notice)),
                )
            })
    }

    /// One flow's pane, opened from the tree: its crumbs with the
    /// List/Board toggle, then its list — the stages' strip, the
    /// grouping line, and its beads grouped by stage with the empty
    /// stages folded — or its board under the toggle, the depot board's
    /// columns for this flow alone.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn talk_flow_list(
        &self,
        model: &Model,
        depot: &str,
        name: &str,
        flow_type: &str,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pane = div().flex_1().min_w_0().h_full().flex().flex_col();
        let Some(flow) = model.flow(depot, name, flow_type) else {
            return pane
                .items_center()
                .justify_center()
                .text_color(palette.faint)
                .child("The flow is gone.");
        };
        let head = self.flow_list_head(model, depot, flow, palette, cx);
        let mono = select::mono(cx);
        let now = chrono::Utc::now();
        let mut list = div()
            .id("flow list")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.decision)
            .flex()
            .flex_col();
        match model.flow_board() {
            true => {
                list = list.child(
                    div()
                        .p_4()
                        .flex()
                        .flex_col()
                        .child(self.flow_section(flow, &now, mono, palette, cx)),
                );
            }
            false => {
                list = list
                    .child(self.flow_strip(depot, flow, pane_width, rem, palette, cx))
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .border_b_1()
                            .border_color(palette.line)
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child("Grouped by stage · moving a bead goes through a Decide card"),
                    );
                for stage in &flow.stages {
                    list = list.child(self.flow_group(
                        model,
                        flow,
                        stage,
                        &now,
                        mono.clone(),
                        pane_width,
                        rem,
                        palette,
                        cx,
                    ));
                }
                list = list
                    .child(self.flow_empty_fold(model, depot, flow, pane_width, rem, palette, cx));
            }
        }
        pane.children(self.reconnect_strip(palette))
            .child(head)
            .child(list)
            .when(!model.notice.is_empty(), |el| {
                el.child(
                    div()
                        .px_4()
                        .py_1()
                        .text_sm()
                        .text_color(palette.warn)
                        .child(format!("! {}", model.notice)),
                )
            })
    }

    /// The flow pane's head: the depot with the flow under it and the
    /// flow's source, with the List/Board toggle at the right. The
    /// depot opens the depot's board.
    pub(crate) fn flow_list_head(
        &self,
        model: &Model,
        depot: &str,
        flow: &Flow,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let board = model.flow_board();
        let to = Selected::Line(depot.to_string());
        div()
            .px_4()
            .py_2p5()
            .border_b_1()
            .border_color(palette.line)
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_baseline()
                    .gap_1p5()
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "flow depot {depot} {}",
                                flow.name
                            )))
                            .debug_selector({
                                let (depot, flow) = (depot.to_string(), flow.name.clone());
                                move || format!("flow depot {depot} {flow}")
                            })
                            .text_size(rems(TYPE_BODY))
                            .text_color(palette.link)
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.select(to.clone(), window, cx)
                            }))
                            .child(depot.to_string()),
                    )
                    .child(
                        div()
                            .text_size(rems(TYPE_BODY))
                            .text_color(palette.faint)
                            .child("›"),
                    )
                    .child(
                        div()
                            .debug_selector({
                                let (depot, flow) = (depot.to_string(), flow.name.clone());
                                move || format!("flow head {depot} {flow}")
                            })
                            .text_size(rems(TYPE_BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(flow.name.clone()),
                    )
                    .when(!flow.source.is_empty(), |el| {
                        let mut source = div()
                            .debug_selector({
                                let (depot, flow) = (depot.to_string(), flow.name.clone());
                                move || format!("flow source {depot} {flow}")
                            })
                            .text_size(rems(TYPE_CODE))
                            .text_color(palette.faint)
                            .child(flow.source.clone());
                        if let Some(mono) = select::mono(cx) {
                            source = source.font_family(mono);
                        }
                        el.child(source)
                    }),
            )
            .child(
                div().flex_none().child(segmented(
                    [("List", false, !board), ("Board", true, board)]
                        .into_iter()
                        .map(|(word, to, open)| {
                            segment(word.to_string(), open, palette)
                                .id(SharedString::from(format!("flow {}", word.to_lowercase())))
                                .debug_selector(move || format!("flow {}", word.to_lowercase()))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.set_flow_board(to, cx)),
                                )
                        })
                        .collect(),
                    palette,
                )),
            )
    }

    /// The flow's stages as a strip: each but the backlog's a knot on
    /// the track with its name, the stages with beads named with their
    /// count, the landed end with its served landing rows. It scrolls
    /// sideways past the pane's edge, where the track runs on a tie with
    /// a › stands at the edge to say so. A press on a knot opens the
    /// stage's context drawer, as a press on a panel station does.
    pub(crate) fn flow_strip(
        &self,
        depot: &str,
        flow: &Flow,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut strip = div()
            .id("flow stations")
            .debug_selector({
                let flow = flow.name.clone();
                move || format!("flow strip {flow}")
            })
            .flex_1()
            .min_w_0()
            .flex()
            .flex_row()
            .items_center()
            .overflow_x_scroll()
            .px_4()
            .py_3();
        let mut labels: Vec<usize> = Vec::new();
        let stages: Vec<&FlowStage> = flow
            .stages
            .iter()
            .filter(|stage| stage.stage != "backlog")
            .collect();
        for (at, stage) in stages.iter().enumerate() {
            if at > 0 {
                strip = strip.child(
                    div()
                        .w(px(20.))
                        .h(px(2.))
                        .mx_1p5()
                        .flex_none()
                        .bg(palette.strong),
                );
            }
            let mut label = stage.stage.clone();
            let mut on = false;
            if !stage.beads.is_empty() {
                label.push_str(&format!(" · {}", stage.beads.len()));
                on = true;
            }
            if Self::flow_has_landings_at(flow, &stage.stage) {
                on = true;
            }
            labels.push(label.chars().count());
            let knot = match station_node(stage) {
                "end" => div()
                    .size(px(10.))
                    .rounded_sm()
                    .flex_none()
                    .bg(palette.faint),
                "work" => div()
                    .size(px(10.))
                    .rounded_full()
                    .flex_none()
                    .border_1()
                    .border_color(palette.ok),
                "people" | "you" if stage.human => div()
                    .size(px(10.))
                    .rounded_full()
                    .flex_none()
                    .border_1()
                    .border_color(palette.faint),
                _ => div()
                    .size(px(10.))
                    .rounded_full()
                    .flex_none()
                    .bg(palette.strong),
            };
            let knot_open = self.context.as_ref().is_some_and(|open| {
                open.stands_on(depot, &flow.name, &flow.flow_type, &stage.stage)
            });
            let (knot_depot, knot_flow, knot_type, knot_stage) = (
                depot.to_string(),
                flow.name.clone(),
                flow.flow_type.clone(),
                stage.stage.clone(),
            );
            let knot_selector = format!("flow knot {} {}", flow.name, stage.stage);
            strip = strip.child(
                div()
                    .id(SharedString::from(knot_selector.clone()))
                    .debug_selector({
                        let knot_selector = knot_selector.clone();
                        move || knot_selector.clone()
                    })
                    .flex()
                    .flex_row()
                    .flex_none()
                    .items_center()
                    .gap_1p5()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.open_context(
                            knot_depot.clone(),
                            knot_flow.clone(),
                            knot_type.clone(),
                            knot_stage.clone(),
                            cx,
                        )
                    }))
                    .child(knot)
                    .child(
                        div()
                            .text_size(rems(TYPE_META))
                            .text_color(match on || knot_open {
                                true => palette.text,
                                false => palette.faint,
                            })
                            .when(on || knot_open, |el| el.font_weight(FontWeight::MEDIUM))
                            .child(label),
                    ),
            );
        }
        let marks = flow_strip_overflows(pane_width, rem, &labels);
        div()
            .flex()
            .flex_row()
            .items_center()
            .border_b_1()
            .border_color(palette.line)
            .child(strip)
            .when(marks, |el| {
                el.child(
                    div()
                        .debug_selector({
                            let flow = flow.name.clone();
                            move || format!("flow strip more {flow}")
                        })
                        .flex_none()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_1p5()
                        .pl_2()
                        .pr_4()
                        .child(div().w(px(20.)).h(px(2.)).flex_none().bg(palette.strong))
                        .child(
                            div()
                                .text_size(rems(TYPE_META))
                                .text_color(palette.faint)
                                .child("›"),
                        ),
                )
            })
    }

    /// Whether the flow's served landings stand under the stage. They
    /// all entered merged, so they stand under the stage of that name
    /// where the flow has one, else under its first terminal stage.
    fn flow_has_landings_at(flow: &Flow, stage: &str) -> bool {
        let under = flow
            .stages
            .iter()
            .find(|candidate| candidate.stage == "merged")
            .or_else(|| flow.stages.iter().find(|candidate| candidate.terminal))
            .map(|candidate| candidate.stage.as_str());
        under == Some(stage) && !flow.landings.is_empty()
    }

    /// One stage's group in the flow's list: its head with its count
    /// and its group words, then one row per bead — the backlog's with
    /// where each stands next, the landed end's with what landed under
    /// it. Nothing where the stage stands empty: the empty fold names
    /// those. Stages never collapse.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn flow_group(
        &self,
        model: &Model,
        flow: &Flow,
        stage: &FlowStage,
        now: &chrono::DateTime<chrono::Utc>,
        mono: Option<SharedString>,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let has_landings = Self::flow_has_landings_at(flow, &stage.stage);
        if stage.beads.is_empty() && !has_landings {
            return div().into_any_element();
        }
        let words = match stage.stage.as_str() {
            "backlog" => format!("{} · {}", stage.beads.len(), stage_group_words(stage)),
            _ if stage.terminal => {
                let mut parts = Vec::new();
                if !stage.beads.is_empty() {
                    parts.push(bead_words(stage.beads.len()));
                }
                parts.push("the end of the flow".to_string());
                parts.join(" · ")
            }
            _ => format!("{} · {}", stage.beads.len(), stage_group_words(stage)),
        };
        let (stage_budget, words_budget) = flow_group_budgets(
            pane_width,
            rem,
            stage.stage.chars().count(),
            words.chars().count(),
        );
        let stage_name = shorten(&stage.stage, stage_budget);
        let group_words = shorten(&words, words_budget);
        let mark: AnyElement = match stage.stage.as_str() {
            "backlog" => div()
                .size(px(10.))
                .rounded_full()
                .flex_none()
                .border_1()
                .border_color(palette.faint)
                .into_any_element(),
            _ if stage.terminal => {
                outcome_glyph(model::Outcome::Approved, ROW_MARK, palette.ok).into_any_element()
            }
            _ => lamp_dot(
                stage_state(stage),
                format!("flow lamp {} {}", flow.name, stage.stage),
                palette,
            )
            .map(|dot| dot.into_any_element())
            .unwrap_or_else(|| div().size(px(8.)).flex_none().into_any_element()),
        };
        let mut group = div().flex().flex_col().child(
            div()
                .debug_selector({
                    let (flow, stage) = (flow.name.clone(), stage.stage.clone());
                    move || format!("flow group {flow} {stage}")
                })
                .px_4()
                .py_2()
                .bg(palette.side)
                .border_b_1()
                .border_color(palette.line)
                .flex()
                .flex_row()
                .items_center()
                .gap_2p5()
                .child(mark)
                .child(
                    div()
                        .text_size(rems(TYPE_SECTION))
                        .font_weight(FontWeight::SEMIBOLD)
                        .min_w_0()
                        .truncate()
                        .child(stage_name),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child(group_words),
                ),
        );
        for bead in &stage.beads {
            group = group.child(self.flow_bead_row(
                model,
                flow,
                stage,
                bead,
                now,
                mono.clone(),
                pane_width,
                rem,
                palette,
                cx,
            ));
        }
        if has_landings {
            for landing in &flow.landings {
                group = group.child(self.flow_landing_row(
                    landing,
                    now,
                    mono.clone(),
                    pane_width,
                    rem,
                    palette,
                    cx,
                ));
            }
        }
        group.into_any_element()
    }

    /// One bead's row in its stage's group: its mark, its short id, its
    /// title, and at the right who works it with how long it has stood
    /// — the backlog's with the stage past it instead. A click opens
    /// the bead whole.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn flow_bead_row(
        &self,
        model: &Model,
        flow: &Flow,
        stage: &FlowStage,
        bead: &FlowBead,
        now: &chrono::DateTime<chrono::Utc>,
        mono: Option<SharedString>,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let sel = Selected::Bead(bead.bead.clone());
        let name = bead.bead.clone();
        let raw_meta = match stage.stage.as_str() {
            "backlog" => next_stage(flow)
                .map(|next| format!("→ {next}"))
                .unwrap_or_default(),
            _ => {
                let mut parts = Vec::new();
                if !bead.worker.is_empty() {
                    parts.push(model::worker_session(&bead.worker).to_string());
                }
                if !bead.entered_stage_at.is_empty() {
                    let span = model::waits(&bead.entered_stage_at, now);
                    if !span.is_empty() {
                        parts.push(span);
                    }
                }
                if waits_for_seat(bead, stage) {
                    parts.push("waits for a seat".to_string());
                }
                if parts.is_empty() && !bead.remark.is_empty() {
                    parts.push(bead.remark.clone());
                }
                parts.join(" · ")
            }
        };
        let (title_budget, meta_budget) = flow_row_budgets(
            pane_width,
            rem,
            bead.title.chars().count(),
            raw_meta.chars().count(),
        );
        let title = shorten(&bead.title, title_budget);
        let meta = shorten(&raw_meta, meta_budget);
        let mark: AnyElement = match stage.stage.as_str() {
            "backlog" => div()
                .size(px(10.))
                .rounded_full()
                .flex_none()
                .border_1()
                .border_color(palette.faint)
                .into_any_element(),
            _ if bead.working => lamp_dot("moving", format!("flow mark {name}"), palette)
                .map(|dot| dot.into_any_element())
                .unwrap_or_else(|| div().size(px(8.)).flex_none().into_any_element()),
            _ if chip_you(bead, model.me(), &model.on_you_beads()) => {
                lamp_dot("turn", format!("flow mark {name}"), palette)
                    .map(|dot| dot.into_any_element())
                    .unwrap_or_else(|| div().size(px(8.)).flex_none().into_any_element())
            }
            _ => div().size(px(8.)).flex_none().into_any_element(),
        };
        let mut id = grown(FLOW_ID)
            .flex_none()
            .truncate()
            .text_size(rems(TYPE_CODE))
            .text_color(palette.faint)
            .child(short_bead(&bead.bead));
        if let Some(mono) = mono {
            id = id.font_family(mono);
        }
        div()
            .id(SharedString::from(format!("flow row {name}")))
            .debug_selector({
                let name = name.clone();
                move || format!("flow row {name}")
            })
            .h(px(40.))
            .px_4()
            .border_b_1()
            .border_color(palette.line)
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.select(sel.clone(), window, cx)))
            .child(mark)
            .child(id)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(rems(TYPE_BODY))
                    .child(title),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .child(meta),
            )
    }

    /// One bead that landed under the flow's landed end: its mark, its
    /// short id, its title, and how long ago it landed. A click opens
    /// the bead whole, as a card's click does; the Bead call serves
    /// closed beads.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn flow_landing_row(
        &self,
        landing: &FlowLanding,
        now: &chrono::DateTime<chrono::Utc>,
        mono: Option<SharedString>,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let name = landing.bead.clone();
        let sel = Selected::Bead(landing.bead.clone());
        let raw_meta = landed_age(&model::waits(&landing.landed_at, now));
        let (title_budget, meta_budget) = flow_row_budgets(
            pane_width,
            rem,
            landing.title.chars().count(),
            raw_meta.chars().count(),
        );
        let title = shorten(&landing.title, title_budget);
        let meta = shorten(&raw_meta, meta_budget);
        let mut id = grown(FLOW_ID)
            .flex_none()
            .truncate()
            .text_size(rems(TYPE_CODE))
            .text_color(palette.faint)
            .child(short_bead(&landing.bead));
        if let Some(mono) = mono {
            id = id.font_family(mono);
        }
        div()
            .id(SharedString::from(format!("flow landed {name}")))
            .debug_selector({
                let name = name.clone();
                move || format!("flow landed {name}")
            })
            .h(px(40.))
            .px_4()
            .border_b_1()
            .border_color(palette.line)
            .flex()
            .flex_row()
            .items_center()
            .gap_3()
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.select(sel.clone(), window, cx)))
            .child(outcome_glyph(
                model::Outcome::Approved,
                ROW_MARK,
                palette.ok,
            ))
            .child(id)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(rems(TYPE_BODY))
                    .child(title),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .child(meta),
            )
    }

    /// The flow's empty stages folded to one row: their count with
    /// their names, unfolding to a head each. Nothing where every
    /// stage holds a bead or a landing.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn flow_empty_fold(
        &self,
        model: &Model,
        depot: &str,
        flow: &Flow,
        pane_width: f32,
        rem: f32,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let empty: Vec<&FlowStage> = flow
            .stages
            .iter()
            .filter(|stage| {
                stage.beads.is_empty() && !Self::flow_has_landings_at(flow, &stage.stage)
            })
            .collect();
        if empty.is_empty() {
            return div().into_any_element();
        }
        let open = model.empty_stages_expanded(depot, &flow.name, &flow.flow_type);
        let (depot_name, flow_name, flow_type) =
            (depot.to_string(), flow.name.clone(), flow.flow_type.clone());
        let count = match empty.len() {
            1 => "1 empty stage".to_string(),
            empty => format!("{empty} empty stages"),
        };
        let names = empty
            .iter()
            .map(|stage| stage.stage.as_str())
            .collect::<Vec<_>>()
            .join(" · ");
        let (count_budget, names_budget) = flow_empty_budgets(
            pane_width,
            rem,
            count.chars().count(),
            names.chars().count(),
        );
        let count = shorten(&count, count_budget);
        let names = shorten(&names, names_budget);
        let mut fold = div().flex().flex_col().child(
            div()
                .id(SharedString::from(format!(
                    "flow empty {depot} {}",
                    flow.name
                )))
                .debug_selector({
                    let (depot, flow) = (depot.to_string(), flow.name.clone());
                    move || format!("flow empty {depot} {flow}")
                })
                .w_full()
                .px_4()
                .py_2()
                .bg(palette.side)
                .border_b_1()
                .border_color(palette.line)
                .flex()
                .flex_row()
                .items_center()
                .gap_2p5()
                .cursor_pointer()
                .hover(|style| style.bg(palette.hover))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_empty_stages(&depot_name, &flow_name, &flow_type, cx)
                }))
                .child(fold_chevron(open, palette.faint))
                .child(
                    div()
                        .text_size(rems(TYPE_SECTION))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(count),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child(names),
                ),
        );
        if open {
            for stage in empty {
                let stage_name = shorten(
                    &stage.stage,
                    flow_empty_stage_budget(pane_width, rem, stage.stage.chars().count()),
                );
                fold = fold.child(
                    div()
                        .debug_selector({
                            let (flow, stage) = (flow.name.clone(), stage.stage.clone());
                            move || format!("flow empty stage {flow} {stage}")
                        })
                        .px_4()
                        .py_2()
                        .border_b_1()
                        .border_color(palette.line)
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap_2p5()
                        .child(div().size(px(10.)).flex_none())
                        .child(
                            div()
                                .text_size(rems(TYPE_SECTION))
                                .font_weight(FontWeight::SEMIBOLD)
                                .min_w_0()
                                .truncate()
                                .child(stage_name),
                        )
                        .child(
                            div()
                                .text_size(rems(TYPE_META))
                                .text_color(palette.faint)
                                .child("empty"),
                        ),
                );
            }
        }
        fold.into_any_element()
    }

    /// One flow's section: the flow name with its type where typed and
    /// its source, then its stages in travel order as columns — the
    /// panel's track straightened — scrolling sideways past the pane's
    /// edge. Stages never collapse.
    pub(crate) fn flow_section(
        &self,
        flow: &Flow,
        now: &chrono::DateTime<chrono::Utc>,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut columns = div()
            .id(SharedString::from(format!("columns {}", flow.name)))
            .overflow_x_scroll()
            .flex()
            .flex_row();
        // The columns take horizontal moves alone and leave the
        // vertical ones to the list, for the same reason as the list.
        columns.style().restrict_scroll_to_axis = Some(true);
        for stage in &flow.stages {
            columns = columns.child(match stage.stage.as_str() {
                "backlog" => self
                    .backlog_column(flow, stage, palette, cx)
                    .into_any_element(),
                _ => self
                    .stage_column(flow, stage, now, mono.clone(), palette, cx)
                    .into_any_element(),
            });
        }
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .debug_selector({
                                let flow = flow.name.clone();
                                move || format!("flow {flow}")
                            })
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(flow.name.clone()),
                    )
                    .child(div().text_xs().text_color(palette.faint).child(
                        match flow.flow_type.is_empty() {
                            true => flow.source.clone(),
                            false => format!("{} · {}", flow.flow_type, flow.source),
                        },
                    )),
            )
            .child(columns)
    }

    /// One stage's column: its name with the group words under it and
    /// the track as the head's rule, then one card per bead. The
    /// terminal stage reads terminal with the buffer stop, and lists
    /// under it what landed there in the trailing 24 hours; an empty
    /// stage says nothing stands here.
    pub(crate) fn stage_column(
        &self,
        flow: &Flow,
        stage: &FlowStage,
        now: &chrono::DateTime<chrono::Utc>,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut column = grown(LINE_COLUMN)
            .debug_selector({
                let (flow, stage) = (flow.name.clone(), stage.stage.clone());
                move || format!("column {flow} {stage}")
            })
            .flex_none()
            .border_l_1()
            .border_color(palette.line)
            .px_2()
            .flex()
            .flex_col()
            .child(
                div().flex().flex_col().pb_1().child(
                    // The name over two group lines, bottom-aligned:
                    // the rules stand level across columns.
                    div()
                        .min_h(rems(LINE_HEAD))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .children(lamp_dot(
                                    stage_lamp(stage),
                                    format!("column {} {} lamp", flow.name, stage.stage),
                                    palette,
                                ))
                                .child(
                                    div()
                                        .debug_selector({
                                            let (flow, stage) =
                                                (flow.name.clone(), stage.stage.clone());
                                            move || format!("column {flow} {stage} head")
                                        })
                                        .truncate()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(stage.stage.clone()),
                                ),
                        )
                        .child(match stage.terminal {
                            true => div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .text_xs()
                                .text_color(palette.faint)
                                .child("terminal")
                                .child(buffer_stop(
                                    format!("column {} {} stop", flow.name, stage.stage),
                                    palette,
                                ))
                                .into_any_element(),
                            false => div()
                                .debug_selector({
                                    let (flow, stage) = (flow.name.clone(), stage.stage.clone());
                                    move || format!("column {flow} {stage} group")
                                })
                                .text_xs()
                                .text_color(palette.faint)
                                .child(stage_group_words(stage))
                                .into_any_element(),
                        }),
                ),
            )
            .child(
                column_rule(stage_lamp(stage), stage.human, palette).debug_selector({
                    let (flow, stage) = (flow.name.clone(), stage.stage.clone());
                    move || format!("column {flow} {stage} rule")
                }),
            );
        // What landed stands under the landed station alone — the
        // stage the list stands it under — and any other column
        // draws the stages' beads and nothing else.
        let landings = match Self::flow_has_landings_at(flow, &stage.stage) {
            true => flow.landings.as_slice(),
            false => &[],
        };
        if stage.beads.is_empty() && landings.is_empty() {
            column = column.child(
                div()
                    .debug_selector({
                        let (flow, stage) = (flow.name.clone(), stage.stage.clone());
                        move || format!("column {flow} {stage} empty")
                    })
                    .pt_2()
                    .text_xs()
                    .text_color(palette.faint)
                    .child("nothing here"),
            );
        }
        for bead in &stage.beads {
            column =
                column.child(self.bead_card(bead, Some(stage), now, mono.clone(), palette, cx));
        }
        for landing in landings {
            column = column.child(self.landing_row(flow, stage, landing, now, palette, cx));
        }
        column
    }

    /// One bead that landed under the flow's landed station: its title
    /// with its headcode and how long ago it landed faint under it, in
    /// the served order, newest first. A tap opens the bead whole, as a
    /// card's tap does; the Bead call serves closed beads. Scoped to
    /// its column, as the column's own selectors are.
    pub(crate) fn landing_row(
        &self,
        flow: &Flow,
        stage: &FlowStage,
        landing: &FlowLanding,
        now: &chrono::DateTime<chrono::Utc>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let bead = landing.bead.clone();
        let sel = Selected::Bead(landing.bead.clone());
        let scope = format!("{} {} {bead}", flow.name, stage.stage);
        let meta = [
            short_bead(&landing.bead),
            landed_age(&model::waits(&landing.landed_at, now)),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        div()
            .id(SharedString::from(format!("landing {scope}")))
            .debug_selector({
                let scope = scope.clone();
                move || format!("landing {scope}")
            })
            .pt_2()
            .flex()
            .flex_col()
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.select(sel.clone(), window, cx)))
            .child(
                div()
                    .debug_selector({
                        let scope = scope.clone();
                        move || format!("landing title {scope}")
                    })
                    .truncate()
                    .text_sm()
                    .child(landing.title.clone()),
            )
            .child(
                div()
                    .debug_selector({
                        let scope = scope.clone();
                        move || format!("landing meta {scope}")
                    })
                    .text_xs()
                    .text_color(palette.faint)
                    .child(meta),
            )
    }

    /// One flow's backlog column: its name with the group words under
    /// it and the track as the head's rule, then its count with the
    /// first titles as plain rows and "+N more" past them. A title's
    /// click opens its bead.
    pub(crate) fn backlog_column(
        &self,
        flow: &Flow,
        stage: &FlowStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut column = grown(LINE_COLUMN)
            .debug_selector({
                let flow = flow.name.clone();
                move || format!("column {flow} backlog")
            })
            .flex_none()
            .border_l_1()
            .border_color(palette.line)
            .px_2()
            .flex()
            .flex_col()
            .child(
                div().flex().flex_col().pb_1().child(
                    // The name over two group lines, bottom-aligned:
                    // the rules stand level across columns.
                    div()
                        .min_h(rems(LINE_HEAD))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .children(lamp_dot(
                                    stage_lamp(stage),
                                    format!("column {} backlog lamp", flow.name),
                                    palette,
                                ))
                                .child(
                                    div()
                                        .debug_selector({
                                            let flow = flow.name.clone();
                                            move || format!("column {flow} backlog head")
                                        })
                                        .truncate()
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(stage.stage.clone()),
                                ),
                        )
                        .child(
                            div()
                                .debug_selector({
                                    let flow = flow.name.clone();
                                    move || format!("column {flow} backlog group")
                                })
                                .text_xs()
                                .text_color(palette.faint)
                                .child(stage_group_words(stage)),
                        ),
                ),
            )
            .child(
                column_rule(stage_lamp(stage), stage.human, palette).debug_selector({
                    let flow = flow.name.clone();
                    move || format!("column {flow} backlog rule")
                }),
            )
            .child(
                div()
                    .debug_selector({
                        let flow = flow.name.clone();
                        move || format!("column {flow} backlog count")
                    })
                    .pt_2()
                    .text_xs()
                    .text_color(palette.faint)
                    .child(waiting_words(stage.beads.len())),
            );
        for bead in stage.beads.iter().take(BACKLOG_TITLES) {
            let sel = Selected::Bead(bead.bead.clone());
            let title = bead.bead.clone();
            column =
                column.child(
                    div()
                        .id(SharedString::from(format!("backlog title {title}")))
                        .debug_selector({
                            let title = title.clone();
                            move || format!("backlog title {title}")
                        })
                        .truncate()
                        .text_xs()
                        .py_0p5()
                        .cursor_pointer()
                        .hover(|style| style.bg(palette.hover))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select(sel.clone(), window, cx)
                        }))
                        .child(bead.title.clone()),
                );
        }
        let rest = stage.beads.len().saturating_sub(BACKLOG_TITLES);
        if rest > 0 {
            column = column.child(
                div()
                    .debug_selector({
                        let flow = flow.name.clone();
                        move || format!("backlog more {flow}")
                    })
                    .pt_1()
                    .text_xs()
                    .text_color(palette.faint)
                    .child(format!("+{rest} more")),
            );
        }
        column
    }

    /// One bead's card in a stage column or the Elsewhere column: the
    /// headcode and the title, then who works it, how long it has stood
    /// at its stage and that it waits for a seat, each where it holds,
    /// the expected arrival in mono, and the remark in its tone.
    /// `stage` names where it stands; Elsewhere's cards stand in none,
    /// and read `elsewhere card` for the tests where a stage's reads
    /// `card`. A click opens the bead's details, and hovering a worked
    /// card says which session works it, with its group, kind and model.
    pub(crate) fn bead_card(
        &self,
        row: &FlowBead,
        stage: Option<&FlowStage>,
        now: &chrono::DateTime<chrono::Utc>,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let which = match stage {
            Some(_) => "card",
            None => "elsewhere card",
        };
        let bead = row.bead.clone();
        let sel = Selected::Bead(row.bead.clone());
        let tip = worker_words(row).map(|(tip, _)| tip);
        let me = self.model.as_ref().map(Model::me).unwrap_or_default();
        let color = remark_color(&row.remark, &row.tone, &row.held_by, me, palette);
        let mut meta = Vec::new();
        if !row.worker.is_empty() {
            meta.push(format!("worked by {}", row.worker));
        }
        if let Some(stage) = stage
            && !row.entered_stage_at.is_empty()
            && let Some(age) = stage_age(&row.entered_stage_at, &stage.stage, now)
        {
            meta.push(age);
        }
        if stage.is_some_and(|stage| waits_for_seat(row, stage)) {
            meta.push("waits for a seat".to_string());
        }
        let mut card = div()
            .id(SharedString::from(format!("{which} {bead}")))
            .debug_selector({
                let bead = bead.clone();
                move || format!("{which} {bead}")
            })
            .mt_2()
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(palette.line)
            .bg(palette.side)
            .flex()
            .flex_col()
            .gap_1()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| this.select(sel.clone(), window, cx)))
            .when_some(tip, |el, tip| {
                el.tooltip(move |_, cx| cx.new(|_| RailTip(tip.clone())).into())
            });
        let mut code = div()
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(palette.text)
            .child(short_bead(&row.bead));
        if let Some(mono) = mono.clone() {
            code = code.font_family(mono);
        }
        card = card.child(code).child(
            div()
                .font_weight(FontWeight::MEDIUM)
                .debug_selector({
                    let bead = bead.clone();
                    move || format!("{which} title {bead}")
                })
                .child(row.title.clone()),
        );
        if !meta.is_empty() {
            card = card.child(
                div()
                    .text_xs()
                    .text_color(palette.faint)
                    .debug_selector({
                        let bead = bead.clone();
                        move || format!("{which} meta {bead}")
                    })
                    .child(meta.join(" · ")),
            );
        }
        let mut expected = div()
            .text_xs()
            .text_color(palette.faint)
            .debug_selector({
                let bead = bead.clone();
                move || format!("{which} expected {bead}")
            })
            .child(row.expected.clone());
        if let Some(mono) = mono {
            expected = expected.font_family(mono);
        }
        card = card.child(expected);
        if !row.remark.is_empty() {
            card = card.child(
                div()
                    .text_xs()
                    .text_color(color)
                    .debug_selector({
                        let bead = bead.clone();
                        move || format!("{which} remark {bead}")
                    })
                    .child(you_first(&row.remark, me)),
            );
        }
        card
    }

    /// The line's Elsewhere column: its head with its count, then its
    /// beads as cards, under the flows.
    pub(crate) fn elsewhere_column(
        &self,
        depot: &FlowDepot,
        now: &chrono::DateTime<chrono::Utc>,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let mut column = grown(LINE_COLUMN).flex_none().flex().flex_col();
        for bead in &depot.elsewhere {
            column = column.child(self.bead_card(bead, None, now, mono.clone(), palette, cx));
        }
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .debug_selector(|| "elsewhere".to_string())
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Elsewhere"),
                    )
                    .child(
                        div()
                            .debug_selector(|| "elsewhere count".to_string())
                            .text_sm()
                            .text_color(palette.faint)
                            .flex_none()
                            .child(bead_words(depot.elsewhere.len())),
                    ),
            )
            .child(column)
    }

    /// One bead travelling with no line: the title, the expected
    /// arrival in mono at the row's end, and faint the id and remark on
    /// the second line, in the plain list newest change first. A click
    /// opens the details.
    pub(crate) fn no_line_bead_row(
        &self,
        row: &FlowBead,
        mono: Option<SharedString>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let bead = row.bead.clone();
        let sel = Selected::Bead(row.bead.clone());
        let el = div()
            .id(SharedString::from(format!("no line bead {bead}")))
            .debug_selector({
                let bead = bead.clone();
                move || format!("no line bead {bead}")
            })
            .mx_1()
            .px_2()
            .py_1p5()
            .rounded_md()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .cursor_pointer()
            .border_1()
            .border_color(rgba(0x00000000))
            .hover(|style| style.border_color(palette.line))
            .on_click(cx.listener(move |this, _, window, cx| this.select(sel.clone(), window, cx)));
        let me = self.model.as_ref().map(Model::me).unwrap_or_default();
        let remark = remark_color(&row.remark, &row.tone, &row.held_by, me, palette);
        let mut time = div()
            .flex_none()
            .debug_selector({
                let bead = bead.clone();
                move || format!("no line bead time {bead}")
            })
            .text_color(match row.expected.as_str() {
                "—" => palette.faint,
                _ => palette.text,
            })
            .child(row.expected.clone());
        if let Some(mono) = mono {
            time = time.font_family(mono);
        }
        el.child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .debug_selector({
                            let bead = bead.clone();
                            move || format!("no line bead title {bead}")
                        })
                        .child(row.title.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_xs()
                                .text_color(palette.faint)
                                .debug_selector({
                                    let bead = bead.clone();
                                    move || format!("no line bead sub {bead}")
                                })
                                .child(bead.clone()),
                        )
                        .children((!row.remark.is_empty()).then(|| {
                            div()
                                .flex_none()
                                .text_xs()
                                .text_color(remark)
                                .debug_selector({
                                    let bead = bead.clone();
                                    move || format!("no line bead remark {bead}")
                                })
                                .child(format!("· {}", row.remark))
                        })),
                ),
        )
        .child(time)
    }

    /// The beads travelling with no line: a plain list, newest change
    /// first, each row the title with the expected arrival and remark.
    /// No flows, no stages, no lamps, because a bead with no depot
    /// travels no flow.
    pub(crate) fn talk_no_line(
        &self,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let pane = div().flex_1().min_w_0().h_full().flex().flex_col();
        let head = div()
            .p_3()
            .border_b_1()
            .border_color(palette.line)
            .flex()
            .flex_col()
            .child(
                div()
                    .debug_selector(|| "no line head".to_string())
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(model::NO_FLOW),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .child(model.no_line_row().map(|row| row.line).unwrap_or_default()),
            );
        let mono = select::mono(cx);
        let mut list = div()
            .id("no line")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.decision)
            .p_4()
            .flex()
            .flex_col()
            .gap_3();
        for row in model.unassigned() {
            list = list.child(self.no_line_bead_row(row, mono.clone(), palette, cx));
        }
        if model.unassigned().is_empty() {
            list = list.child(div().text_color(palette.faint).child("None in transit."));
        }
        pane.children(self.reconnect_strip(palette))
            .child(head)
            .child(list)
            .when(!model.notice.is_empty(), |el| {
                el.child(
                    div()
                        .px_4()
                        .py_1()
                        .text_sm()
                        .text_color(palette.warn)
                        .child(format!("! {}", model.notice)),
                )
            })
    }
}
