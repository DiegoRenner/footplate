//! The Line window: one depot's line read whole through `Line` —
//! its flows named by their source, the stage track in the flow
//! map's words, the stage detail with its Rule line, its chain, its
//! Other rules and its next stages, its stage file and prompts each
//! behind Read, and the crew prompts below. A prompt the read says a
//! card can carry has an Edit, an editor over the text that stands,
//! whose Send mails the yardmaster the proposed text: the card it
//! becomes stands in Decide, and this window approves nothing. A
//! prompt the read says no card can carry has the reason in Edit's
//! place, and a stage file has no Edit. Nothing else here writes.

use gpui::{
    AnyElement, Context, Div, Entity, Focusable, FontWeight, SharedString, Window, div, prelude::*,
    px, rems,
};

use super::Footplate;
use super::ask::card_button;
use super::panel::{buffer_stop, panel_track_line};
use super::ui::{field, shorten, small_button};
use crate::api::{Event, LineFlow, LinePrompt, LineStage};
use crate::input;
use crate::model::{self, EditNow, Model, PromptOwner, Selected};
use crate::theme::Palette;

/// What the editor says before Send, which is design yardr-y1cn's two
/// costs of filing by mail. The mail is the filing, so the text stays in
/// the mail log, and the edit waits on the yardmaster's next pass, so
/// the card will stand in Decide and nothing has changed until it is
/// approved there.
const EDITOR_WORDS: &str = "The mail is the filing, so the text stays in the mail log. The yardmaster files a card, which will stand in Decide. Nothing changes until you approve it there, and a session already running keeps the prompt it started with.";

/// What the editor says of a prompt that changed under it. The text
/// typed is over the one that was read, so a card over it is stale.
const EDITOR_MOVED: &str = "This prompt changed since you read it, so Send is held. Copy your text if you want it, cancel, and edit again over the text that stands now.";

/// How many characters of the yardmaster's answer the row shows: the
/// rest is in its mail.
const ANSWER_CHARS: usize = 600;

/// The Line window's prompt editor, while one is open: whose prompt, the
/// prompt as the read named it when the editor opened (what the mail
/// names, and what a later read is compared with) and the field the
/// proposed text is typed in. One stands at a time, and one that holds
/// typed text is never dropped by another's opening.
pub(crate) struct LineEdit {
    pub(crate) depot: String,
    pub(crate) owner: PromptOwner,
    /// The row the editor stands under: the prompt's text key.
    pub(crate) key: String,
    pub(crate) over: LinePrompt,
    pub(crate) field: Entity<input::Line>,
    /// A send is on its way: no second one goes after it.
    pub(crate) sending: bool,
    /// Why the last send did not go, in the yard's words, with what was
    /// typed still in the field.
    pub(crate) said: Option<String>,
}

impl LineEdit {
    /// Whether the field holds more than the text the editor opened over.
    fn changed(&self, cx: &gpui::App) -> bool {
        self.field.read(cx).text() != input::line_breaks(&self.over.text)
    }
}

/// The last prompt edit the Line window sent: whose, over which prompt,
/// the text, and the yardmaster's answer once one has come. It stays,
/// through reads of the line, until another is sent or it is cleared:
/// where the line is read again its prompt says whether the edit is
/// waiting, landed or moved.
pub(crate) struct LineSent {
    pub(crate) depot: String,
    pub(crate) owner: PromptOwner,
    pub(crate) over: LinePrompt,
    pub(crate) proposed: String,
    /// The newest event of the yard when the mail went: only mail after
    /// it can be the answer.
    pub(crate) after: u64,
    pub(crate) answer: Option<String>,
}

/// Whether the text typed is something to send: not blank, and not the
/// text that stands.
fn sendable(over: &str, proposed: &str) -> bool {
    !proposed.trim().is_empty() && proposed != input::line_breaks(over)
}

impl Footplate {
    /// Opens the editor over `over`, the prompt as the read named it, with
    /// the text that stands in the field and the focus in it. One editor
    /// stands at a time: Edit on the row that holds it only takes the focus
    /// back, and Edit on another, while the one that stands holds typed
    /// text, says to finish or cancel that first, so that no text typed is
    /// ever dropped for another's opening.
    pub(crate) fn edit_prompt(
        &mut self,
        depot: String,
        owner: PromptOwner,
        key: String,
        over: LinePrompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(open) = self.line_edit.as_ref() {
            if open.depot == depot && open.key == key {
                window.focus(&open.field.focus_handle(cx));
                return;
            }
            if open.changed(cx) {
                let words = format!(
                    "finish or cancel the open edit of the prompt of {} {} in {} first",
                    open.owner.kind(),
                    open.owner.name(),
                    open.depot
                );
                return self.notice(words, cx);
            }
        }
        let field = cx.new(input::Line::editor);
        field.update(cx, |line, cx| {
            line.set_text(&over.text, cx);
            // The text is read from its top, not from its end.
            line.set_cursor(0, cx);
        });
        // Send is dim while there is nothing to send: the window is drawn
        // again with what is typed.
        cx.observe(&field, |_, _, cx| cx.notify()).detach();
        window.focus(&field.focus_handle(cx));
        self.line_edit = Some(LineEdit {
            depot,
            owner,
            key,
            over,
            field,
            sending: false,
            said: None,
        });
        self.notice(String::new(), cx);
    }

    /// Shuts the editor, and what was typed in it goes with it. A send
    /// that is on its way cannot be recalled: keep the editor while it is
    /// sending, so its guard prevents another edit from sending the same
    /// mail again.
    pub(crate) fn cancel_prompt_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.line_edit.as_ref().is_some_and(|edit| edit.sending) {
            return;
        }
        if self.line_edit.take().is_some() {
            window.focus(&self.focus.clone());
        }
        cx.notify();
    }

    /// Whether the line as read says the prompt `edit` was opened over is
    /// another now, or gone. A line not read at the moment (a read is on
    /// its way) says nothing.
    fn line_edit_moved(&self, edit: &LineEdit) -> bool {
        let Some(line) = self
            .model
            .as_ref()
            .and_then(|m| m.line_read_for(&edit.depot))
        else {
            return false;
        };
        model::line_prompt_of(line, &edit.owner)
            .is_none_or(|now| model::line_prompt_moved(&edit.over, now))
    }

    /// Mails the yardmaster the proposed text, once: the mail names the
    /// depot, the group or crew member and the prompt file and digest the
    /// editor was opened over, and carries the text whole. Held while a
    /// send is on its way, while the yard is reconnecting (nothing is
    /// queued, so a mail sent as the connection broke cannot go twice),
    /// while the prompt has moved under the editor, and while there is
    /// nothing to send. A mail that went closes the editor and leaves the
    /// hand-off; one that did not leaves the editor as it was, with why.
    pub(crate) fn send_prompt_edit(&mut self, cx: &mut Context<Self>) {
        if self.reconnecting() {
            return;
        }
        let Some(edit) = self.line_edit.as_ref() else {
            return;
        };
        let proposed = edit.field.read(cx).text().to_string();
        if edit.sending || self.line_edit_moved(edit) || !sendable(&edit.over.text, &proposed) {
            return;
        }
        let (depot, owner, over) = (edit.depot.clone(), edit.owner.clone(), edit.over.clone());
        let editor = edit.field.entity_id();
        // A yard with no yardmaster to write to refuses the mail in the
        // same words: said before it goes, with the text kept.
        let unwritable = self
            .model
            .as_ref()
            .and_then(|m| m.unwritable(model::YARDMASTER));
        if let Some(edit) = self.line_edit.as_mut() {
            edit.sending = unwritable.is_none();
            edit.said = unwritable.map(|why| format!("not sent: {why}"));
            if !edit.sending {
                return cx.notify();
            }
        }
        cx.notify();
        let mail = model::line_edit_mail(&depot, &owner, &over, &proposed);
        let session = self.session;
        self.ask_through(
            cx,
            move |client, me| client.mail(model::YARDMASTER, &mail, me, &[]),
            move |this, sent, cx| {
                // A yard that is not open any more shows nothing of this.
                if this.session != session {
                    return;
                }
                let mine = |this: &Self| {
                    this.line_edit
                        .as_ref()
                        .is_some_and(|edit| edit.field.entity_id() == editor)
                };
                match sent {
                    Ok(()) => {
                        if mine(this) {
                            this.line_edit = None;
                            // The field held the focus and is not drawn
                            // any more: esc is the window's.
                            let (focus, handle) = (this.focus.clone(), this.handle);
                            cx.defer(move |cx| {
                                let _ = handle.update(cx, |_, window, _| window.focus(&focus));
                            });
                        }
                        let after = this.model.as_ref().map_or(0, Model::events_seen);
                        this.line_sent = Some(LineSent {
                            depot,
                            owner,
                            over,
                            proposed,
                            after,
                            answer: None,
                        });
                        this.change_in(session, cx, |m| m.sent(model::YARDMASTER));
                    }
                    Err(err) => {
                        if mine(this)
                            && let Some(edit) = this.line_edit.as_mut()
                        {
                            edit.sending = false;
                            edit.said = Some(format!("not sent: {err}"));
                        }
                        cx.notify();
                    }
                }
            },
        );
    }

    /// The stream told of `event`. Where it is the yardmaster's mail to the
    /// person after a prompt edit went, the hand-off shows it as it is: the
    /// answer, whether a card stands in Decide or the prompt changed since
    /// it was read. Only the first mail after the send is taken.
    pub(crate) fn line_hears(&mut self, event: &Event) {
        let (Some(sent), Some(model)) = (self.line_sent.as_mut(), self.model.as_ref()) else {
            return;
        };
        if sent.answer.is_some() || event.seq <= sent.after {
            return;
        }
        if let Some(text) = model.answer_in(event, model::YARDMASTER) {
            sent.answer = Some(text.trim().to_string());
        }
    }

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
                        body = body.child(self.line_detail(depot, flow, stage, palette, cx));
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
        depot: &str,
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
            // A stage file has no Edit, and says why (design yardr-y1cn
            // 3.2): it is part of the flow's home and changes with the flow.
            let why = div()
                .text_sm()
                .text_color(palette.faint)
                .debug_selector({
                    let stage = stage.stage.clone();
                    move || format!("line stagefile {stage} edit why")
                })
                .child("no Edit, since a stage file changes with its flow")
                .into_any_element();
            detail = detail.child(self.line_text_row(
                format!("Stage file · {}", file.path),
                format!("line stagefile {}", stage.stage),
                key,
                &file.text,
                Some(why),
                false,
                palette,
                cx,
            ));
        }
        if let Some(route) = &stage.route {
            for group in &route.chain {
                match &group.prompt {
                    Some(prompt) => {
                        let head = match prompt.source.is_empty() {
                            true => format!("Prompt · {}", model::line_path_words(&prompt.path)),
                            false => format!(
                                "Prompt · {} ({})",
                                model::line_path_words(&prompt.path),
                                prompt.source
                            ),
                        };
                        detail = detail.child(self.line_prompt_row(
                            depot,
                            PromptOwner::Group(group.name.clone()),
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
                    // A group with no prompt has none to edit: the stage
                    // says so, by what serves it, and no editor stands
                    // over a stage file in its place (design yardr-y1cn 3.2).
                    None => {
                        detail = detail.child(
                            div()
                                .text_sm()
                                .text_color(palette.faint)
                                .debug_selector({
                                    let (stage, group) = (stage.stage.clone(), group.name.clone());
                                    move || format!("line prompt {stage} {group} none")
                                })
                                .child(format!(
                                    "Prompt · {} · {}",
                                    group.name,
                                    model::line_no_prompt_words(group)
                                )),
                        );
                    }
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
                        true => format!("Prompt · {}", model::line_path_words(&prompt.path)),
                        false => format!(
                            "Prompt · {} ({})",
                            model::line_path_words(&prompt.path),
                            prompt.source
                        ),
                    };
                    crew = crew.child(self.line_prompt_row(
                        &line.depot,
                        PromptOwner::Crew(member.name.clone()),
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

    /// A prompt's row: its head, Read for its text, and beside Read an
    /// Edit where the read says a card can carry the prompt, or why one
    /// cannot where it says that. The gate is that answer alone, never
    /// the prompt's source. No Edit where the prompt's text could not be
    /// read: its tried paths and error stand in its place. Under the row
    /// stand the editor, where it is open on this prompt, and the
    /// hand-off of an edit sent for the prompt's owner.
    #[allow(clippy::too_many_arguments)]
    fn line_prompt_row(
        &self,
        depot: &str,
        owner: PromptOwner,
        head: String,
        key: String,
        selector: String,
        prompt: &LinePrompt,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let editing = self
            .line_edit
            .as_ref()
            .filter(|edit| edit.depot == depot && edit.key == key);
        let edit = match (prompt.carriable, prompt.why.is_empty()) {
            (true, _) => {
                Some(self.line_edit_control(depot, &owner, &key, &selector, prompt, cx, palette))
            }
            (false, false) => Some(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("{selector} edit why")
                    })
                    .child(prompt.why.clone())
                    .into_any_element(),
            ),
            (false, true) => None,
        };
        let mut row = self.line_text_row(
            head,
            selector.clone(),
            key,
            &prompt.text,
            edit,
            editing.is_some(),
            palette,
            cx,
        );
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
        if let Some(edit) = editing {
            row = row.child(self.line_editor(edit, &selector, palette, cx));
        }
        if let Some(sent) = self
            .line_sent
            .as_ref()
            .filter(|sent| sent.depot == depot && sent.owner == owner)
        {
            row = row.child(self.line_hand_off(sent, &selector, palette, cx));
        }
        row
    }

    /// The Edit beside a carriable prompt's Read: it opens the editor
    /// over the text the read gave.
    #[allow(clippy::too_many_arguments)]
    fn line_edit_control(
        &self,
        depot: &str,
        owner: &PromptOwner,
        key: &str,
        selector: &str,
        prompt: &LinePrompt,
        cx: &mut Context<Self>,
        palette: Palette,
    ) -> AnyElement {
        let selector = format!("{selector} edit");
        let (depot, owner, key, prompt) = (
            depot.to_string(),
            owner.clone(),
            key.to_string(),
            prompt.clone(),
        );
        small_button("Edit".to_string(), palette)
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .on_click(cx.listener(move |this, _, window, cx| {
                this.edit_prompt(
                    depot.clone(),
                    owner.clone(),
                    key.clone(),
                    prompt.clone(),
                    window,
                    cx,
                )
            }))
            .into_any_element()
    }

    /// The editor under a prompt's row: the field over the text that
    /// stands, what Send does and what comes of it, said before Send, and
    /// Cancel and Send. Send is held while the text is blank or what
    /// stands, while a send is on its way and while the prompt has moved
    /// under the editor, which says so. A send that did not go says why in
    /// red, with the text as it was.
    fn line_editor(
        &self,
        edit: &LineEdit,
        selector: &str,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let selector = format!("{selector} editor");
        let typed = edit.field.read(cx).text().to_string();
        let moved = self.line_edit_moved(edit);
        let enabled = !edit.sending && !moved && sendable(&edit.over.text, &typed);
        let over = match edit.over.digest.is_empty() {
            true => "the prompt as read".to_string(),
            false => format!(
                "the prompt as read (digest {})",
                model::line_short_digest(&edit.over.digest)
            ),
        };
        let mut editor = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                field(edit.field.focus_handle(cx), edit.field.clone(), palette)
                    .min_h(rems(8.))
                    .debug_selector({
                        let selector = selector.clone();
                        move || selector.clone()
                    }),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("{selector} words")
                    })
                    .child(format!(
                        "Send mails the yardmaster this text, whole, over {over}. {EDITOR_WORDS}"
                    )),
            );
        if moved {
            editor = editor.child(
                div()
                    .text_sm()
                    .text_color(palette.faint)
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("{selector} moved")
                    })
                    .child(EDITOR_MOVED),
            );
        }
        if let Some(said) = &edit.said {
            editor = editor.child(
                div()
                    .text_sm()
                    .text_color(palette.warn)
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("{selector} said")
                    })
                    .child(said.clone()),
            );
        }
        let send = match edit.sending {
            true => "Sending…",
            false => "Send",
        };
        let click = enabled.then(|| cx.listener(|this, _, _, cx| this.send_prompt_edit(cx)));
        editor.child(
            div()
                .flex()
                .justify_end()
                .gap_2()
                .child(
                    small_button("Cancel".to_string(), palette)
                        .id(SharedString::from(format!("{selector} cancel")))
                        .debug_selector({
                            let selector = selector.clone();
                            move || format!("{selector} cancel")
                        })
                        .on_click(
                            cx.listener(|this, _, window, cx| this.cancel_prompt_edit(window, cx)),
                        ),
                )
                .child(card_button(
                    format!("{selector} send"),
                    send.to_string(),
                    palette,
                    click,
                )),
        )
    }

    /// What stands under a prompt after an edit was sent for it: that the
    /// card will stand in Decide and that this window approves nothing,
    /// or, once the line is read again, that the prompt now reads as the
    /// text sent or changed since it was read, with the yardmaster's
    /// answer under it when one has come. Read again reads the line again;
    /// Clear takes the hand-off away.
    fn line_hand_off(
        &self,
        sent: &LineSent,
        selector: &str,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let selector = format!("{selector} sent");
        let now = self
            .model
            .as_ref()
            .and_then(|m| m.line_read_for(&sent.depot))
            .and_then(|line| model::line_prompt_of(line, &sent.owner));
        let (state, words) = match model::line_sent_now(&sent.over, &sent.proposed, now) {
            EditNow::Waiting => (
                "waiting",
                "Sent to the yardmaster. The card will stand in Decide. Nothing changes until you approve it there, and this window approves nothing.".to_string(),
            ),
            EditNow::Landed => (
                "landed",
                "The prompt now reads as the text you sent, so its card was approved.".to_string(),
            ),
            EditNow::Moved => (
                "moved",
                format!(
                    "This prompt changed since it was read. {} The yard refuses to approve a card over the old text as stale, so edit again over the text that stands.",
                    model::line_moved_words(&sent.over, now)
                ),
            ),
        };
        let depot = sent.depot.clone();
        let mut hand_off = div().flex().flex_col().gap_1().child(
            div()
                .text_sm()
                .text_color(palette.faint)
                .debug_selector({
                    let selector = selector.clone();
                    move || format!("{selector} {state}")
                })
                .child(words),
        );
        if let Some(answer) = &sent.answer {
            hand_off = hand_off.child(
                div()
                    .text_sm()
                    .debug_selector({
                        let selector = selector.clone();
                        move || format!("{selector} answer")
                    })
                    .child(format!(
                        "The yardmaster wrote: {}",
                        shorten(answer, ANSWER_CHARS)
                    )),
            );
        }
        hand_off.child(
            div()
                .flex()
                .gap_2()
                .child(
                    small_button("Read again".to_string(), palette)
                        .id(SharedString::from(format!("{selector} read")))
                        .debug_selector({
                            let selector = selector.clone();
                            move || format!("{selector} read")
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select(Selected::LineWindow(depot.clone()), window, cx)
                        })),
                )
                .child(
                    small_button("Clear".to_string(), palette)
                        .id(SharedString::from(format!("{selector} clear")))
                        .debug_selector({
                            let selector = selector.clone();
                            move || format!("{selector} clear")
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.line_sent = None;
                            cx.notify();
                        })),
                ),
        )
    }

    /// One text's row: its head with Read, which stands its text open
    /// below (unless an editor stands over it instead), and, where `edit`
    /// stands, the Edit or the reason none does, beside Read.
    #[allow(clippy::too_many_arguments)]
    fn line_text_row(
        &self,
        head: String,
        selector: String,
        key: String,
        text: &str,
        edit: Option<AnyElement>,
        editing: bool,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let open = self.line_reads.contains(&key) && !editing;
        let mut row = div().flex().flex_col().gap_1().child(
            div()
                .flex()
                .flex_row()
                .flex_wrap()
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
                .children(edit),
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

#[cfg(debug_assertions)]
impl Footplate {
    /// The editor's and the hand-off's shots, once the line is read:
    /// `line-edit` opens the editor on the first stage's first prompt a card
    /// can carry, with a change typed in it, and `line-sent` stands the
    /// hand-off of an edit written over that prompt as it stood before it
    /// changed, with the yardmaster's answer. Once; a debug build's alone.
    pub(crate) fn shot_line_window(&mut self, cx: &mut Context<Self>) {
        let shot = std::env::var("FOOTPLATE_SHOT").unwrap_or_default();
        if !matches!(shot.as_str(), "line-edit" | "line-sent")
            || self.line_edit.is_some()
            || self.line_sent.is_some()
        {
            return;
        }
        let handle = self.handle;
        cx.spawn(async move |view, cx| {
            let _ = handle.update(cx, |_, window, cx| {
                view.update(cx, |this, cx| this.shot_line_edit(&shot, window, cx))
            });
        })
        .detach();
    }

    fn shot_line_edit(&mut self, shot: &str, window: &mut Window, cx: &mut Context<Self>) {
        let found = self
            .model
            .as_ref()
            .and_then(|m| m.line_read_for("yardr"))
            .and_then(|line| {
                let flow = line.flows.first()?;
                let stage = flow.stages.first()?;
                stage.route.as_ref()?.chain.iter().find_map(|group| {
                    let prompt = group.prompt.as_ref().filter(|prompt| prompt.carriable)?;
                    let key = format!(
                        "prompt:{}:{}:{}:{}:{}",
                        flow.flow_type, flow.name, flow.source, stage.stage, group.name
                    );
                    Some((key, group.name.clone(), prompt.clone()))
                })
            });
        let Some((key, group, prompt)) = found else {
            return;
        };
        let owner = PromptOwner::Group(group);
        match shot {
            "line-edit" => {
                self.edit_prompt("yardr".into(), owner, key, prompt.clone(), window, cx);
                if let Some(edit) = self.line_edit.as_ref() {
                    let typed = format!(
                        "{}Leave the branch with one commit per piece.\n",
                        prompt.text
                    );
                    edit.field.update(cx, |line, cx| line.set_text(&typed, cx));
                }
            }
            "line-sent" => {
                let digest = "9d01c7a2e5f3b84610ad3c2f5e7b9a0c1d4e6f8a2b3c5d7e9f0a1b2c3d4e5f60";
                let over = LinePrompt {
                    path: format!("/home/jo/.yardr/roles/builders.{digest}.md"),
                    found: format!("/home/jo/.yardr/roles/builders.{digest}.md"),
                    digest: digest.into(),
                    text: "Build the thing.\n".into(),
                    ..prompt
                };
                self.line_sent = Some(LineSent {
                    depot: "yardr".into(),
                    owner,
                    over,
                    proposed: "Build the thing.\nLeave the branch with one commit per piece.\n"
                        .into(),
                    after: 0,
                    answer: Some(
                        "No card was filed: the prompt changed since it was read. Read the line again, then send the edit over the text that stands."
                            .into(),
                    ),
                });
            }
            _ => {}
        }
        // The prompt's row stands near the foot of the pane at the shot's
        // size: the editor's words and its Send, or the hand-off, are below.
        self.decision.scroll_to_bottom();
        cx.notify();
    }
}
