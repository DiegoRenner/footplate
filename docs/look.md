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
| raised | `#1C1D1F` | `#FFFFFF` | a decided card's frame, a secondary button |
| line | `#27282B` | `#E2E2DF` | hairlines: bands, the rules between asks, field borders |
| strong | `#333538` | `#CFCFCB` | a secondary button's edge |
| open | `#2A2B2E` | `#E2E2DF` | the open row, filled |
| text | `#E3E5E8` | `#22242A` | the primary text |
| faint | `#ABADB0` | `#5B5E64` | quiet words, counts, metadata |
| accent | `#D3602F` | `#D3602F` | what waits on the person reading: a state, never a button |
| on accent | `#161718` | `#161718` | text on the accent |
| button | `#B54E22` | `#B54E22` | the one filled button's fill, white on it |
| text accent | `#E2794A` | `#8F3C19` | yard-rust text at contrast: waiting words, the recommended lever's plate and tag |
| ok | `#4CC38A` | `#1F9A60` | the running dot, a working lamp, an approval's mark |
| link | `#7AA2F7` | `#1A56C8` | links, in messages and ask and bead ids |
| peer | `#B693F5` | `#6440B8` | a linked name's peer suffix (`@airy`) |

A button and a state never share a fill: the filled button is the
deeper rust (`button`, hover `#C0552A`, pressed `#8F3C19`) and the
accent at the brighter step fills only states — the chosen segment,
the checked box, the rail's waiting badge. Green is only the running
dot, a crew member's working lamp and an approval's mark in the
history; red (`#E5675A` dark, `#B3382C` light) is only errors and the
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
| title | 15 | semibold | another ask's question |
| section | 13 | semibold | a section's or a chat group's head |
| label | 13 | medium | a lever's label, a row's head (semibold on an open ask's row), the decision pane's bead title |
| body | 13 | regular | a card's context lines |
| meta | 12 | regular | counts, metadata, a lever's plate and consequence, the line over a question |
| code | 12 | mono | commands, and a row's short id |

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
- An open ask's card stands flat on the pane, 10px between its rows;
  each card after the first is ruled off from the one above by a 1px
  `line`, with 16px either side of it. A decided card keeps the raised,
  bordered frame: raised ground, a 1px `line` border, an 8px radius
  (`RADIUS_CARD`) and 16px of padding.
- A list row's lamp is 8 (`ROW_LAMP`) with 8 after it
  (`ROW_LAMP_GAP`); the lines under a row's head are indented by the two,
  so they stand under its first letter. A decided row's mark is 12
  (`ROW_MARK`) in the same room.
- A row's lines end in an ellipsis past a char budget (`ui::shorten`):
  34 for a head (`ROW_TITLE_CHARS`) and 34 for an ask's question
  (`ROW_ASK_CHARS`), which fit the list at 352. gpui's `truncate()` clips
  mid-glyph in a row's flex items, so it is only the backstop where the
  list is narrower.
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

One shape for every list's row: a lamp dot of 8px where the row carries
a state (green working, rust waiting on you, red failed, grey at rest),
or the room for one; the head in the label role, one line, ended in an
ellipsis; under it, past the lamp's room, a line for each further fact
in the meta role and quiet; and at the end of the last line a short id
in the mono face at the code size, or a count in quiet words.

- An open ask's row is its bead: the rust lamp and the bead's title,
  semibold, then a line for each open ask (what it wants and its number
  in the text colour, `run 1`, `pick 2`, `words 4`, then the question's
  first line), then who asks, each once, and how long the oldest ask
  waits (`yardmaster · 12m`), with the bead's short id.
- A parked bead's row has no lamp, its title under the open rows'
  titles, then its stage, group and age (`pick · design-pickers · 20m`).
- A decided row has its outcome's mark in the lamp's room: Lucide's
  glyph at one stroke (the design system allows no unicode character as
  an icon), `check` approved in green, `message-square` words or a secret
  given, `undo-2` taken back, `clock` put aside, `x` declined, the rest
  quiet. Then the question's first line, and the outcome in the text
  colour with whose and when quiet after it. A fold of look-alikes
  (same day, asker and outcome, questions differing only in numbers)
  stands in the same shape, its count (`×4 ▸`) where the id stands.

The open row is filled with `open`; a row under the pointer is told by
its hairline alone, so exactly one row ever reads open. A section head
is its title in the section role with its count in quiet words at the
right (`2 to decide`, `none waiting`); counts are words, never a
numeral badge, the yard's notices alone excepted. A topic's head
stands over the open rows only where two asks or more share it beside
another topic.

## Cards and the one-fill rule

The decision pane's head is the yard's dot with the yard, the bead and
the topic in the meta role and quiet, over the bead's title in the label
role. Under it the bead's asks stand as flat cards, ruled apart. Each
card: the yard's weight lines in text accent; a line over the question
in the meta role, `Waiting on you · run 1` in text accent, then how long
and who asks quiet (`asked 12m · yardmaster`), and at its right the ask
line (`footplate-q2rd · ask 1`, to select and copy) and the bead whole's
disclosure (`▸ details (e)`); the question (the current ask's in the
question role, the rest in the title role); the context in the body
role; then the ask's own body, its field, its files and its refusal, the
action row and the keys line. A run or setup ask's question and context
lines each carry the asker's label (`yardmaster says:`), quiet in the
faint colour at the regular weight: every line the asker wrote is marked
as theirs, so none reads as the yard's.

Under all the bead's cards stands its thread: the bead's notes newest
first (as in the card's details and the bead whole in Lines; the crew's
conversation alone reads oldest first), each headed by who wrote it, its
time and the bead, its words beside its image where it has one (156
wide, 60 high), its other files under its words as one quiet line of
name and size each, their Open and run on the bead whole behind the
disclosure. A short card and its thread stand together at 960×640.

A pick ask's options are levers: a key plate holding its number alone,
the label and its consequence, so every label stands at the same place.
The chosen lever alone is outlined in the accent; the recommended one is
chosen until another is picked, so two outlines never stand at once.
The recommended lever's plate is in text accent with an accent edge, the
word `recommended` keeps a column at its row's right in text accent, and
its why stands under its label inside the lever. No option lever is ever
filled.

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
