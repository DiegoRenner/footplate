//! The Line window: one depot's line read whole through `Line` —
//! its flows named by their source, the stage track in the flow
//! map's words, the stage detail with its Rule line, its chain, its
//! Other rules and its next stages, its stage file and prompts each
//! behind Read, and the crew prompts below. It writes nothing: Edit
//! stands faint until slice B on a prompt a card can carry, and a
//! stage file has none.

use gpui::{Context, Div, FontWeight, SharedString, div, prelude::*, px};

use super::Footplate;
use super::panel::{buffer_stop, panel_track_line};
use super::ui::small_button;
use crate::api::{LineFlow, LinePrompt, LineStage};
use crate::model::{self, Model, Selected};
use crate::theme::Palette;

impl Footplate {
    /// The Line window for `depot`: the pickers over the depots
    /// already listed and the depot's flows, the stage track, the
    /// open stage's detail and the crew below. A yard from before
    /// the call says so in the pane, in no red, and the window is
    /// offered no more; a read that failed says so there too, and
    /// opening the window again tries again.
    pub(crate) fn talk_line_window(
        &self,
        model: &Model,
        depot: &str,
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
                    .font_weight(FontWeight::SEMIBOLD)
                    .debug_selector(|| "line head".to_string())
                    .child("Line"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .debug_selector(|| "line reopen".to_string())
                    .child("Read when the window opens; reopen it after an edit lands."),
            );
        if model.line_unserved() {
            return pane
                .children(self.reconnect_strip(palette))
                .child(head)
                .child(
                    div().flex_1().min_h_0().p_4().child(
                        div()
                            .debug_selector(|| "line too old".to_string())
                            .text_color(palette.faint)
                            .child(model::LINE_TOO_OLD),
                    ),
                )
                .into_any_element();
        }
        if !model.is_operator() {
            return pane
                .children(self.reconnect_strip(palette))
                .child(head)
                .child(
                    div().flex_1().min_h_0().p_4().child(
                        div()
                            .debug_selector(|| "line operator".to_string())
                            .text_color(palette.faint)
                            .child("The Line window is the operator's."),
                    ),
                )
                .into_any_element();
        }
        let mut body = div()
            .id("line window")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.decision)
            .p_4()
            .flex()
            .flex_col()
            .gap_4();
        // The depot picker: the depots already listed, each opening
        // the window on it, which reads its line.
        let depots = div().flex().flex_col().gap_1().child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.faint)
                .child("Depot"),
        );
        let mut depot_row = div().flex().flex_row().flex_wrap().gap_2();
        for listed in model.depots() {
            let name = listed.name.clone();
            let sel = Selected::LineWindow(name.clone());
            depot_row = depot_row.child(
                small_button(name.clone(), palette)
                    .id(SharedString::from(format!("line depot {name}")))
                    .debug_selector({
                        let name = name.clone();
                        move || format!("line depot {name}")
                    })
                    .when(name == depot, |el| el.bg(palette.open))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.line_flow.clear();
                        this.line_stage.clear();
                        this.select(sel.clone(), window, cx);
                    })),
            );
        }
        body = body.child(depots.child(depot_row));
        match model.line_read_for(depot) {
            None => {
                let words = match model.line_notice() {
                    Some(notice) => notice.to_string(),
                    None => "Reading the line…".to_string(),
                };
                body = body.child(
                    div()
                        .debug_selector(|| "line reading".to_string())
                        .text_color(palette.faint)
                        .child(words),
                );
            }
            Some(line) => {
                let flow = line
                    .flows
                    .iter()
                    .find(|flow| model::line_flow_key(flow) == self.line_flow)
                    .or_else(|| line.flows.first());
                // The flow picker: each flow named by its source.
                let flows = div().flex().flex_col().gap_1().child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(palette.faint)
                        .child("Flow"),
                );
                let mut flow_row = div().flex().flex_row().flex_wrap().gap_2();
                for named in &line.flows {
                    let key = model::line_flow_key(named);
                    let picked = flow.is_some_and(|flow| model::line_flow_key(flow) == key);
                    let name = named.name.clone();
                    let source = named.source.clone();
                    let selector = match named.flow_type.is_empty() {
                        true => format!("{name} {source}"),
                        false => format!("{} {name} {source}", named.flow_type),
                    };
                    let selector = format!("line flow {selector}");
                    flow_row = flow_row.child(
                        small_button(model::line_flow_words(named), palette)
                            .id(SharedString::from(selector.clone()))
                            .debug_selector({
                                let selector = selector.clone();
                                move || selector.clone()
                            })
                            .when(picked, |el| el.bg(palette.open))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.line_flow = key.clone();
                                this.line_stage.clear();
                                cx.notify();
                            })),
                    );
                }
                body = body.child(flows.child(flow_row));
                if let Some(flow) = flow {
                    body = body.child(self.line_track(flow, palette, cx));
                    for (index, warning) in flow.warnings.iter().enumerate() {
                        body = body.child(
                            div()
                                .text_sm()
                                .text_color(palette.faint)
                                .debug_selector({
                                    let flow = flow.name.clone();
                                    move || format!("line flow warning {flow} {index}")
                                })
                                .child(warning.clone()),
                        );
                    }
                    let stage = flow
                        .stages
                        .iter()
                        .find(|stage| stage.stage == self.line_stage)
                        .or_else(|| flow.stages.first());
                    if let Some(stage) = stage {
                        body = body.child(self.line_detail(flow, stage, palette, cx));
                    }
                }
                body = body.child(self.line_crew(line, palette, cx));
            }
        }
        pane.children(self.reconnect_strip(palette))
            .child(head)
            .child(body)
            .into_any_element()
    }

    /// One flow's stage track: each stage a cell with its name, who
    /// serves it in the flow map's words and its open count, over
    /// the panel's track line, with the buffer stop where entering
    /// closes the bead. A tap opens the stage's detail below.
    fn line_track(
        &self,
        flow: &LineFlow,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut track = div()
            .id(SharedString::from(format!(
                "line track {}",
                model::line_flow_heading(flow)
            )))
            .overflow_x_scroll()
            .flex()
            .flex_row();
        track.style().restrict_scroll_to_axis = Some(true);
        let flow_heading = model::line_flow_heading(flow);
        let selected = flow
            .stages
            .iter()
            .find(|stage| stage.stage == self.line_stage)
            .or_else(|| flow.stages.first())
            .map(|stage| stage.stage.as_str());
        for stage in &flow.stages {
            let who = model::line_who(stage);
            let picked = selected == Some(stage.stage.as_str());
            let name = stage.stage.clone();
            let flow_name = flow_heading.clone();
            let mut cell = div()
                .id(SharedString::from(format!(
                    "line stage {} {}",
                    flow_heading, stage.stage
                )))
                .debug_selector({
                    let (flow, stage) = (flow_heading.clone(), stage.stage.clone());
                    move || format!("line stage {flow} {stage}")
                })
                .flex_none()
                .w(px(132.))
                .px_2()
                .py_1()
                .rounded_md()
                .flex()
                .flex_col()
                .gap_1()
                .cursor_pointer()
                .when(picked, |el| el.bg(palette.open))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.line_stage = name.clone();
                    cx.notify();
                }))
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(stage.stage.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(palette.faint)
                        .debug_selector({
                            let (flow, stage) = (flow_name.clone(), stage.stage.clone());
                            move || format!("line who {flow} {stage}")
                        })
                        .child(who),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(palette.faint)
                        .child(match stage.terminal {
                            true => "—".to_string(),
                            false => format!("{} open", stage.open),
                        }),
                )
                .child(panel_track_line(who == "people", stage.open > 0, palette));
            if stage.terminal {
                cell = cell.child(buffer_stop(
                    format!("line stop {} {}", flow_heading, stage.stage),
                    palette,
                ));
            }
            track = track.child(cell);
        }
        track
    }

    /// One stage's detail under the track: its headline, its Rule
    /// line, its chain, its Other rules, its next stages, its stage
    /// file and its groups' prompts, each behind Read.
    fn line_detail(
        &self,
        flow: &LineFlow,
        stage: &LineStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let who = model::line_who(stage);
        let served = match stage.route.as_ref().and_then(|route| route.chain.first()) {
            Some(group) => format!("served by {} ({who})", group.name),
            None if stage.terminal => "entering it closes the bead".to_string(),
            None => format!("served by nobody ({who})"),
        };
        let mut detail = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .debug_selector({
                        let stage = stage.stage.clone();
                        move || format!("line detail {stage}")
                    })
                    .child(format!("Stage {} · {served}", stage.stage)),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .debug_selector({
                        let stage = stage.stage.clone();
                        move || format!("line open {stage}")
                    })
                    .child(match stage.open {
                        1 => "Open · 1 bead".to_string(),
                        open => format!("Open · {open} beads"),
                    }),
            );
        if let Some(route) = &stage.route {
            detail = detail.child(
                div()
                    .debug_selector({
                        let stage = stage.stage.clone();
                        move || format!("line rule {stage}")
                    })
                    .child(format!(
                        "Rule {}",
                        model::line_rule_words(&route.id, &route.scope, &route.filters)
                    )),
            );
            for (at, group) in route.chain.iter().enumerate() {
                let words = match at {
                    0 => format!("{} · {}", group.name, model::line_group_words(group)),
                    _ => format!("then {} · {}", group.name, model::line_group_words(group)),
                };
                let words = match group.when.is_empty() {
                    true => words,
                    false => format!("{words} where {}", group.when),
                };
                detail = detail.child(
                    div()
                        .text_sm()
                        .debug_selector({
                            let (stage, group) = (stage.stage.clone(), group.name.clone());
                            move || format!("line chain {stage} {group}")
                        })
                        .child(words),
                );
            }
            if let Some(limit) = route.limit {
                detail = detail.child(
                    div()
                        .text_sm()
                        .text_color(palette.faint)
                        .debug_selector({
                            let stage = stage.stage.clone();
                            move || format!("line limit {stage}")
                        })
                        .child(format!("limit {limit}")),
                );
            }
        }
        if !stage.other_routes.is_empty() {
            detail = detail.child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(palette.faint)
                    .child("Other rules"),
            );
            for other in &stage.other_routes {
                detail = detail.child(
                    div()
                        .text_sm()
                        .debug_selector({
                            let (stage, id) = (stage.stage.clone(), other.id.clone());
                            move || format!("line other {stage} {id}")
                        })
                        .child(format!(
                            "{} → {}",
                            model::line_rule_words(&other.id, &other.scope, &other.filters),
                            other.group
                        )),
                );
            }
        }
        let next = match stage.outcomes.is_empty() {
            false => stage
                .outcomes
                .iter()
                .map(model::line_outcome_words)
                .collect::<Vec<_>>()
                .join(" · "),
            true => stage.next.join(", "),
        };
        if !next.is_empty() {
            detail = detail.child(
                div()
                    .text_sm()
                    .debug_selector({
                        let stage = stage.stage.clone();
                        move || format!("line next {stage}")
                    })
                    .child(format!("Next {next}")),
            );
        }
        for warning in &stage.warnings {
            detail = detail.child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .debug_selector({
                        let stage = stage.stage.clone();
                        move || format!("line warning {stage}")
                    })
                    .child(warning.clone()),
            );
        }
        if let Some(file) = &stage.stage_file {
            let key = format!(
                "stagefile:{}:{}:{}:{}",
                flow.flow_type, flow.name, flow.source, stage.stage
            );
            detail = detail.child(self.line_text_row(
                format!("Stage file · {}", file.path),
                format!("line stagefile {}", stage.stage),
                key,
                &file.text,
                None,
                palette,
                cx,
            ));
        }
        if let Some(route) = &stage.route {
            for group in &route.chain {
                if let Some(prompt) = &group.prompt {
                    let head = match prompt.source.is_empty() {
                        true => format!("Prompt · {}", prompt.path),
                        false => format!("Prompt · {} ({})", prompt.path, prompt.source),
                    };
                    detail = detail.child(self.line_prompt_row(
                        head,
                        format!(
                            "prompt:{}:{}:{}:{}:{}",
                            flow.flow_type, flow.name, flow.source, stage.stage, group.name
                        ),
                        format!("line prompt {} {}", stage.stage, group.name),
                        prompt,
                        palette,
                        cx,
                    ));
                }
            }
        }
        detail
    }

    /// The crew below the flows: one row per member, its prompt
    /// behind Read where its config names one.
    fn line_crew(
        &self,
        line: &crate::api::Line,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut crew = div().flex().flex_col().gap_2().child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(palette.faint)
                .debug_selector(|| "line crew head".to_string())
                .child("Crew"),
        );
        if line.crew.is_empty() {
            crew = crew.child(div().text_sm().text_color(palette.faint).child("No crew."));
        }
        for member in &line.crew {
            crew = crew.child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .debug_selector({
                        let name = member.name.clone();
                        move || format!("line crew {name}")
                    })
                    .child(member.name.clone()),
            );
            match &member.prompt {
                Some(prompt) => {
                    let head = match prompt.source.is_empty() {
                        true => format!("Prompt · {}", prompt.path),
                        false => format!("Prompt · {} ({})", prompt.path, prompt.source),
                    };
                    crew = crew.child(self.line_prompt_row(
                        head,
                        format!("crew:{}", member.name),
                        format!("line crew prompt {}", member.name),
                        prompt,
                        palette,
                        cx,
                    ));
                }
                None => {
                    crew = crew.child(div().text_sm().text_color(palette.faint).child("no prompt"));
                }
            }
        }
        crew
    }

    /// A prompt's row: its head, Read for its text, and Edit faint
    /// where a card can carry the prompt (slice B makes it live),
    /// with why it cannot where it cannot. No Edit where the
    /// prompt's text could not be read: its tried paths and error
    /// stand in its place.
    fn line_prompt_row(
        &self,
        head: String,
        key: String,
        selector: String,
        prompt: &LinePrompt,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let edit = match prompt.carriable {
            true => Some(("faint".to_string(), String::new())),
            false => match prompt.why.is_empty() {
                true => None,
                false => Some(("why".to_string(), prompt.why.clone())),
            },
        };
        let mut row =
            self.line_text_row(head, selector.clone(), key, &prompt.text, edit, palette, cx);
        if prompt.text.is_empty() && !prompt.error.is_empty() {
            row = row.child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("{selector} error")
                    })
                    .child(match prompt.tried.is_empty() {
                        true => prompt.error.clone(),
                        false => format!("{} (tried {})", prompt.error, prompt.tried.join(", ")),
                    }),
            );
        }
        row
    }

    /// One text's row: its head with Read, which stands its text
    /// open below, and, where `edit` stands, Edit faint beside it
    /// or the words why none stands. A stage file passes no edit.
    #[allow(clippy::too_many_arguments)]
    fn line_text_row(
        &self,
        head: String,
        selector: String,
        key: String,
        text: &str,
        edit: Option<(String, String)>,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let open = self.line_reads.contains(&key);
        let mut row = div().flex().flex_col().gap_1().child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .debug_selector({
                            let selector = selector.clone();
                            move || selector.clone()
                        })
                        .child(head),
                )
                .child(
                    small_button("Read".to_string(), palette)
                        .id(SharedString::from(format!("{selector} read")))
                        .debug_selector({
                            let selector = selector.clone();
                            move || format!("{selector} read")
                        })
                        .on_click(cx.listener({
                            let key = key.clone();
                            move |this, _, _, cx| {
                                if !this.line_reads.remove(&key) {
                                    this.line_reads.insert(key.clone());
                                }
                                cx.notify();
                            }
                        })),
                )
                .children(edit.map(|(kind, why)| {
                    let selector = selector.clone();
                    match kind.as_str() {
                        "why" => div()
                            .text_sm()
                            .text_color(palette.faint)
                            .debug_selector(move || format!("{selector} edit why"))
                            .child(why)
                            .into_any_element(),
                        _ => div()
                            .text_sm()
                            .text_color(palette.faint)
                            .debug_selector(move || format!("{selector} edit"))
                            .child("Edit")
                            .into_any_element(),
                    }
                })),
        );
        if open {
            row = row.child(
                div()
                    .text_sm()
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("{selector} text")
                    })
                    .child(text.to_string()),
            );
        }
        row
    }
}
