//! The context drawer: what shapes one stage of a depot's flow —
//! its stage file, the group that serves it and the group's prompt —
//! or one crew member — its role and what its session reads when it
//! starts — standing at the pane's right. A press on a panel station
//! or a flow strip's knot opens the stage's; the Context button of a
//! crew member's conversation head opens the member's. Either stands
//! beside the pane where the pane keeps its width past it, and over
//! the pane's right where it does not. Its Line read is the Line
//! window's, over another selection; its prompt editor, hand-off and
//! Served-by picker are the Line window's rows, drawn here.

use gpui::{
    ClipboardItem, Context, Div, FontWeight, SharedString, Stateful, Window, div, prelude::*, px,
    rems, rgba,
};

use super::Footplate;
use super::expanded_text_key;
use super::metrics::{CONTEXT_WIDTH, context_beside, grown};
use super::ui::{file_glyph, small_button};
use crate::api::{Line, LineFlow, LineGroup, LinePrompt, LineStage};
use crate::model::{self, ContextSubject, Model, PromptOwner};
use crate::select;
use crate::theme::{Palette, TYPE_BODY, TYPE_CODE, TYPE_LABEL, TYPE_META};

/// Which of the drawer's tabs stands open: the stage's three — the
/// stage file, the serving group, the group's prompt — or the crew
/// member's two — its role, what its session reads when it starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ContextTab {
    Stage,
    Group,
    Prompt,
    Role,
    Prime,
}

/// The context drawer standing at the window's right: what it stands
/// on, its tab, whether the Group tab's Served-by picker stands open,
/// and which long files stand shown whole past their fold.
pub(crate) struct ContextOpen {
    pub(crate) subject: ContextSubject,
    pub(crate) tab: ContextTab,
    pub(crate) served_open: bool,
    pub(crate) stage_whole: bool,
    pub(crate) prompt_whole: bool,
    pub(crate) role_whole: bool,
}

impl ContextOpen {
    /// The member the drawer stands on, where it stands on one's
    /// conversation rather than on a stage.
    pub(crate) fn crew_member(&self) -> Option<&str> {
        match &self.subject {
            ContextSubject::Crew { member, .. } => Some(member),
            ContextSubject::Stage { .. } => None,
        }
    }

    /// Whether the drawer stands on the stage: a press on its station
    /// keeps the drawer and its tab.
    pub(crate) fn stands_on(&self, depot: &str, flow: &str, flow_type: &str, stage: &str) -> bool {
        matches!(&self.subject, ContextSubject::Stage {
            depot: at,
            flow: at_flow,
            flow_type: at_type,
            stage: at_stage,
        } if at == depot && at_flow == flow && at_type == flow_type && at_stage == stage)
    }
}

/// How many lines of a stage file or a prompt stand before the fold:
/// past these the drawer says how many more stand, and the whole file
/// stands behind one tap.
const FOLDED_LINES: usize = 30;

/// A long file folded: its first `shown` lines whole, with how many
/// lines stand past them. `None` where the file stands whole already.
fn folded(text: &str, shown: usize) -> Option<(&str, usize)> {
    let mut rest = text.split_inclusive('\n');
    let mut end = 0;
    for _ in 0..shown {
        end += rest.next()?.len();
    }
    match rest.count() {
        0 => None,
        hidden => Some((&text[..end], hidden)),
    }
}

/// A herdr group's harness as the Group tab says it: its kind with its
/// model, each where the config names it, and the harness's own where
/// it names neither.
fn harness_words(group: &LineGroup) -> String {
    match (group.kind.is_empty(), group.model.is_empty()) {
        (false, false) => format!("{} · model {}", group.kind, group.model),
        (false, true) => group.kind.clone(),
        (true, false) => format!("model {}", group.model),
        (true, true) => "the harness's own".to_string(),
    }
}

/// The Group tab's second fact, by what serves the stage: a herdr
/// group's harness with its model, an exec group's command whole, a
/// manual group's members, and a peer group's yard. `None` where the
/// runner names nothing to say.
fn runner_detail(group: &LineGroup) -> Option<(&'static str, String)> {
    match group.runner.as_str() {
        "herdr" => Some(("harness", harness_words(group))),
        "exec" if !group.cmd.is_empty() => Some(("command", group.cmd.clone())),
        "manual" => Some((
            "members",
            match group.members.is_empty() {
                true => "anyone may move a bead on".to_string(),
                false => group.members.join(", "),
            },
        )),
        "peer" if !group.yard.is_empty() => Some(("yard", group.yard.clone())),
        _ => None,
    }
}

/// A prompt's file line: its source where the read names one, its lines
/// counted, and whose sessions start with it.
fn prompt_meta(source: &str, lines: usize, group: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !source.is_empty() {
        parts.push(source.to_string());
    }
    match lines {
        1 => parts.push("1 line".to_string()),
        lines => parts.push(format!("{lines} lines")),
    }
    parts.push(format!("the prompt {group} starts each session with"));
    parts.join(" · ")
}

/// What the Group tab says serves the stage, before the Served-by tap:
/// the stage of the depot's flow.
fn serves_words(depot: &str, flow: &str, stage: &str) -> String {
    format!("serves stage {stage} of {depot}'s {flow}")
}

/// What the Group tab says where no route serves the stage: a bead
/// here waits for a person to move it.
fn unserved_words(depot: &str, flow: &str, stage: &str) -> String {
    format!(
        "No group serves {stage} in {depot}'s {flow} flow: a bead here waits for a person to move it."
    )
}

/// A file's lines counted, as a file line says them.
fn counted_lines(lines: usize) -> String {
    match lines {
        1 => "1 line".to_string(),
        lines => format!("{lines} lines"),
    }
}

/// A role's file line: the depot whose line it was read with, its
/// lines counted, and when its member's session reads it.
fn role_meta(depot: &str, lines: usize) -> String {
    format!(
        "depot {depot} · {} · read when its session starts",
        counted_lines(lines)
    )
}

/// What the Prime tab says a member's session reads when it starts, in
/// order: the yard's own command for it.
fn prime_command(member: &str) -> String {
    format!("yardr prime --crew {member}")
}

/// The Prime tab's second item, where the settings carry either of the
/// operator's words: their values, and whose settings they are. `None`
/// where the answer carries neither, and the Prime tab shows the role
/// alone.
fn prime_operator_words(style: &str, note: &str) -> Option<(String, String)> {
    let values = [style, note]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    if values.is_empty() {
        return None;
    }
    let whose = match (!style.is_empty(), !note.is_empty()) {
        (true, true) => "the yard's settings · every session reads both",
        _ => "the yard's settings",
    };
    Some((values, whose.to_string()))
}

/// What an edit does, under Propose an edit on either drawer's prompt:
/// one mail to the yardmaster, and the change back as a card in
/// Decide.
const EDIT_NOTE: &str = "An edit goes to the yardmaster as one mail with the whole text, the file and the digest as read; the change comes back to you as a card in Decide.";

impl Footplate {
    /// Stands the context drawer open on `stage` of the flow, and reads
    /// the depot's line where the one held is another depot's or none.
    /// One right-side panel shows: the bead drawer shuts with it. A
    /// press on the station it stands on keeps its tab and its place.
    pub(crate) fn open_context(
        &mut self,
        depot: String,
        flow: String,
        flow_type: String,
        stage: String,
        cx: &mut Context<Self>,
    ) {
        self.drawer = None;
        self.change(cx, |m| m.open_context(&depot, &flow, &flow_type, &stage));
        let subject = ContextSubject::Stage {
            depot,
            flow,
            flow_type,
            stage,
        };
        let same = self
            .context
            .as_ref()
            .is_some_and(|open| open.subject == subject);
        if !same {
            self.context_scroll.scroll_to_item(0);
            self.context = Some(ContextOpen {
                subject,
                tab: ContextTab::Stage,
                served_open: false,
                stage_whole: false,
                prompt_whole: false,
                role_whole: false,
            });
        }
        cx.notify();
    }

    /// Stands the context drawer open on the crew member's
    /// conversation, and reads the first depot's line where the one
    /// held is another depot's or none: the crew's prompts read the
    /// same whatever depot names the call. One right-side panel shows:
    /// the bead drawer shuts with it, as does a stage's drawer. A
    /// press on the button it stands on shuts it again; another press
    /// reads again, as a failed read's hint says.
    pub(crate) fn open_crew_context(&mut self, member: String, cx: &mut Context<Self>) {
        if self.context.as_ref().and_then(ContextOpen::crew_member) == Some(member.as_str()) {
            self.shut_context(cx);
            return;
        }
        let depot = self
            .model
            .as_ref()
            .and_then(|model| model.depots().first().map(|depot| depot.name.clone()))
            .unwrap_or_default();
        self.drawer = None;
        self.change(cx, |m| m.open_crew_context(&depot, &member));
        self.context_scroll.scroll_to_item(0);
        self.context = Some(ContextOpen {
            subject: ContextSubject::Crew { depot, member },
            tab: ContextTab::Role,
            served_open: false,
            stage_whole: false,
            prompt_whole: false,
            role_whole: false,
        });
        cx.notify();
    }

    /// The conversation head's Context button: the member's drawer open
    /// or shut. It reads in the text colour over the accent's edge
    /// while the drawer stands on this conversation.
    pub(crate) fn context_button(
        &self,
        with: &str,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let on = self.context.as_ref().and_then(ContextOpen::crew_member) == Some(with);
        let color = match on {
            true => palette.text,
            false => palette.faint,
        };
        let member = with.to_string();
        let selector = format!("talk context {with}");
        div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .h(px(28.))
            .px_2p5()
            .flex()
            .flex_row()
            .items_center()
            .gap_1p5()
            .rounded_md()
            .border_1()
            .border_color(match on {
                true => palette.accent,
                false => palette.strong,
            })
            .text_size(rems(TYPE_META))
            .font_weight(FontWeight::MEDIUM)
            .text_color(color)
            .cursor_pointer()
            .hover(|style| style.bg(palette.hover))
            .child(file_glyph(14., color))
            .child("Context")
            .on_click(cx.listener(move |this, _, _, cx| this.open_crew_context(member.clone(), cx)))
    }

    /// Shuts the context drawer: a Line answer for its depot says
    /// nothing after this.
    pub(crate) fn shut_context(&mut self, cx: &mut Context<Self>) {
        self.context = None;
        self.change(cx, |m| {
            m.shut_context();
            Vec::new()
        });
    }

    /// Reads the context drawer's depot again: the hand-off's Read
    /// again, which says where an edit stands against the text that
    /// stands now.
    pub(crate) fn reread_context(&mut self, cx: &mut Context<Self>) {
        self.change(cx, |m| m.reread_context());
    }

    /// Opens the drawer's `tab`, from its top.
    pub(crate) fn context_tab(&mut self, tab: ContextTab, cx: &mut Context<Self>) {
        if let Some(open) = self.context.as_mut() {
            open.tab = tab;
        }
        self.context_scroll.scroll_to_item(0);
        cx.notify();
    }

    /// Stands the Group tab's Served-by picker open, or shuts it again:
    /// the Served-by tap.
    pub(crate) fn context_served(&mut self, cx: &mut Context<Self>) {
        if let Some(open) = self.context.as_mut() {
            open.served_open = !open.served_open;
        }
        cx.notify();
    }

    /// Shows the open tab's file whole past its fold: the file's tap.
    /// The fold stands again on another stage.
    pub(crate) fn context_unfold(&mut self, cx: &mut Context<Self>) {
        if let Some(open) = self.context.as_mut() {
            match open.tab {
                ContextTab::Stage => open.stage_whole = true,
                ContextTab::Prompt => open.prompt_whole = true,
                ContextTab::Role => open.role_whole = true,
                ContextTab::Group | ContextTab::Prime => {}
            }
        }
        cx.notify();
    }

    /// The drawer's flow as the depot's line reads it, by the name and
    /// type the drawer stands on. None where the drawer stands on a
    /// crew member, which no flow carries.
    fn context_flow<'a>(open: &ContextOpen, model: &'a Model) -> Option<&'a LineFlow> {
        let ContextSubject::Stage {
            depot,
            flow,
            flow_type,
            ..
        } = &open.subject
        else {
            return None;
        };
        model.line_read_for(depot).and_then(|line| {
            line.flows
                .iter()
                .find(|read| read.name == *flow && read.flow_type == *flow_type)
        })
    }

    /// The drawer's stage as the depot's line reads it.
    fn context_stage<'a>(open: &ContextOpen, model: &'a Model) -> Option<&'a LineStage> {
        let ContextSubject::Stage { stage, .. } = &open.subject else {
            return None;
        };
        Self::context_flow(open, model)
            .and_then(|flow| flow.stages.iter().find(|read| read.stage == *stage))
    }

    /// The drawer's line as read for the depot it stands on, either
    /// subject's.
    fn context_line<'a>(open: &ContextOpen, model: &'a Model) -> Option<&'a Line> {
        model.line_read_for(open.subject.depot())
    }

    /// The drawer's member's role prompt as the depot's line reads it:
    /// the crew member's own. `None` where the read has none, as where
    /// the member's config names no prompt.
    fn context_role<'a>(open: &ContextOpen, model: &'a Model) -> Option<&'a LinePrompt> {
        let ContextSubject::Crew { member, .. } = &open.subject else {
            return None;
        };
        Self::context_line(open, model)
            .and_then(|line| model::line_prompt_of(line, &PromptOwner::Crew(member.clone())))
    }

    /// The context drawer at the window's right: one stage's file,
    /// its serving group and the group's prompt under three tabs, or
    /// one crew member's role and prime under two. None where no
    /// station stands pressed and no Context button. It stands beside
    /// the pane where the pane keeps its width past it, and over the
    /// pane's right where it does not; the list stays in reach either
    /// way.
    pub(crate) fn context_drawer(
        &self,
        model: &Model,
        palette: Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement> {
        let open = self.context.as_ref()?;
        // The window's drawer and the model's agree: the drawer reads
        // the model's line for the depot they name together.
        if model.context() != Some(&open.subject) {
            return None;
        }
        let beside = context_beside(window.viewport_size().width, f32::from(self.settings.rem()));
        let mut drawer = grown(CONTEXT_WIDTH)
            .h_full()
            .flex()
            .flex_col()
            .bg(palette.raised)
            .border_l_1()
            .border_color(palette.line)
            .id("context drawer")
            .debug_selector(|| "context drawer".to_string());
        drawer = match beside {
            true => drawer.flex_none(),
            // Over the pane's right: the pane keeps what the drawer
            // leaves, and the list beside it stays in reach.
            false => drawer.absolute().top(px(0.)).right(px(0.)).bottom(px(0.)),
        };
        let mut drawer = drawer
            .child(self.context_head(open, palette, cx))
            .child(self.context_tabs(open, model, palette, cx));
        // The stage's tabs stand on its stage as read; the member's on
        // its depot's line, and say themselves where the read has no
        // role prompt.
        let (body, footer): (Div, Option<Div>) = match &open.subject {
            ContextSubject::Stage { .. } => match Self::context_stage(open, model) {
                Some(stage) => (
                    self.context_body(open, model, stage, palette, cx),
                    self.context_footer(open, model, stage, palette, cx),
                ),
                None => (self.context_empty(open, model, palette), None),
            },
            ContextSubject::Crew { .. } => match Self::context_line(open, model) {
                Some(_) => (
                    self.context_crew_body(open, model, palette, cx),
                    self.context_crew_footer(open, model, palette, cx),
                ),
                None => (self.context_empty(open, model, palette), None),
            },
        };
        drawer = drawer.child(
            div()
                .id("context body")
                .debug_selector(|| "context body".to_string())
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.context_scroll)
                .child(body),
        );
        drawer = drawer.children(footer);
        Some(drawer)
    }

    /// The drawer's head: what it stands on — the depot, the flow
    /// and the stage, or the member's context — with its ×.
    fn context_head(
        &self,
        open: &ContextOpen,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (kicker, name): (&'static str, String) = match &open.subject {
            ContextSubject::Stage {
                depot,
                flow,
                flow_type,
                stage,
            } => (
                "stage",
                match flow_type.is_empty() {
                    true => format!("{depot} · {flow} · {stage}"),
                    false => format!("{depot} · {flow_type} · {flow} · {stage}"),
                },
            ),
            ContextSubject::Crew { member, .. } => ("crew", format!("{member}'s context")),
        };
        div()
            .flex_none()
            .h(px(52.))
            .pl_4()
            .pr_2()
            .border_b_1()
            .border_color(palette.line)
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .child(
                div()
                    .debug_selector(|| "context kind".to_string())
                    .rounded(px(4.))
                    .border_1()
                    .border_color(palette.line)
                    .px_1()
                    .text_size(rems(TYPE_CODE))
                    .text_color(palette.faint)
                    .child({
                        let mut kicker = div().child(kicker);
                        if let Some(mono) = select::mono(cx) {
                            kicker = kicker.font_family(mono);
                        }
                        kicker
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .debug_selector(|| "context head".to_string())
                    .text_size(rems(TYPE_LABEL))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(palette.text)
                    .child(name),
            )
            .child(
                div()
                    .id("context close")
                    .debug_selector(|| "context close".to_string())
                    .text_xl()
                    .text_color(palette.faint)
                    .cursor_pointer()
                    .child("×")
                    .on_click(cx.listener(|this, _, _, cx| this.shut_context(cx))),
            )
    }

    /// The drawer's tabs: the stage file, the serving group and the
    /// group's prompt, or the member's role and its prime with how many
    /// shape it. The open one reads in the text colour over the
    /// accent's edge.
    fn context_tabs(
        &self,
        open: &ContextOpen,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let prime = match prime_operator_words(model.operator_style(), model.operator_note()) {
            Some(_) => "Prime · 2".to_string(),
            None => "Prime · 1".to_string(),
        };
        let listed: Vec<(ContextTab, String)> = match &open.subject {
            ContextSubject::Stage { .. } => vec![
                (ContextTab::Stage, "Stage".into()),
                (ContextTab::Group, "Group".into()),
                (ContextTab::Prompt, "Prompt".into()),
            ],
            ContextSubject::Crew { .. } => {
                vec![
                    (ContextTab::Role, "Role".into()),
                    (ContextTab::Prime, prime),
                ]
            }
        };
        let mut tabs = div()
            .flex_none()
            .flex()
            .flex_row()
            .gap_1()
            .px_3()
            .border_b_1()
            .border_color(palette.line);
        for (tab, label) in listed {
            let on = open.tab == tab;
            let name = match tab {
                ContextTab::Stage => "stage",
                ContextTab::Group => "group",
                ContextTab::Prompt => "prompt",
                ContextTab::Role => "role",
                ContextTab::Prime => "prime",
            };
            let selector = format!("context tab {name}");
            tabs = tabs.child(
                div()
                    .id(SharedString::from(selector.clone()))
                    .debug_selector({
                        let selector = selector.clone();
                        move || selector.clone()
                    })
                    .h(px(34.))
                    .px_2()
                    .mb(px(-1.))
                    .flex()
                    .items_center()
                    .text_size(rems(TYPE_META))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(match on {
                        true => palette.text,
                        false => palette.faint,
                    })
                    .border_b_2()
                    .border_color(match on {
                        true => palette.accent,
                        false => rgba(0x00000000),
                    })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.context_tab(tab, cx)))
                    .child(label),
            );
        }
        tabs
    }

    /// The drawer's body where the line has nothing for it: the flow or
    /// the stage is gone from the line, the line is still being read,
    /// its read failed, the yard is too old for it, or the window is
    /// not the operator's. The member's, where the yard serves no depot
    /// to read the line of.
    fn context_empty(&self, open: &ContextOpen, model: &Model, palette: Palette) -> Div {
        let depot = open.subject.depot();
        let words = match &open.subject {
            ContextSubject::Stage { .. } if model.line_read_for(depot).is_some() => {
                match Self::context_flow(open, model) {
                    Some(_) => "The stage is gone.",
                    None => "The flow is gone.",
                }
            }
            ContextSubject::Crew { .. } if depot.is_empty() => {
                "The yard has no depot to read the line of."
            }
            _ if model.line_unserved() => model::LINE_TOO_OLD,
            _ if !model.is_operator() => "The context drawer is the operator's.",
            _ => model.line_notice().unwrap_or("Reading the line…"),
        };
        let mut empty = div().flex().flex_col().gap_2().p_4().child(
            div()
                .debug_selector(|| "context empty".to_string())
                .text_color(palette.faint)
                .child(words.to_string()),
        );
        if !depot.is_empty()
            && model.line_read_for(depot).is_none()
            && !model.line_unserved()
            && model.is_operator()
            && model.line_notice().is_some()
        {
            let retry = match &open.subject {
                ContextSubject::Stage { .. } => "Press the station again to try again.",
                ContextSubject::Crew { .. } => "Press Context again to try again.",
            };
            empty = empty.child(
                div()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .debug_selector(|| "context retry".to_string())
                    .child(retry),
            );
        }
        empty
    }

    /// A stage drawer's body for the open tab: the stage file, the
    /// serving group, or the group's prompt.
    fn context_body(
        &self,
        open: &ContextOpen,
        model: &Model,
        stage: &LineStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        match open.tab {
            ContextTab::Stage => self.context_stage_tab(open, model, stage, palette, cx),
            ContextTab::Group => self.context_group_tab(open, model, stage, palette, cx),
            ContextTab::Prompt => self.context_prompt_tab(open, model, stage, palette, cx),
            // No crew tab stands on a stage.
            ContextTab::Role | ContextTab::Prime => div(),
        }
    }

    /// A crew drawer's body for the open tab: the member's role, or
    /// what its session reads when it starts.
    fn context_crew_body(
        &self,
        open: &ContextOpen,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        match open.tab {
            ContextTab::Role => self.context_role_tab(open, model, palette, cx),
            ContextTab::Prime => self.context_prime_tab(open, model, palette),
            // No stage tab stands on a member.
            _ => div(),
        }
    }

    /// A tab's file line: the path in code over what names it — the
    /// depot and the flow's source for a stage file, the source, the
    /// lines and whose sessions for a prompt.
    fn context_file_meta(
        &self,
        selector: &str,
        path: &str,
        meta: &str,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_4()
            .py_2p5()
            .border_b_1()
            .border_color(palette.line)
            .child(
                div()
                    .debug_selector({
                        let selector = selector.to_string();
                        move || selector.clone()
                    })
                    .text_size(rems(TYPE_CODE))
                    .text_color(palette.text)
                    .child({
                        let mut code = div().child(path.to_string());
                        if let Some(mono) = select::mono(cx) {
                            code = code.font_family(mono);
                        }
                        code
                    }),
            )
            .child(
                div()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .child(meta.to_string()),
            )
    }

    /// A tab's file rendered as the window's Markdown, folded past
    /// [`FOLDED_LINES`] lines behind one tap, with the whole file where
    /// `whole` stands.
    fn context_file(
        &self,
        key: u64,
        selector: &str,
        text: &str,
        whole: bool,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut file = div()
            .px_4()
            .py_3()
            .text_size(rems(TYPE_BODY))
            .text_color(palette.text);
        let shown = match whole {
            true => None,
            false => folded(text, FOLDED_LINES),
        };
        let (text, hidden) = match shown {
            Some((shown, hidden)) => (shown, hidden),
            None => (text, 0),
        };
        file = file.child(select::text(
            expanded_text_key(key, selector, 0, text),
            text.to_string(),
            &self.selection,
        ));
        if hidden > 0 {
            let words = match hidden {
                1 => "1 more line".to_string(),
                hidden => format!("{hidden} more lines"),
            };
            file = file.child(
                div()
                    .pt_2()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_baseline()
                    .gap_1()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .child(words)
                    .child("·")
                    .child(
                        div()
                            .id(SharedString::from(format!("{selector} whole")))
                            .debug_selector({
                                let selector = selector.to_string();
                                move || format!("{selector} whole")
                            })
                            .text_color(palette.link)
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| this.context_unfold(cx)))
                            .child("Show the whole file"),
                    ),
            );
        }
        file
    }

    /// The Stage tab: the stage file rendered as the window's Markdown,
    /// read-only until a later bead proposes edits to it.
    fn context_stage_tab(
        &self,
        open: &ContextOpen,
        model: &Model,
        stage: &LineStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let ContextSubject::Stage { depot, flow, .. } = &open.subject else {
            return div();
        };
        let Some(file) = stage.stage_file.as_ref() else {
            return div().p_4().child(
                div()
                    .debug_selector(|| "context stage none".to_string())
                    .text_color(palette.faint)
                    .child("No stage file."),
            );
        };
        let read = Self::context_flow(open, model);
        let mut meta = format!("depot {depot}");
        if let Some(source) = read
            .map(|flow| flow.source.as_str())
            .filter(|source| !source.is_empty())
        {
            meta.push_str(&format!(" · {source}"));
        }
        let key = select::key(("context stage", depot, flow, &stage.stage));
        div()
            .flex()
            .flex_col()
            .child(self.context_file_meta("context stage file", &file.path, &meta, palette, cx))
            .child(self.context_file(
                key,
                "context stage",
                &file.text,
                open.stage_whole,
                palette,
                cx,
            ))
    }

    /// The Group tab: the route's chain — the serving group's runner,
    /// harness and seats, its members and its prompt — with Served by…
    /// to change what serves the stage.
    fn context_group_tab(
        &self,
        open: &ContextOpen,
        model: &Model,
        stage: &LineStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let ContextSubject::Stage { depot, flow, .. } = &open.subject else {
            return div();
        };
        let Some(route) = stage.route.as_ref() else {
            return match stage.terminal {
                true => div().m_3().child(
                    div()
                        .debug_selector(|| "context group end".to_string())
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .rounded_md()
                        .border_1()
                        .border_color(palette.line)
                        .p_3()
                        .child(format!(
                            "No group serves {}: entering it closes the bead.",
                            stage.stage
                        )),
                ),
                false => self.context_unserved(open, model, stage, palette, cx),
            };
        };
        let Some(group) = route.chain.first() else {
            return self.context_unserved(open, model, stage, palette, cx);
        };
        let read = Self::context_flow(open, model);
        let heading = read
            .map(model::line_flow_heading)
            .unwrap_or_else(|| flow.clone());
        let mut tab = div().flex().flex_col().child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .px_4()
                .py_2p5()
                .border_b_1()
                .border_color(palette.line)
                .child(
                    div()
                        .debug_selector(|| "context group name".to_string())
                        .text_size(rems(TYPE_CODE))
                        .text_color(palette.text)
                        .child({
                            let mut code = div().child(group.name.clone());
                            if let Some(mono) = select::mono(cx) {
                                code = code.font_family(mono);
                            }
                            code
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_baseline()
                        .gap_1()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child(serves_words(depot, &heading, &stage.stage))
                        .child("·")
                        .child(
                            div()
                                .id("context served")
                                .debug_selector(|| "context served".to_string())
                                .text_color(palette.link)
                                .cursor_pointer()
                                .on_click(cx.listener(|this, _, _, cx| this.context_served(cx)))
                                .child("Served by… (change the route)"),
                        ),
                ),
        );
        tab = tab.child(self.context_facts(open, model, group, palette));
        for group in route.chain.iter().skip(1) {
            let mut words = format!("then {} · {}", group.name, model::line_group_words(group));
            if !group.when.is_empty() {
                words.push_str(&format!(" where {}", group.when));
            }
            tab = tab.child(
                div()
                    .px_4()
                    .pt_2()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.text)
                    .debug_selector({
                        let group = group.name.clone();
                        move || format!("context chain {group}")
                    })
                    .child(words),
            );
        }
        if open.served_open {
            let groups = model
                .line_read_for(depot)
                .map(model::line_groups)
                .unwrap_or_default();
            tab = tab.child(div().px_4().py_3().child(self.line_served(
                depot,
                &stage.stage,
                Some((&route.id, &route.scope, &route.filters)),
                &group.name,
                &groups,
                false,
                palette,
                cx,
            )));
        }
        if let Some(filed) = self
            .line_filed
            .as_ref()
            .filter(|f| f.depot == *depot && (f.stage.is_empty() || f.stage == stage.stage))
        {
            tab = tab.child(
                div()
                    .px_4()
                    .pb_3()
                    .child(self.line_filed_row(filed, palette, cx)),
            );
        }
        tab
    }

    /// The Group tab where no route serves the stage: the warn, and
    /// Served by… to pick a group for it, whose route card the
    /// yardmaster files.
    fn context_unserved(
        &self,
        open: &ContextOpen,
        model: &Model,
        stage: &LineStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let ContextSubject::Stage { depot, flow, .. } = &open.subject else {
            return div();
        };
        let read = Self::context_flow(open, model);
        let heading = read
            .map(model::line_flow_heading)
            .unwrap_or_else(|| flow.clone());
        let mut tab = div().flex().flex_col().child(
            div()
                .m_3()
                .rounded_md()
                .border_1()
                .border_color(palette.accent)
                .p_3()
                .text_size(rems(TYPE_META))
                .text_color(palette.text)
                .debug_selector(|| "context unserved".to_string())
                .child(unserved_words(depot, &heading, &stage.stage)),
        );
        tab = tab.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .px_4()
                .pb_3()
                .child(
                    small_button("Served by…".to_string(), palette)
                        .id("context served")
                        .debug_selector(|| "context served".to_string())
                        .on_click(cx.listener(|this, _, _, cx| this.context_served(cx))),
                )
                .child(
                    div()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .debug_selector(|| "context served note".to_string())
                        .child(
                            "Pick a group to mark it; Send mails the yardmaster the route card, which comes back as a card in Decide.",
                        ),
                ),
        );
        if open.served_open {
            let groups = model
                .line_read_for(depot)
                .map(model::line_groups)
                .unwrap_or_default();
            tab = tab.child(div().px_4().pb_3().child(self.line_served(
                depot,
                &stage.stage,
                None,
                "",
                &groups,
                false,
                palette,
                cx,
            )));
        }
        if let Some(filed) = self
            .line_filed
            .as_ref()
            .filter(|f| f.depot == *depot && (f.stage.is_empty() || f.stage == stage.stage))
        {
            tab = tab.child(
                div()
                    .px_4()
                    .pb_3()
                    .child(self.line_filed_row(filed, palette, cx)),
            );
        }
        tab
    }

    /// One chain group's facts: its runner, what the runner runs on,
    /// its seats, its members and its prompt.
    fn context_facts(
        &self,
        open: &ContextOpen,
        model: &Model,
        group: &LineGroup,
        palette: Palette,
    ) -> Div {
        let mut facts = div().flex().flex_col().gap_2().px_4().py_3();
        let row = |label: &str, value: Div| {
            div()
                .flex()
                .flex_row()
                .gap_3()
                .child(
                    div()
                        .w(px(96.))
                        .flex_none()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child(label.to_string()),
                )
                .child(value)
        };
        facts = facts.child(row(
            "runner",
            div()
                .flex_1()
                .min_w_0()
                .text_size(rems(TYPE_META))
                .text_color(palette.text)
                .debug_selector(|| "context facts runner".to_string())
                .child(format!(
                    "{} · {}",
                    group.runner,
                    model::line_runner_words(&group.runner)
                )),
        ));
        if let Some((label, value)) = runner_detail(group) {
            let value = match label {
                "command" => div()
                    .flex_1()
                    .min_w_0()
                    .text_size(rems(TYPE_CODE))
                    .text_color(palette.text)
                    .child(value),
                _ => div()
                    .flex_1()
                    .min_w_0()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.text)
                    .child(value),
            };
            facts = facts.child(row(
                label,
                value.debug_selector(|| "context facts runner detail".to_string()),
            ));
        }
        let seats = match &open.subject {
            ContextSubject::Stage {
                depot,
                flow,
                flow_type,
                stage,
            } => model
                .flow(depot, flow, flow_type)
                .and_then(|read| read.stages.iter().find(|read| read.stage == *stage))
                .and_then(|read| model::context_seats_words(read.limit, read.taken)),
            ContextSubject::Crew { .. } => None,
        };
        if let Some(seats) = seats {
            facts = facts.child(row(
                "seats",
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.text)
                    .debug_selector(|| "context facts seats".to_string())
                    .child(seats),
            ));
        }
        if !group.members.is_empty() && group.runner != "manual" {
            facts = facts.child(row(
                "members",
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.text)
                    .debug_selector(|| "context facts members".to_string())
                    .child(group.members.join(", ")),
            ));
        }
        match group.prompt.as_ref() {
            Some(prompt) => {
                facts = facts.child(row(
                    "prompt",
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(rems(TYPE_CODE))
                        .text_color(palette.text)
                        .debug_selector(|| "context facts prompt".to_string())
                        .child(model::line_path_words(&prompt.path)),
                ));
            }
            None => {
                facts = facts.child(row(
                    "prompt",
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .debug_selector(|| "context facts prompt".to_string())
                        .child(model::line_no_prompt_words(group).to_string()),
                ));
            }
        }
        facts
    }

    /// The Prompt tab: the serving group's prompt rendered as the
    /// window's Markdown, with the Line window's editor and hand-off
    /// under it where an edit stands open or went.
    fn context_prompt_tab(
        &self,
        open: &ContextOpen,
        model: &Model,
        stage: &LineStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let ContextSubject::Stage { depot, flow, .. } = &open.subject else {
            return div();
        };
        let group = stage.route.as_ref().and_then(|route| route.chain.first());
        let prompt = group.and_then(|group| group.prompt.as_ref());
        let (Some(group), Some(prompt)) = (group, prompt) else {
            let words = match group {
                Some(group) => model::line_no_prompt_words(group).to_string(),
                None => "No group serves this stage, so there is no prompt.".to_string(),
            };
            return div().p_4().child(
                div()
                    .debug_selector(|| "context prompt none".to_string())
                    .text_color(palette.faint)
                    .child(words),
            );
        };
        let key = select::key(("context prompt", depot, flow, &stage.stage));
        let mut tab = div().flex().flex_col().child(self.context_file_meta(
            "context prompt file",
            &model::line_path_words(&prompt.path),
            &prompt_meta(&prompt.source, prompt.text.lines().count(), &group.name),
            palette,
            cx,
        ));
        tab = tab.child(self.context_file(
            key,
            "context prompt",
            &prompt.text,
            open.prompt_whole,
            palette,
            cx,
        ));
        if prompt.text.is_empty() && !prompt.error.is_empty() {
            tab = tab.child(
                div()
                    .px_4()
                    .pb_3()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .debug_selector(|| "context prompt error".to_string())
                    .child(match prompt.tried.is_empty() {
                        true => prompt.error.clone(),
                        false => format!("{} (tried {})", prompt.error, prompt.tried.join(", ")),
                    }),
            );
        }
        let read = Self::context_flow(open, model);
        let edit_key = read
            .map(|flow| {
                format!(
                    "prompt:{}:{}:{}:{}:{}",
                    flow.flow_type, flow.name, flow.source, stage.stage, group.name
                )
            })
            .unwrap_or_default();
        if let Some(edit) = self
            .line_edit
            .as_ref()
            .filter(|edit| edit.depot == *depot && edit.key == edit_key)
        {
            tab = tab.child(div().px_4().pb_3().child(self.line_editor(
                edit,
                "context prompt",
                palette,
                cx,
            )));
        }
        if let Some(sent) = self.line_sent.as_ref().filter(|sent| {
            sent.depot == *depot && sent.owner == PromptOwner::Group(group.name.clone())
        }) {
            tab = tab.child(div().px_4().pb_3().child(self.line_hand_off(
                sent,
                "context prompt",
                palette,
                true,
                cx,
            )));
        }
        tab
    }

    /// The Role tab: the member's role prompt rendered as the window's
    /// Markdown, with the Line window's editor and hand-off under it
    /// where an edit stands open or went.
    fn context_role_tab(
        &self,
        open: &ContextOpen,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Div {
        let ContextSubject::Crew { depot, member } = &open.subject else {
            return div();
        };
        let Some(prompt) = Self::context_role(open, model) else {
            return div().p_4().child(
                div()
                    .debug_selector(|| "context role none".to_string())
                    .text_color(palette.faint)
                    .child("No role prompt."),
            );
        };
        let key = select::key(("context role", depot, member));
        let mut tab = div().flex().flex_col().child(self.context_file_meta(
            "context role file",
            &model::line_path_words(&prompt.path),
            &role_meta(depot, prompt.text.lines().count()),
            palette,
            cx,
        ));
        tab = tab.child(self.context_file(
            key,
            "context role",
            &prompt.text,
            open.role_whole,
            palette,
            cx,
        ));
        if prompt.text.is_empty() && !prompt.error.is_empty() {
            tab = tab.child(
                div()
                    .px_4()
                    .pb_3()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .debug_selector(|| "context role error".to_string())
                    .child(match prompt.tried.is_empty() {
                        true => prompt.error.clone(),
                        false => format!("{} (tried {})", prompt.error, prompt.tried.join(", ")),
                    }),
            );
        }
        let owner = PromptOwner::Crew(member.clone());
        let edit_key = format!("crew:{member}");
        if let Some(edit) = self
            .line_edit
            .as_ref()
            .filter(|edit| edit.depot == *depot && edit.key == edit_key)
        {
            tab = tab.child(div().px_4().pb_3().child(self.line_editor(
                edit,
                "context role",
                palette,
                cx,
            )));
        }
        if let Some(sent) = self
            .line_sent
            .as_ref()
            .filter(|sent| sent.depot == *depot && sent.owner == owner)
        {
            tab = tab.child(div().px_4().pb_3().child(self.line_hand_off(
                sent,
                "context role",
                palette,
                true,
                cx,
            )));
        }
        tab
    }

    /// The Prime tab: what is known to shape the member's session, in
    /// the order it reads it — its role, then the operator's style and
    /// note where the settings carry them — with the yard's command
    /// for the whole.
    fn context_prime_tab(&self, open: &ContextOpen, model: &Model, palette: Palette) -> Div {
        let ContextSubject::Crew { depot, member } = &open.subject else {
            return div();
        };
        let prompt = Self::context_role(open, model);
        let tab = div().flex().flex_col().child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .px_4()
                .py_2p5()
                .border_b_1()
                .border_color(palette.line)
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_baseline()
                        .gap_1()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .debug_selector(|| "context prime about".to_string())
                        .child(format!(
                            "What {member}'s session reads when it starts, in order:"
                        ))
                        .child(
                            div()
                                .text_size(rems(TYPE_CODE))
                                .text_color(palette.text)
                                .child(prime_command(member)),
                        ),
                ),
        );
        let mut prime = div().flex().flex_col().px_4().pt_1().pb_4();
        let mut words = div().flex_1().min_w_0().flex().flex_col().gap_1().child(
            div()
                .text_size(rems(TYPE_META))
                .font_weight(FontWeight::MEDIUM)
                .text_color(palette.text)
                .child("Its role"),
        );
        match prompt {
            Some(prompt) => {
                words = words
                    .child(
                        div()
                            .text_size(rems(TYPE_CODE))
                            .text_color(palette.text)
                            .child(model::line_path_words(&prompt.path)),
                    )
                    .child(
                        div()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .child(format!(
                                "depot {depot} · {}",
                                counted_lines(prompt.text.lines().count())
                            )),
                    );
            }
            None => {
                words = words.child(
                    div()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .child("no prompt"),
                );
            }
        }
        prime = prime.child(
            div()
                .flex()
                .flex_row()
                .gap_2p5()
                .py_2p5()
                .border_b_1()
                .border_color(palette.line)
                .debug_selector(|| "context prime role".to_string())
                .child(
                    div()
                        .w(px(18.))
                        .flex_none()
                        .text_size(rems(TYPE_CODE))
                        .text_color(palette.faint)
                        .child("1"),
                )
                .child(words),
        );
        if let Some((values, whose)) =
            prime_operator_words(model.operator_style(), model.operator_note())
        {
            prime = prime.child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2p5()
                    .py_2p5()
                    .border_b_1()
                    .border_color(palette.line)
                    .debug_selector(|| "context prime operator".to_string())
                    .child(
                        div()
                            .w(px(18.))
                            .flex_none()
                            .text_size(rems(TYPE_CODE))
                            .text_color(palette.faint)
                            .child("2"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_size(rems(TYPE_META))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(palette.text)
                                    .child("How to work with you"),
                            )
                            .child(
                                div()
                                    .text_size(rems(TYPE_CODE))
                                    .text_color(palette.text)
                                    .child(values),
                            )
                            .child(
                                div()
                                    .text_size(rems(TYPE_META))
                                    .text_color(palette.faint)
                                    .child(whose),
                            ),
                    ),
            );
        }
        tab.child(prime)
    }

    /// A stage drawer's foot: Copy path on a file, and on a prompt
    /// Propose an edit with what an edit does. The Group tab carries
    /// none: its Served by… stands in its body. None while the prompt's
    /// editor stands open: its Send and Cancel are the foot then.
    fn context_footer(
        &self,
        open: &ContextOpen,
        model: &Model,
        stage: &LineStage,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        match open.tab {
            ContextTab::Group => None,
            // No crew tab stands on a stage.
            ContextTab::Role | ContextTab::Prime => None,
            ContextTab::Stage => {
                let file = stage.stage_file.as_ref()?;
                let path = file.path.clone();
                Some(
                    div()
                        .flex_none()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .px_4()
                        .py_2p5()
                        .border_t_1()
                        .border_color(palette.line)
                        .child(Self::copy_path(&path, palette)),
                )
            }
            ContextTab::Prompt => {
                let ContextSubject::Stage { depot, .. } = &open.subject else {
                    return None;
                };
                let group = stage.route.as_ref().and_then(|route| route.chain.first())?;
                let prompt = group.prompt.as_ref()?;
                let flow = Self::context_flow(open, model)?;
                let key = format!(
                    "prompt:{}:{}:{}:{}:{}",
                    flow.flow_type, flow.name, flow.source, stage.stage, group.name
                );
                if self
                    .line_edit
                    .as_ref()
                    .is_some_and(|edit| edit.depot == *depot && edit.key == key)
                {
                    return None;
                }
                let mut foot = div()
                    .flex_none()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_4()
                    .py_2p5()
                    .border_t_1()
                    .border_color(palette.line);
                let mut editable = false;
                match (prompt.carriable, prompt.why.is_empty()) {
                    (true, _) => {
                        editable = true;
                        let (depot, owner, prompt) = (
                            depot.clone(),
                            PromptOwner::Group(group.name.clone()),
                            prompt.clone(),
                        );
                        foot = foot.child(
                            small_button("Propose an edit".to_string(), palette)
                                .id("context propose")
                                .debug_selector(|| "context propose".to_string())
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.edit_prompt(
                                        depot.clone(),
                                        owner.clone(),
                                        key.clone(),
                                        prompt.clone(),
                                        window,
                                        cx,
                                    )
                                })),
                        );
                    }
                    (false, false) => {
                        foot = foot.child(
                            div()
                                .text_size(rems(TYPE_META))
                                .text_color(palette.faint)
                                .debug_selector(|| "context propose why".to_string())
                                .child(prompt.why.clone()),
                        );
                    }
                    (false, true) => {}
                }
                foot = foot.child(Self::copy_path(&prompt.path, palette));
                if editable {
                    foot = foot.child(
                        div()
                            .w_full()
                            .text_size(rems(TYPE_META))
                            .text_color(palette.faint)
                            .debug_selector(|| "context foot note".to_string())
                            .child(EDIT_NOTE),
                    );
                }
                Some(foot)
            }
        }
    }

    /// A crew drawer's foot: on the Role tab Propose an edit with what
    /// an edit does, and Copy path. The Prime tab lists, and carries
    /// none. None while the role's editor stands open: its Send and
    /// Cancel are the foot then.
    fn context_crew_footer(
        &self,
        open: &ContextOpen,
        model: &Model,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let ContextSubject::Crew { depot, member } = &open.subject else {
            return None;
        };
        if !matches!(open.tab, ContextTab::Role) {
            return None;
        }
        let prompt = Self::context_role(open, model)?;
        let key = format!("crew:{member}");
        if self
            .line_edit
            .as_ref()
            .is_some_and(|edit| edit.depot == *depot && edit.key == key)
        {
            return None;
        }
        let mut foot = div()
            .flex_none()
            .flex()
            .flex_row()
            .flex_wrap()
            .items_center()
            .gap_2()
            .px_4()
            .py_2p5()
            .border_t_1()
            .border_color(palette.line);
        let mut editable = false;
        match (prompt.carriable, prompt.why.is_empty()) {
            (true, _) => {
                editable = true;
                let (depot, owner, prompt) = (
                    depot.clone(),
                    PromptOwner::Crew(member.clone()),
                    prompt.clone(),
                );
                foot = foot.child(
                    small_button("Propose an edit".to_string(), palette)
                        .id("context propose")
                        .debug_selector(|| "context propose".to_string())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.edit_prompt(
                                depot.clone(),
                                owner.clone(),
                                key.clone(),
                                prompt.clone(),
                                window,
                                cx,
                            )
                        })),
                );
            }
            (false, false) => {
                foot = foot.child(
                    div()
                        .text_size(rems(TYPE_META))
                        .text_color(palette.faint)
                        .debug_selector(|| "context propose why".to_string())
                        .child(prompt.why.clone()),
                );
            }
            (false, true) => {}
        }
        foot = foot.child(Self::copy_path(&prompt.path, palette));
        if editable {
            foot = foot.child(
                div()
                    .w_full()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .debug_selector(|| "context foot note".to_string())
                    .child(EDIT_NOTE),
            );
        }
        Some(foot)
    }

    /// Copy path: the link word that returns the file's path whole.
    fn copy_path(path: &str, palette: Palette) -> Stateful<Div> {
        div()
            .id("context copy path")
            .debug_selector(|| "context copy path".to_string())
            .flex_none()
            .text_size(rems(TYPE_META))
            .text_color(palette.link)
            .cursor_pointer()
            .on_click({
                let path = path.to_string();
                move |_, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(path.clone()))
            })
            .child("Copy path")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_file_folds_past_its_shown_lines_with_the_rest_counted() {
        assert_eq!(folded("", 30), None);
        assert_eq!(folded("one\n", 30), None);
        let thirty = (0..30).map(|n| format!("{n}\n")).collect::<String>();
        assert_eq!(folded(&thirty, 30), None);
        let over = format!("{thirty}last");
        assert_eq!(folded(&over, 30), Some((thirty.as_str(), 1)));
        let two = format!("{thirty}last\nbut one\n");
        assert_eq!(folded(&two, 30), Some((thirty.as_str(), 2)));
    }

    #[test]
    fn the_group_tab_names_its_runner_and_crew_context_names_its_prime() {
        let herdr = LineGroup {
            runner: "herdr".into(),
            kind: "claude".into(),
            model: "opus".into(),
            ..Default::default()
        };
        assert_eq!(
            runner_detail(&herdr),
            Some(("harness", "claude · model opus".to_string()))
        );
        let bare = LineGroup {
            runner: "herdr".into(),
            ..Default::default()
        };
        assert_eq!(
            runner_detail(&bare),
            Some(("harness", "the harness's own".to_string()))
        );
        let exec = LineGroup {
            runner: "exec".into(),
            cmd: "scripts/merge-local --fast".into(),
            ..Default::default()
        };
        assert_eq!(
            runner_detail(&exec),
            Some(("command", "scripts/merge-local --fast".to_string()))
        );
        let people = LineGroup {
            runner: "manual".into(),
            members: vec!["jo".into(), "sam".into()],
            ..Default::default()
        };
        assert_eq!(
            runner_detail(&people),
            Some(("members", "jo, sam".to_string()))
        );
        let anyone = LineGroup {
            runner: "manual".into(),
            ..Default::default()
        };
        assert_eq!(
            runner_detail(&anyone),
            Some(("members", "anyone may move a bead on".to_string()))
        );
        let peer = LineGroup {
            runner: "peer".into(),
            yard: "elsewhere".into(),
            ..Default::default()
        };
        assert_eq!(
            runner_detail(&peer),
            Some(("yard", "elsewhere".to_string()))
        );
        assert_eq!(runner_detail(&LineGroup::default()), None);
        assert_eq!(
            prompt_meta("home", 38, "builders"),
            "home · 38 lines · the prompt builders starts each session with"
        );
        assert_eq!(
            prompt_meta("", 1, "builders"),
            "1 line · the prompt builders starts each session with"
        );
        assert_eq!(
            serves_words("footplate", "default", "new"),
            "serves stage new of footplate's default"
        );
        assert_eq!(
            unserved_words("footplate", "default", "review"),
            "No group serves review in footplate's default flow: a bead here waits for a person to move it."
        );
        assert_eq!(
            role_meta("footplate", 178),
            "depot footplate · 178 lines · read when its session starts"
        );
        assert_eq!(
            role_meta("notes", 1),
            "depot notes · 1 line · read when its session starts"
        );
        assert_eq!(prime_command("yardmaster"), "yardr prime --crew yardmaster");
        assert_eq!(
            prime_operator_words("show", "I read diffs, not prose"),
            Some((
                "show · I read diffs, not prose".to_string(),
                "the yard's settings · every session reads both".to_string()
            ))
        );
        assert_eq!(prime_operator_words("", ""), None);
        assert_eq!(
            prime_operator_words("show", ""),
            Some(("show".to_string(), "the yard's settings".to_string()))
        );
        assert_eq!(
            prime_operator_words("", "I read diffs, not prose"),
            Some((
                "I read diffs, not prose".to_string(),
                "the yard's settings".to_string()
            ))
        );
    }
}
