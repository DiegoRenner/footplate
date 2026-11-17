# The look

What the window is drawn with: its palette, its type scale, its spacing
and the row and card patterns every screen builds against. The values
here are the built ones, read from `src/theme.rs`,
`src/window/metrics.rs` and the drawing code they name; the mockups in
yardr-design are the direction, never the source of a number. Part of
footplate's manual; the [README](../README.md) is the front door.

## Palette

Fifteen tokens, the design system's whole (`tokens/colors.css` in
yardr-design), in a dark and a light look. The window's look is
`Look::Auto` (the desktop's, as it changes), `Look::Light` or
`Look::Dark`; the palette for it is `Palette::of` in `src/theme.rs`.
No screen names a colour of its own.

| Token | Dark | Light | Stands for |
| --- | --- | --- | --- |
| ground | `#161718` | `#F7F7F6` | the list's and the conversation's ground |
| side | `#101112` | `#EDEDEB` | the rail, a field, a run or setup block |
| raised | `#1C1D1F` | `#FFFFFF` | an ask card's frame, a secondary button |
| line | `#27282B` | `#E2E2DF` | hairlines: bands, separators, card and field borders |
| strong | `#333538` | `#CFCFCB` | a secondary button's edge |
| open | `#2A2B2E` | `#E2E2DF` | the open row, filled |
| text | `#E3E5E8` | `#22242A` | the primary text |
| faint | `#ABADB0` | `#5B5E64` | quiet words, counts, metadata |
| accent | `#D3602F` | `#D3602F` | what waits on the person reading: a state, never a button |
| on accent | `#161718` | `#161718` | text on the accent |
| button | `#B54E22` | `#B54E22` | the one filled button's fill, white on it |
| text accent | `#E2794A` | `#8F3C19` | yard-rust text at contrast: waiting words, a lever's plate |
| ok | `#4CC38A` | `#1F9A60` | the running dot alone |
| link | `#7AA2F7` | `#1A56C8` | links, in messages and ask and bead ids |
| peer | `#B693F5` | `#6440B8` | a linked name's peer suffix (`@airy`) |

A button and a state never share a fill: the filled button is the
deeper rust (`button`, hover `#C0552A`, pressed `#8F3C19`) and the
accent at the brighter step fills only states — the chosen segment,
the checked box, the rail's waiting badge. Green is only the running
dot; red (`#E5675A` dark, `#B3382C` light) is only errors and the
states that say a thing failed.

## Type scale

The design tokens' roles (`tokens/typography.css`), at the window's
one rem, so the text setting scales every role together. At the usual
16-pixel rem: question 22, title 15, section 13, label 13, body 13,
meta 12, code 12. The constants are `TYPE_QUESTION`, `TYPE_TITLE`,
`TYPE_SECTION`, `TYPE_LABEL`, `TYPE_BODY`, `TYPE_META` and
`TYPE_CODE` in `src/theme.rs`. The question is direction 2's `.q`
(22/600), the one text the card owns the pane for.

| Role | Size | Weight | Stands for |
| --- | --- | --- | --- |
| question | 22 | semibold | the current ask's question |
| title | 15 | semibold | another ask's question, a pane's title |
| section | 13 | semibold | a section's or a chat group's head |
| label | 13 | medium | a lever's label, a row's head |
| body | 13 | regular | a card's context lines |
| meta | 12 | regular | counts, metadata, a lever's plate and consequence |
| code | 12 | mono | commands, and the list's time column |

The mono face is the machine's first of Menlo, Consolas, DejaVu Sans
Mono (`select::mono`); no other face is set anywhere. The window's root
text stands at the person's text setting over this scale; the rail's
labels are fixed at 10 pixels, as the rail carries no reading text.

## Spacing and measures

Fixed in pixels where the frame stands fixed, in rems where the words
in it set the size (`src/window/metrics.rs`).

- The window opens at 960×640; its floor is 624×360 (560 and the
  rail), and the rail is 64 wide with 52-wide tiles.
- The list is 352 wide at the usual text size, capped so the pane
  beside it keeps 290: at 960 the pane is 544, and at the floor the
  list narrows to 270 to keep the pane. A lever label and a list title
  in the shot's content stand whole at 352.
- A card's frame: raised ground, a 1px `line` border, an 8px radius
  (`RADIUS_CARD`), 16px of padding and 10px between its rows.
- A lever: at least 32 high (`LEVER_H`, the control's height), a 6px
  radius (`RADIUS_CONTROL`), 8px of padding at its sides. An action
  lever (`LEVER_ACTION_H`, direction 2's 58) stands taller, its key
  plate above its label; the four stand in one row under a run or
  setup card's command.
- Controls stand at least 32 high; a row's padding is 8 by 6; a
  section head stands at least 32 high.
- The conversation's reading column stands full width up to 880,
  centred in the pane.

## Rows

One row for every list: a lamp dot of 8px where the row carries a
state (green working, rust waiting on you, red failed, grey at rest),
a head that wraps to two lines in the label role, one metadata line
under it for each further fact in the meta role and quiet, and at the
right a short id or a count in quiet words. Decision and parked rows
carry a time column at their right: how long the ask or bead waits
(`3d`, `2h`, `5m`, `now`), in the mono face at the code size. The
metadata line leaves that age out, so it reads once in the row.
The open row is filled with `open`; a row under the pointer is told by
its hairline alone, so exactly one row ever reads open. A section head
is its title in the section role with its count in quiet words at the
right (`2 to decide`, `none waiting`); counts are words, never a
numeral badge, the yard's notices alone excepted.

## Cards and the one-fill rule

An ask card is the raised, bordered frame above. Inside it: the yard's
weight lines in text accent, the question (the current ask's in the
question role, the rest in the title role) with how long it waits
beside it, who asked, the context in the body role, the bead whole
behind one disclosure, then the ask's own body, its field, its files
and its refusal, the action row and the keys line. Under all the bead's
cards stands its thread: the bead's notes in order, each headed by who
wrote it, its time and the bead, its words beside its image where it
has one (156 wide, 60 high), its other files under its words as one
quiet line of name and size each, their Open and run on the bead whole
behind the disclosure. A short card and its thread stand together at
960×640.

A pick ask's options are levers: a key plate, the label and its
consequence. The chosen lever and the recommended one are outlined in
the accent; the recommended one's plate names it (`2 · recommended`)
in text accent. No option lever is ever filled.

One filled control per ask: the card's only fill is its Answer button
(a pick ask) or its Approve lever (a run or setup ask), in `button`
with white on it. A run or setup card's four moves are levers in one
row — Approve filled, Ask about this (`a`), Later (`x`) and Decline
(`d`) outlined, each with its key plate above its label. Later,
Decline and Ask about this on a pick card are bordered secondary
buttons. A run ask's command stands in a block on the side ground,
its Copy returning the plain command; a setup ask's taken account is
outlined, as a lever is, and its Approve is the card's one fill
likewise.
