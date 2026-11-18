//! A stage's context drawer: what shapes one stage of a depot's
//! flow — its stage file, the group that serves it and the group's
//! prompt — standing at the pane's right while its station stands
//! pressed. A press on a panel station or a flow strip's knot opens
//! it; it stands beside the pane where the pane keeps its width past
//! it, and over the pane's right where it does not. Its Line read is
//! the Line window's, over another selection; its prompt editor,
//! hand-off and Served-by picker are the Line window's rows, drawn
//! here. The crew drawer reuses this shell.

use gpui::{
    ClipboardItem, Context, Div, FontWeight, SharedString, Stateful, Window, div, prelude::*, px,
    rems, rgba,
};

use super::Footplate;
use super::expanded_text_key;
use super::metrics::{CONTEXT_WIDTH, context_beside, grown};
use super::ui::small_button;
use crate::api::{LineFlow, LineGroup, LineStage};
use crate::model::{self, Model, PromptOwner};
use crate::select;
use crate::theme::{Palette, TYPE_BODY, TYPE_CODE, TYPE_LABEL, TYPE_META};

/// Which of the drawer's three tabs stands open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ContextTab {
    Stage,
    Group,
    Prompt,
}

/// The context drawer standing at the window's right: the stage it
/// stands on, its tab, whether the Group tab's Served-by picker stands
/// open, and which long files stand shown whole past their fold.
pub(crate) struct ContextOpen {
    pub(crate) depot: String,
    pub(crate) flow: String,
    pub(crate) flow_type: String,
    pub(crate) stage: String,
    pub(crate) tab: ContextTab,
    pub(crate) served_open: bool,
    pub(crate) stage_whole: bool,
    pub(crate) prompt_whole: bool,
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
        let same = self.context.as_ref().is_some_and(|open| {
            open.depot == depot
                && open.flow == flow
                && open.flow_type == flow_type
                && open.stage == stage
        });
        if !same {
            self.context_scroll.scroll_to_item(0);
            self.context = Some(ContextOpen {
                depot,
                flow,
                flow_type,
                stage,
                tab: ContextTab::Stage,
                served_open: false,
                stage_whole: false,
                prompt_whole: false,
            });
        }
        cx.notify();
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
                ContextTab::Group => {}
            }
        }
        cx.notify();
    }

    /// The drawer's flow as the depot's line reads it, by the name and
    /// type the drawer stands on.
    fn context_flow<'a>(open: &ContextOpen, model: &'a Model) -> Option<&'a LineFlow> {
        model.line_read_for(&open.depot).and_then(|line| {
            line.flows
                .iter()
                .find(|flow| flow.name == open.flow && flow.flow_type == open.flow_type)
        })
    }

    /// The drawer's stage as the depot's line reads it.
    fn context_stage<'a>(open: &ContextOpen, model: &'a Model) -> Option<&'a LineStage> {
        Self::context_flow(open, model)
            .and_then(|flow| flow.stages.iter().find(|stage| stage.stage == open.stage))
    }

    /// The context drawer at the window's right: one stage's file, its
    /// serving group and the group's prompt under three tabs, over the
    /// Lines views. None where no station stands pressed. It stands
    /// beside the pane where the pane keeps its width past it, and over
    /// the pane's right where it does not; the list stays in reach
    /// either way.
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
        let at = model.context()?;
        if at.depot != open.depot
            || at.flow != open.flow
            || at.flow_type != open.flow_type
            || at.stage != open.stage
        {
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
            .child(self.context_tabs(open, palette, cx));
        let stage = Self::context_stage(open, model);
        drawer = drawer.child(
            div()
                .id("context body")
                .debug_selector(|| "context body".to_string())
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .track_scroll(&self.context_scroll)
                .child(match stage {
                    Some(stage) => self.context_body(open, model, stage, palette, cx),
                    None => self.context_empty(open, model, palette),
                }),
        );
        if let Some(stage) = stage {
            drawer = drawer.children(self.context_footer(open, model, stage, palette, cx));
        }
        Some(drawer)
    }

    /// The drawer's head: what it stands on — the depot, the flow and
    /// the stage — with its ×.
    fn context_head(
        &self,
        open: &ContextOpen,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let name = match open.flow_type.is_empty() {
            true => format!("{} · {} · {}", open.depot, open.flow, open.stage),
            false => format!(
                "{} · {} · {} · {}",
                open.depot, open.flow_type, open.flow, open.stage
            ),
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
                        let mut kind = div().child("stage");
                        if let Some(mono) = select::mono(cx) {
                            kind = kind.font_family(mono);
                        }
                        kind
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

    /// The drawer's three tabs: the stage file, the serving group, the
    /// group's prompt. The open one reads in the text colour over the
    /// accent's edge.
    fn context_tabs(
        &self,
        open: &ContextOpen,
        palette: Palette,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut tabs = div()
            .flex_none()
            .flex()
            .flex_row()
            .gap_1()
            .px_3()
            .border_b_1()
            .border_color(palette.line);
        for (tab, label) in [
            (ContextTab::Stage, "Stage"),
            (ContextTab::Group, "Group"),
            (ContextTab::Prompt, "Prompt"),
        ] {
            let on = open.tab == tab;
            let selector = format!("context tab {}", label.to_lowercase());
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
    /// not the operator's.
    fn context_empty(&self, open: &ContextOpen, model: &Model, palette: Palette) -> Div {
        let words = if model.line_read_for(&open.depot).is_some() {
            match Self::context_flow(open, model) {
                Some(_) => "The stage is gone.",
                None => "The flow is gone.",
            }
        } else if model.line_unserved() {
            model::LINE_TOO_OLD
        } else if !model.is_operator() {
            "The context drawer is the operator's."
        } else {
            model.line_notice().unwrap_or("Reading the line…")
        };
        let mut empty = div().flex().flex_col().gap_2().p_4().child(
            div()
                .debug_selector(|| "context empty".to_string())
                .text_color(palette.faint)
                .child(words.to_string()),
        );
        if model.line_read_for(&open.depot).is_none()
            && !model.line_unserved()
            && model.is_operator()
            && model.line_notice().is_some()
        {
            empty = empty.child(
                div()
                    .text_size(rems(TYPE_META))
                    .text_color(palette.faint)
                    .debug_selector(|| "context retry".to_string())
                    .child("Press the station again to try again."),
            );
        }
        empty
    }

    /// The drawer's body for the open tab: the stage file, the serving
    /// group, or the group's prompt.
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
        let Some(file) = stage.stage_file.as_ref() else {
            return div().p_4().child(
                div()
                    .debug_selector(|| "context stage none".to_string())
                    .text_color(palette.faint)
                    .child("No stage file."),
            );
        };
        let flow = Self::context_flow(open, model);
        let mut meta = format!("depot {}", open.depot);
        if let Some(source) = flow
            .map(|flow| flow.source.as_str())
            .filter(|source| !source.is_empty())
        {
            meta.push_str(&format!(" · {source}"));
        }
        let key = select::key(("context stage", &open.depot, &open.flow, &open.stage));
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
        let flow = Self::context_flow(open, model);
        let heading = flow
            .map(model::line_flow_heading)
            .unwrap_or_else(|| open.flow.clone());
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
                        .child(serves_words(&open.depot, &heading, &stage.stage))
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
                .line_read_for(&open.depot)
                .map(model::line_groups)
                .unwrap_or_default();
            tab = tab.child(div().px_4().py_3().child(self.line_served(
                &open.depot,
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
            .filter(|f| f.depot == open.depot && (f.stage.is_empty() || f.stage == stage.stage))
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
        let flow = Self::context_flow(open, model);
        let heading = flow
            .map(model::line_flow_heading)
            .unwrap_or_else(|| open.flow.clone());
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
                .child(unserved_words(&open.depot, &heading, &stage.stage)),
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
                            "Picks a group for the stage; the route change goes to the yardmaster, and comes back as a card in Decide.",
                        ),
                ),
        );
        if open.served_open {
            let groups = model
                .line_read_for(&open.depot)
                .map(model::line_groups)
                .unwrap_or_default();
            tab = tab.child(div().px_4().pb_3().child(self.line_served(
                &open.depot,
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
            .filter(|f| f.depot == open.depot && (f.stage.is_empty() || f.stage == stage.stage))
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
        let seats = model
            .flow(&open.depot, &open.flow, &open.flow_type)
            .and_then(|flow| flow.stages.iter().find(|stage| stage.stage == open.stage))
            .and_then(|stage| model::context_seats_words(stage.limit, stage.taken));
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
        let key = select::key(("context prompt", &open.depot, &open.flow, &open.stage));
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
        let flow = Self::context_flow(open, model);
        let edit_key = flow
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
            .filter(|edit| edit.depot == open.depot && edit.key == edit_key)
        {
            tab = tab.child(div().px_4().pb_3().child(self.line_editor(
                edit,
                "context prompt",
                palette,
                cx,
            )));
        }
        if let Some(sent) = self.line_sent.as_ref().filter(|sent| {
            sent.depot == open.depot && sent.owner == PromptOwner::Group(group.name.clone())
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

    /// The drawer's foot: Copy path on a file, and on a prompt Propose
    /// an edit with what an edit does. The Group tab carries none: its
    /// Served by… stands in its body. None while the prompt's editor
    /// stands open: its Send and Cancel are the foot then.
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
                    .is_some_and(|edit| edit.depot == open.depot && edit.key == key)
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
                            open.depot.clone(),
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
                            .child(
                                "An edit goes to the yardmaster as one mail with the whole text, the file and the digest as read; the change comes back to you as a card in Decide.",
                            ),
                    );
                }
                Some(foot)
            }
        }
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
    fn the_group_tab_names_the_runner_s_harness_and_the_prompt_s_file() {
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
    }
}
