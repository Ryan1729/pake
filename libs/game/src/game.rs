#![allow(unused_imports)]

use gfx::{CHAR_SPACING_H, CHAR_SPACING_W, SPACING_H, SPACING_W, chart_block, Commands, Highlighting::{Highlighted, Plain}};
use look_up::{holdem::{ALL_SORTED_HANDS, hand_win_probability}, probability::{FIFTY_PERCENT, SEVENTY_FIVE_PERCENT, EIGHTY_SEVEN_POINT_FIVE_PERCENT, Probability}};
use models::{Card, ALL_CARDS, Money, NonZeroMoney, holdem::{MAX_PLAYERS, MAX_POTS, Action, ActionKind, ActionSpec, AllowedKindMode, CommunityCards, Deck, Facing, FullBoard, Hand, HandIndex, HandLen, Hands, PerPlayer, Pot, PotAction, RoundOutcome, gen_action, gen_deck, gen_hand_index}};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled};

use xs::{Xs, Seed};

use std::io::Write;

const TEXT: PaletteIndex = 6;

#[derive(Clone, Default)]
pub struct HoldemMenuSelection {
    pub action_kind: ActionKind,
    pub bet: Money,
}

#[derive(Clone, Copy, Default)]
pub enum Modal {
    #[default]
    Nothing,
    Chart,
}

#[derive(Clone)]
pub struct HoldemStateBundle {
    pub deck: Deck,
    pub hands: Hands,
    pub dealer: HandIndex,
    pub current: HandIndex,
    pub pot: Pot,
    pub selection: HoldemMenuSelection,
    pub modal: Modal,
}

#[derive(Clone)]
pub enum HoldemTableState {
    Undealt { player_count: HandLen, starting_money: Money },
    PreFlop {
        bundle: HoldemStateBundle,
    },
    PostFlop {
        bundle: HoldemStateBundle,
        community_cards: CommunityCards,
    },
    Showdown {
        bundle: HoldemStateBundle,
        full_board: FullBoard,
    },
}

impl Default for HoldemTableState {
    fn default() -> Self {
        Self::Undealt {
            player_count: <_>::default(),
            starting_money: 500,
        }
    }
}

type Personality = Option<CpuPersonality>;

#[derive(Clone, Debug)]
struct CpuPersonality {
    // TODO
}

#[derive(Clone, Default, PartialEq)]
enum SkipState {
    #[default]
    Watch,
    Skip,
}

#[derive(Clone, Default)]
pub struct HoldemTable {
    state: HoldemTableState,
    moneys: [Money; MAX_PLAYERS as usize],
    personalities: [Personality; MAX_PLAYERS as usize],
    skip: SkipState,
}

#[derive(Clone, Default)]
enum ModeName {
    #[default]
    Holdem,
    //AceyDeucey
}

#[derive(Clone)]
enum Mode {
    Title(ModeName),
    Holdem(HoldemTable),
}

impl Default for Mode {
    fn default() -> Self {
        Self::Title(<_>::default())
    }
}

#[derive(Clone, Default)]
pub struct State {
    pub rng: Xs,
    pub ctx: ui::Context,
    pub mode: Mode
}

impl State {
    pub fn new(seed: Seed) -> State {
        // 22 Players, User dealt a pair of 8s, beaten by a 8-high straight.
        //let seed = [177, 142, 173, 15, 242, 60, 217, 65, 49, 80, 175, 162, 108, 73, 4, 62];
        // 22 Players, User dealt a pair of Aces, wins with Aces over Queens.
        // let seed = [148, 99, 192, 160, 91, 61, 217, 65, 108, 157, 212, 200, 23, 73, 4, 62];
        let mut rng = xs::from_seed(seed);

        State {
            rng,
            .. <_>::default()
        }
    }
}

mod ui {
    use super::*;

    /// A group of things that are used together to render UI. Naming suggestions
    /// welcome!
    pub(crate) struct Group<'commands, 'ctx, 'speaker> {
        pub commands: &'commands mut Commands,
        pub ctx: &'ctx mut Context,
        pub input: Input,
        pub speaker: &'speaker mut Speaker,
    }

    pub type HoldemMenuId = u8;

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub enum Id {
        #[default]
        Zero,
        TitleBeginButton,
        BackToTitleScreen,
        Submit,
        PlayerCountSelect,
        StartingMoneySelect,
        HoldemHand(HandIndex),
        HoldemMenu(HoldemMenuId),
        HoldemChartButton,
        SkipRemainderOfGameSelect,
        ShowdownSubmit,
    }

    #[derive(Copy, Clone, Default, Debug)]
    pub struct Context {
        pub active: Id,
        pub hot: Id,
        pub next_hot: Id,
    }

    impl Context {
        pub fn set_not_active(&mut self) {
            self.active = Id::Zero;
        }
        pub fn set_active(&mut self, id: Id) {
            self.active = id;
        }
        pub fn set_next_hot(&mut self, id: Id) {
            self.next_hot = id;
        }
        pub fn set_not_hot(&mut self) {
            self.hot = Id::Zero;
        }
        pub fn frame_init(&mut self) {
            if self.active == Id::Zero {
                self.hot = self.next_hot;
            }
            self.next_hot = Id::Zero;
        }
    }

    pub(crate) struct ButtonSpec<'text> {
        pub id: Id,
        pub rect: unscaled::Rect,
        pub text: &'text [u8],
    }

    pub(crate) fn button_press<'commands, 'ctx, 'speaker>(
        group: &mut Group<'commands, 'ctx, 'speaker>,
        id: Id,
    ) -> bool {
        let mut output = false;

        if group.ctx.active == id {
            if group.input.released_this_frame(Button::A) {
                output = group.ctx.hot == id;

                group.ctx.set_not_active();
            }
            group.ctx.set_next_hot(id);
        } else if group.ctx.hot == id {
            if group.input.pressed_this_frame(Button::A) {
                group.ctx.set_active(id);
                group.speaker.request_sfx(SFX::ButtonPress);
            }
            group.ctx.set_next_hot(id);
        }

        output
    }

    pub(crate) fn do_button<'commands, 'ctx, 'speaker, 'text>(
        group: &mut Group<'commands, 'ctx, 'speaker>,
        spec: ButtonSpec<'text>,
    ) -> bool {
        use gfx::NineSlice as ns;
        let id = spec.id;

        let result = button_press(group, id);

        if group.ctx.active == id && group.input.gamepad.contains(Button::A) {
            group.commands.draw_nine_slice(ns::ButtonPressed, spec.rect);
        } else if group.ctx.hot == id {
            group.commands.draw_nine_slice(ns::ButtonHot, spec.rect);
        } else {
            group.commands.draw_nine_slice(ns::Button, spec.rect);
        }

        let xy = gfx::center_line_in_rect(
            spec.text.len() as _,
            spec.rect,
        );

        //Long labels aren't great UX anyway, I think, so don't bother reflowing.
        group.commands.print_chars(
            spec.text,
            xy.x,
            xy.y,
            TEXT
        );

        result
    }

    /// As a user of this `fn` you are expected to have drawn the separate states
    /// that are selected between before calling this, in the given rect.
    pub(crate) fn draw_quick_select<'commands, 'ctx, 'speaker, 'text>(
        group: &mut Group<'commands, 'ctx, 'speaker>,
        rect: unscaled::Rect,
        id: Id,
    ) {
        use gfx::chevron;
        let mut highlighting = gfx::Highlighting::Plain;
        if group.ctx.active == id
        || group.ctx.hot == id {
            group.ctx.set_next_hot(id);
            highlighting = gfx::Highlighting::Highlighted;
        }

        let x = (rect.x + (rect.w / 2)) - (chevron::WIDTH / 2);

        group.commands.draw_up_chevron(
            highlighting,
            x,
            rect.y,
        );

        group.commands.draw_down_chevron(
            highlighting,
            x,
            rect.y + rect.h - chevron::HEIGHT,
        );
    }
}

use ui::{ButtonSpec, Id::*, do_button};

#[derive(Clone, Copy, Default)]
enum ModeCmd {
    #[default]
    NoOp,
    BackToTitleScreen
}

pub fn update_and_render(
    commands: &mut Commands,
    state: &mut State,
    input: Input,
    speaker: &mut Speaker,
) {
    macro_rules! new_group {
        () => {
            &mut ui::Group {
                commands,
                ctx: &mut state.ctx,
                input,
                speaker,
            }
        }
    }

    state.ctx.frame_init();

    let mut cmd = ModeCmd::default();

    match &mut state.mode {
        Mode::Title(mode_name) => {
            let group = new_group!();

            const TITLE_X: unscaled::X = unscaled::x_const_add_w(
                unscaled::X(0),
                unscaled::w_const_div(
                    unscaled::w_const_sub(command::WIDTH_W, gfx::title::WIDTH),
                    2
                )
            );
            const TITLE_Y: unscaled::Y = unscaled::Y(15);
            group.commands.draw_title(
                TITLE_X,
                TITLE_Y,
            );

            let mut y = TITLE_Y + gfx::title::HEIGHT;

            {
                y += SPACING_H;

                const CONTROLS_W: unscaled::W = unscaled::W(100);
                const CONTROLS_H: unscaled::H = unscaled::H(100);
                const CONTROLS_X: unscaled::X = unscaled::x_const_add_w(
                    unscaled::X(0),
                    unscaled::w_const_div(
                        unscaled::w_const_sub(command::WIDTH_W, CONTROLS_W),
                        2
                    )
                );

                let controls_rect = unscaled::Rect {
                    x: CONTROLS_X,
                    y,
                    w: CONTROLS_W,
                    h: CONTROLS_H,
                };

                let controls_text = b"this game uses the z, x, and arrow keys";

                let xy = gfx::center_line_in_rect(
                    controls_text.len() as _,
                    controls_rect,
                );
                group.commands.print_chars(
                    controls_text,
                    xy.x,
                    xy.y,
                    TEXT
                );
            }

            // TODO in debug and/or a feature only, take a CLI arg or similar to
            // select a mode and skip the title screen, without user input

            

            let w = unscaled::W(50);
            let h = unscaled::H(50);

            if do_button(
                group,
                ButtonSpec {
                    id: TitleBeginButton,
                    rect: unscaled::Rect {
                        x: unscaled::X(0) + ((command::WIDTH_W) - w) / 2,
                        y: unscaled::Y(0) + command::HEIGHT_H - (h * 2),
                        w,
                        h,
                    },
                    text: b"begin",
                }
            ) {
                state.mode = match mode_name {
                    ModeName::Holdem => {
                        Mode::Holdem(<_>::default())
                    },
                    //ModeName::AceyDeucey => {
                        //Mode::AceyDeucey(<_>::default())
                    //},
                };
            }

            if let Zero = group.ctx.hot {
                group.ctx.set_next_hot(TitleBeginButton);
            }
        }
        Mode::Holdem(table) => {
            cmd = holdem_update_and_render(
                commands,
                HoldemState {
                    rng: &mut state.rng,
                    ctx: &mut state.ctx,
                    table,
                },
                input,
                speaker,
            );
        }
    }

    match cmd {
        ModeCmd::NoOp => {},
        ModeCmd::BackToTitleScreen => {
            state.mode = Mode::Title(ModeName::Holdem);
        }
    }
}

struct HoldemState<'state> {
    pub rng: &'state mut Xs,
    pub ctx: &'state mut ui::Context,
    pub table: &'state mut HoldemTable
}

fn holdem_update_and_render(
    commands: &mut Commands,
    state: HoldemState<'_>,
    input: Input,
    speaker: &mut Speaker,
) -> ModeCmd {
    use HoldemTableState::*;
    use ui::Id::*;

    let rng = state.rng;

    let mut cmd = ModeCmd::default();

    macro_rules! new_group {
        () => {
            &mut ui::Group {
                commands,
                ctx: state.ctx,
                input,
                speaker,
            }
        }
    }

    const COMMUNITY_BASE_X: unscaled::X = unscaled::X(150);
    const COMMUNITY_BASE_Y: unscaled::Y = unscaled::Y(150);

    macro_rules! stack_eval_text {
        ($text:ident = $eval: expr) => {
            let mut eval_text = [0 as u8; 64];
            let _cant_actually_fail = write!(
                &mut eval_text[..],
                "{}",
                $eval
            );

            // Lowercase the first letter, since upper case is weird in the font
            // right now
            eval_text[0].make_ascii_lowercase();

            let $text = eval_text;
        }
    }

    macro_rules! stack_money_text {
        ($text:ident = $money: expr) => {
            let mut money_text = [0 as u8; 20];
            money_text[0] = b'$';
            let _cant_actually_fail = write!(
                &mut money_text[1..],
                "{}",
                $money
            );

            let $text = money_text;
        }
    }

    macro_rules! draw_money_in_rect {
        ($group:ident, $money: expr, $rect: expr) => {
            stack_money_text!(text = $money);

            {
                let xy = gfx::center_line_in_rect(
                    pre_nul_len(&text),
                    $rect,
                );
                $group.commands.print_chars(
                    &text,
                    xy.x,
                    xy.y,
                    TEXT
                );
            }
        }
    }

    let min_money_unit: NonZeroMoney = NonZeroMoney::MIN.saturating_add(5 - 1);
    let small_blind_amount: NonZeroMoney = min_money_unit;
    let large_blind_amount: NonZeroMoney = small_blind_amount.saturating_add(min_money_unit.get());

    macro_rules! do_holdem_hands {
        ($group: ident $(,)? $bundle: ident , $community_opt: expr) => ({
            let group = $group;
            let hands = &$bundle.hands;
            let dealer = $bundle.dealer;
            let current = $bundle.current;
            let current_i = usize::from(current);
            let pot = &mut $bundle.pot;

            use platform_types::unscaled::xy;
            let mut coords: [unscaled::XY; models::holdem::MAX_PLAYERS as usize] = [
                xy!(0 0) ; models::holdem::MAX_PLAYERS as usize
            ];

            let hand_width = gfx::card::WIDTH.get() + (gfx::card::WIDTH.get() / 2) + 5;

            {
                let mut i = 0u8;
                'outer: for y in 0..4 {
                    for x in 0..7 {
                        coords[usize::from(i)] = xy!(
                            x * hand_width,
                            y * ((gfx::card::HEIGHT.get() / 2) + 1)
                            + SPACING_H.get()
                        );

                        i += 1;
                        if i >= 22 {
                            break 'outer;
                        }
                    }
                }
            }

            let hands_len = hands.len().u8();

            {
                let mut i = 0;
                for _ in hands.iter() {
                    let at = coords[i];

                    if current_i == i {
                        group.commands.draw_holdem_hand_underlight(
                            at.x,
                            at.y
                        );
                    }

                    i += 1;
                }
            }

            {
                let mut i: HandIndex = 0;
                for hand in hands.iter() {
                    let at = coords[usize::from(i)];

                    let show_if_player_owned = match group.ctx.hot {
                        HoldemHand(index) => index == i,
                        HoldemMenu(_)
                        | HoldemChartButton => true,
                        _ => false,
                    } && current == i;

                    if pot.has_folded(i) {
                        let facing = if let Some(_personality) = &state.table.personalities[current_i] {
                            // TODO make decision based on personality
                            if cfg!(debug_assertions) {
                                Facing::Up(hand)
                            } else {
                                Facing::Down
                            }
                        } else {
                            if show_if_player_owned {
                                Facing::Up(hand)
                            } else {
                                Facing::Down
                            }
                        };

                        group.commands.draw_folded_holdem_hand(
                            facing,
                            at.x,
                            at.y,
                        );
                    } else {
                        let facing = if show_if_player_owned
                        && state.table.personalities[current_i].is_none() {
                            Facing::Up(hand)
                        } else {
                            Facing::Down
                        };

                        group.commands.draw_holdem_hand(
                            facing,
                            at.x,
                            at.y,
                        );
                    }

                    i += 1;
                }
            }

            // The total bet needed to call
            let call_amount = pot.call_amount();
            let minimum_raise_total = call_amount + min_money_unit.get();
            // The amount extra needed to call
            let call_remainder = call_amount.saturating_sub(
                pot.amount_for(current)
            );
            // The amount that would be leftover if the player was to call
            let call_leftover = state.table.moneys[current_i]
                .checked_sub(call_remainder);

            let allowed_kind_mode =
                if call_remainder > 0 {
                    AllowedKindMode::All
                } else if call_leftover.unwrap_or(0) > 0 {
                    AllowedKindMode::NoFolding
                } else {
                    AllowedKindMode::AllIn
                };

            const ACTION_KIND: ui::HoldemMenuId = 0;
            const MONEY_AMOUNT: ui::HoldemMenuId = 1;
            const SUBMIT: ui::HoldemMenuId = 2;
            const MENU_KIND_ONE_PAST_MAX: ui::HoldemMenuId = 3;

            let mut i = 0;
            for _ in hands.iter() {
                match group.ctx.hot {
                    HoldemHand(mut index) if usize::from(index) == i => {
                        stack_money_text!(money_text = state.table.moneys[i]);

                        group.commands.draw_nine_slice(
                            gfx::NineSlice::Button,
                            HAND_DESC_RECT
                        );

                        {
                            let x = HAND_DESC_RECT.x + SPACING_W;
                            let mut y = HAND_DESC_RECT.y + gfx::CHAR_H;
                            group.commands.print_chars(
                                &money_text,
                                x,
                                y,
                                TEXT
                            );
                            y += gfx::CHAR_LINE_ADVANCE;

                            if usize::from(dealer) == i {
                                group.commands.print_chars(
                                    b"dealer",
                                    x,
                                    y,
                                    TEXT
                                );
                            }
                            y += gfx::CHAR_LINE_ADVANCE;

                            if current_i == i {
                                group.commands.print_chars(
                                    b"current",
                                    x,
                                    y,
                                    TEXT
                                );
                            }
                        }

                        if group.input.pressed_this_frame(Button::LEFT) {
                            if index == 0 {
                                group.ctx.set_next_hot(HoldemChartButton);
                            } else {
                                index -= 1;
                                group.ctx.set_next_hot(HoldemHand(index));
                            }
                        } else if group.input.pressed_this_frame(Button::RIGHT) {
                            index += 1;
                            if index >= hands_len {
                                group.ctx.set_next_hot(HoldemChartButton);
                            } else {
                                group.ctx.set_next_hot(HoldemHand(index));
                            }
                        } else if group.input.pressed_this_frame(Button::A) {
                            $bundle.selection.action_kind = match (allowed_kind_mode, $bundle.selection.action_kind) {
                                (AllowedKindMode::NoFolding, ActionKind::Fold) => ActionKind::Call,
                                (AllowedKindMode::All, action_kind)
                                | (AllowedKindMode::NoFolding, action_kind) => action_kind,
                                (AllowedKindMode::AllIn, _) => ActionKind::Call,
                            };
                            group.ctx.set_next_hot(HoldemMenu(ACTION_KIND));
                        } else {
                            group.ctx.set_next_hot(HoldemHand(index));
                        }

                        break
                    }
                    _ => {}
                }

                i += 1;
            }

            {
                let mut i = 0;
                for _ in hands.iter() {
                    let at = coords[i];

                    match group.ctx.hot {
                        HoldemHand(index) if usize::from(index) == i => {
                            group.commands.draw_holdem_hand_selected(
                                at.x,
                                at.y
                            );
                        },
                        _ => {},
                    };

                    i += 1;
                }
            }

            const MENU_H: unscaled::H = unscaled::h_const_div(
                command::HEIGHT_H,
                6
            );

            const MENU_RECT: unscaled::Rect = unscaled::Rect {
                x: unscaled::X(0),
                y: unscaled::y_const_add_h(
                    unscaled::Y(0),
                    unscaled::h_const_sub(
                        command::HEIGHT_H,
                        MENU_H
                    )
                ),
                w: command::WIDTH_W,
                h: MENU_H,
            };

            const HAND_DESC_H: unscaled::H = unscaled::h_const_div(
                command::HEIGHT_H,
                4
            );

            const HAND_DESC_RECT: unscaled::Rect = unscaled::Rect {
                x: unscaled::X(0),
                y: unscaled::y_const_add_h(
                    unscaled::Y(0),
                    unscaled::h_const_sub(
                        command::HEIGHT_H,
                        HAND_DESC_H
                    )
                ),
                w: command::WIDTH_W,
                h: HAND_DESC_H,
            };

            {
                let w = unscaled::W(50);
                let h = unscaled::H(50);

                if do_button(
                    group,
                    ButtonSpec {
                        id: HoldemChartButton,
                        rect: unscaled::Rect {
                            x: unscaled::X(0) + ((command::WIDTH_W) - w),
                            y: HAND_DESC_RECT.y - h,
                            w,
                            h,
                        },
                        text: b"chart",
                    }
                ) {
                    $bundle.modal = Modal::Chart;
                }
            }

            match group.ctx.hot {
                HoldemChartButton => {
                    group.commands.draw_nine_slice(
                        gfx::NineSlice::Button,
                        HAND_DESC_RECT
                    );

                    if group.input.pressed_this_frame(Button::LEFT)
                    || group.input.pressed_this_frame(Button::UP) {
                        group.ctx.set_next_hot(HoldemHand(hands_len - 1));
                    } else if group.input.pressed_this_frame(Button::RIGHT)
                    || group.input.pressed_this_frame(Button::DOWN) {
                        group.ctx.set_next_hot(HoldemHand(0));
                    }
                }
                _ => {}
            }

            if let Zero = group.ctx.hot {
                group.ctx.set_next_hot(HoldemHand(0));
            }

            {
                let mut y = COMMUNITY_BASE_Y;
                for amount in pot.individual_pots(&state.table.moneys) {
                    stack_money_text!(main_pot_text = amount);

                    group.commands.print_chars(
                        &main_pot_text,
                        COMMUNITY_BASE_X - pre_nul_len(&main_pot_text) * gfx::CHAR_ADVANCE,
                        y,
                        TEXT
                    );

                    y += gfx::CHAR_LINE_ADVANCE;
                }

                // TODO confirm this looks okay with the maximum number of amounts
                // which would be some function of MAX_PLAYERS. Exactly MAX_PLAYERS?
            }

            if $bundle.selection.bet < minimum_raise_total {
                $bundle.selection.bet = minimum_raise_total;
            }
            if $bundle.selection.bet > state.table.moneys[current_i] {
                $bundle.selection.bet = state.table.moneys[current_i];
            }

            let action_opt = match (
                pot.has_folded(current),
                &state.table.personalities[current_i]
            ) {
                (true, _) => Some(Action::Fold),
                (false, Some(_personality)) => {
                    // TODO Base choice of action off of personality

                    let hand = hands.get(current)
                                .map(|&h| h)
                                .unwrap_or_default();

                    let mut action = match $community_opt {
                        None => {
                            let probability = hand_win_probability(hand);
                            if probability >= SEVENTY_FIVE_PERCENT {
                                let multiple = Money::from(xs::range(rng, 3..6));
                                Action::Raise(minimum_raise_total + large_blind_amount.get().saturating_mul(multiple))
                            } else if probability >= FIFTY_PERCENT {
                                if xs::range(rng, 0..5) == 0 {
                                    // Don't be perfectly predictable!
                                    gen_action(
                                        rng,
                                        ActionSpec {
                                            one_past_max_money: NonZeroMoney::MIN.saturating_add(state.table.moneys[current_i]),
                                            min_money_unit,
                                            minimum_raise_total,
                                        }
                                    )
                                } else {
                                    Action::Call
                                }
                            } else {
                                Action::Fold
                            }
                        },
                        Some(community_cards) => {
                            let own_eval = evaluate::holdem_hand(
                                community_cards,
                                hand,
                            );

                            let mut other_hands = ALL_SORTED_HANDS.iter()
                                .filter(|h| {
                                    let is_already_used =
                                    h[0] == hand[0]
                                    || h[0] == hand[1]
                                    || h[1] == hand[0]
                                    || h[1] == hand[1]
                                    || community_cards.contains(h[0])
                                    || community_cards.contains(h[1]);

                                    !is_already_used
                                });

                            let mut below_count = 0;
                            let mut equal_count = 0;
                            let mut above_count = 0;

                            for other_hand in other_hands {
                                use core::cmp::Ordering::*;
                                let other_eval = evaluate::holdem_hand(
                                    community_cards,
                                    *other_hand,
                                );

                                match own_eval.cmp(&other_eval) {
                                    Less => {
                                        below_count += 1;
                                    },
                                    Equal => {
                                        equal_count += 1;
                                    },
                                    Greater => {
                                        below_count += 1;
                                    },
                                }
                            }

                            if below_count > (equal_count + above_count) {
                                // TODO raise sometimes
                                Action::Call
                            } else {
                                Action::Fold
                            }
                        }
                    };

                    match action {
                        Action::Fold => {
                            if call_remainder == 0 {
                                action = Action::Call;
                            }
                        },
                        Action::Call => {},
                        Action::Raise(raise_amount) => {
                            if state.table.moneys[current_i]
                                .checked_sub(raise_amount)
                                .is_none() {
                                action = Action::Raise(state.table.moneys[current_i]);
                            }
                        },
                    }

                    Some(action)
                },
                (false, None) => {
                    match group.ctx.hot {
                        HoldemMenu(menu_id) => {
                            stack_money_text!(money_text = state.table.moneys[current_i]);

                            group.commands.draw_nine_slice(
                                gfx::NineSlice::Button,
                                MENU_RECT
                            );

                            {
                                let x = MENU_RECT.x + SPACING_W;
                                let mut y = MENU_RECT.y + SPACING_H;
                                group.commands.print_chars(
                                    &money_text,
                                    x,
                                    y,
                                    TEXT
                                );
                                y += gfx::CHAR_LINE_ADVANCE;
                            }

                            let player_action_opt = {
                                let mut x = MENU_RECT.x + SPACING_W * 10;
                                let y = MENU_RECT.y + SPACING_H;

                                let action_kind_rect = unscaled::Rect {
                                    x,
                                    y,
                                    w: unscaled::W(50),
                                    h: MENU_RECT.h - SPACING_H * 2,
                                };

                                let action_kind_text = $bundle.selection.action_kind.text();

                                {
                                    let xy = gfx::center_line_in_rect(
                                        action_kind_text.len() as _,
                                        action_kind_rect,
                                    );
                                    group.commands.print_chars(
                                        action_kind_text,
                                        xy.x,
                                        xy.y,
                                        TEXT
                                    );
                                }

                                if allowed_kind_mode != AllowedKindMode::AllIn {
                                    ui::draw_quick_select(
                                        group,
                                        action_kind_rect,
                                        HoldemMenu(ACTION_KIND),
                                    );
                                } else {
                                    group.ctx.set_next_hot(HoldemMenu(SUBMIT));
                                }

                                let money_rect = unscaled::Rect {
                                    x: action_kind_rect.x + action_kind_rect.w,
                                    ..action_kind_rect
                                };

                                match $bundle.selection.action_kind {
                                    ActionKind::Raise => {
                                        draw_money_in_rect!(group, $bundle.selection.bet, money_rect);

                                        ui::draw_quick_select(
                                            group,
                                            money_rect,
                                            HoldemMenu(MONEY_AMOUNT),
                                        );
                                    }
                                    ActionKind::Call => {
                                        match allowed_kind_mode {
                                            AllowedKindMode::All
                                            | AllowedKindMode::NoFolding => {
                                                draw_money_in_rect!(group, call_remainder, money_rect);
                                            },
                                            AllowedKindMode::AllIn => {
                                                let label = b"all-in";
                                                let xy = gfx::center_line_in_rect(
                                                    label.len() as _,
                                                    money_rect,
                                                );
                                                group.commands.print_chars(
                                                    label,
                                                    xy.x,
                                                    xy.y,
                                                    6
                                                );
                                            }
                                        }
                                    }
                                    ActionKind::Fold => {}
                                }

                                if do_button(
                                    group,
                                    ButtonSpec {
                                        id: HoldemMenu(SUBMIT),
                                        rect: unscaled::Rect {
                                            x: action_kind_rect.x + action_kind_rect.w + action_kind_rect.w,
                                            ..action_kind_rect
                                        },
                                        text: b"submit",
                                    }
                                ) {
                                    Some(match $bundle.selection.action_kind {
                                        ActionKind::Fold => Action::Fold,
                                        ActionKind::Call => Action::Call,
                                        ActionKind::Raise => Action::Raise($bundle.selection.bet),
                                    })
                                } else {
                                    None
                                }
                            };

                            if group.input.pressed_this_frame(Button::B) {
                                group.ctx.set_next_hot(HoldemHand(current));
                            } else if group.input.pressed_this_frame(Button::LEFT) {
                                let mut new_id = menu_id;
                                new_id = match new_id.checked_sub(1) {
                                    Some(new_id) => new_id,
                                    None => MENU_KIND_ONE_PAST_MAX - 1,
                                };

                                if new_id == MONEY_AMOUNT
                                && $bundle.selection.action_kind != ActionKind::Raise {
                                    new_id = match new_id.checked_sub(1) {
                                        Some(new_id) => new_id,
                                        None => MENU_KIND_ONE_PAST_MAX - 1,
                                    };
                                }

                                group.ctx.set_next_hot(HoldemMenu(new_id));
                            } else if group.input.pressed_this_frame(Button::RIGHT) {
                                let mut new_id = menu_id;
                                new_id += 1;
                                if new_id >= MENU_KIND_ONE_PAST_MAX {
                                    new_id = 0;
                                }

                                if new_id == MONEY_AMOUNT
                                && $bundle.selection.action_kind != ActionKind::Raise {
                                    new_id += 1;
                                    if new_id >= MENU_KIND_ONE_PAST_MAX {
                                        new_id = 0;
                                    }
                                }

                                group.ctx.set_next_hot(HoldemMenu(new_id));
                            } else {
                                match menu_id {
                                    ACTION_KIND => {
                                        if group.input.pressed_this_frame(Button::UP) {
                                            $bundle.selection.action_kind = $bundle.selection.action_kind.next_up(allowed_kind_mode);
                                        } else if group.input.pressed_this_frame(Button::DOWN) {
                                            $bundle.selection.action_kind = $bundle.selection.action_kind.next_down(allowed_kind_mode);
                                        }
                                    }
                                    MONEY_AMOUNT => {
                                        if group.input.pressed_this_frame(Button::UP) {
                                            $bundle.selection.bet = $bundle.selection.bet.saturating_add(min_money_unit.get());
                                        } else if group.input.pressed_this_frame(Button::DOWN) {
                                            $bundle.selection.bet = $bundle.selection.bet.saturating_sub(min_money_unit.get());
                                        }
                                    }
                                    _ => {}
                                }
                            }

                            player_action_opt
                        }
                        _ => {
                            None
                        }
                    }
                }
            };

            match $bundle.modal {
                // TODO only decide on action if modal is nothing?
                Modal::Nothing => {},
                Modal::Chart => {
                    group.commands.draw_nine_slice(
                        gfx::NineSlice::Window,
                        FULLSCREEN_MODAL_RECT
                    );

                    // TODO? Could prebake many of these chart related calculations
                    // instead of redoing them so often.
                    #[derive(Clone, Copy)]
                    enum ChartElem {
                        LineBreak,
                        Title(&'static [u8]),
                        Hand(Hand),
                    }

                    const SUITED_CHART_ELEMS_LEN: usize = 92;
                    const SUITED_CHART_ELEMS: [ChartElem; SUITED_CHART_ELEMS_LEN] = {
                        use ChartElem::*;

                        // Ace at the low index because ace high.
                        let clubs = [0, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

                        let mut output = [ChartElem::LineBreak; SUITED_CHART_ELEMS_LEN];

                        let mut index = 0;

                        output[index] = Title(b"suited hands");
                        index += 1;

                        let mut card_1_i = 0;
                        while card_1_i < clubs.len() {
                            output[index] = LineBreak;
                            index += 1;

                            let card_1 = clubs[card_1_i];
                            let mut card_2_i = card_1_i + 1;

                            while card_2_i < clubs.len() {
                                let card_2 = clubs[card_2_i];

                                let hand = if card_1 < card_2 {
                                    [card_1, card_2]
                                } else {
                                    [card_2, card_1]
                                };
                                output[index] = Hand(hand);
                                index += 1;

                                card_2_i += 1;
                            }

                            card_1_i += 1;
                        }

                        output
                    };

                    const UNSUITED_CHART_ELEMS_LEN: usize = 106;
                    const UNSUITED_CHART_ELEMS: [ChartElem; UNSUITED_CHART_ELEMS_LEN] = {
                        use ChartElem::*;

                        // Ace at the low index because ace high.
                        let clubs = [0, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

                        let mut output = [ChartElem::LineBreak; UNSUITED_CHART_ELEMS_LEN];

                        let mut index = 0;

                        output[index] = Title(b"unsuited hands");
                        index += 1;

                        let mut card_1_i = 0;
                        while card_1_i < clubs.len() {
                            output[index] = LineBreak;
                            index += 1;

                            let card_1 = clubs[card_1_i];
                            let mut card_2_i = card_1_i;

                            while card_2_i < clubs.len() {
                                let card_2 = clubs[card_2_i];

                                let hand = if card_1 < card_2 {
                                    [card_1, card_2 + models::RANK_COUNT]
                                } else {
                                    [card_2, card_1 + models::RANK_COUNT]
                                };
                                output[index] = Hand(hand);
                                index += 1;

                                card_2_i += 1;
                            }

                            card_1_i += 1;
                        }

                        output
                    };

                    struct ChartThreshold {
                        colour: PaletteIndex,
                        threshold: Probability,
                        text: &'static [u8],
                    }

                    const CHART_THRESHOLDS: [ChartThreshold; 4] = [
                        ChartThreshold {
                            colour: platform_types::BLUE_INDEX,
                            threshold: EIGHTY_SEVEN_POINT_FIVE_PERCENT,
                            text: b"> 87.5%",
                        },
                        ChartThreshold {
                            colour: platform_types::GREEN_INDEX,
                            threshold: SEVENTY_FIVE_PERCENT,
                            text: b"> 75%",
                        },
                        ChartThreshold {
                            colour: platform_types::YELLOW_INDEX,
                            threshold: FIFTY_PERCENT,
                            text: b"> 50%",
                        },
                        ChartThreshold {
                            colour: platform_types::RED_INDEX,
                            threshold: 0,
                            text: b"<= 50%",
                        },
                    ];

                    macro_rules! render_chart {
                        ($x_start: expr, $y_start: expr, $iter: expr) => ({
                            let x_start = $x_start;
                            let mut x = x_start;
                            let mut y = $y_start;
                            for elem in $iter {
                                match elem {
                                    ChartElem::LineBreak => {
                                        y += chart_block::HEIGHT;
                                        x = x_start;
                                    },
                                    ChartElem::Title(title) => {
                                        group.commands.print_chars(
                                            title,
                                            x,
                                            y,
                                            TEXT
                                        );
                                    },
                                    ChartElem::Hand(hand) => {
                                        let probability = hand_win_probability(hand);

                                        let mut colour_index = 0;
                                        for ChartThreshold { colour, threshold, .. } in CHART_THRESHOLDS {
                                            if probability > threshold {
                                                colour_index = colour;
                                                break
                                            }
                                        }

                                        group.commands.draw_chart_block(
                                            x,
                                            y,
                                            colour_index
                                        );

                                        let mut hand_text = models::holdem::short_hand_text(hand);

                                        group.commands.print_chars(
                                            &hand_text,
                                            x + CHAR_SPACING_W,
                                            y + CHAR_SPACING_H,
                                            TEXT
                                        );

                                        x += chart_block::WIDTH;
                                    },
                                }
                            }

                            (x, y)
                        })
                    }
                    let (_, y) = render_chart!(
                        unscaled::X(0) + chart_block::WIDTH + SPACING_W,
                        unscaled::Y(0) + SPACING_H,
                        SUITED_CHART_ELEMS
                    );

                    let (_, _) = render_chart!(
                        unscaled::X(0) + SPACING_W,
                        y + SPACING_H,
                        UNSUITED_CHART_ELEMS
                    );

                    {
                        let x = unscaled::X(0) + unscaled::Inner::from(models::RANK_COUNT + 1) * chart_block::WIDTH + SPACING_W;
                        let mut y = unscaled::Y(0) + SPACING_H;
                        for ChartThreshold { colour, threshold, text } in CHART_THRESHOLDS {
                            group.commands.draw_chart_block(
                                x,
                                y,
                                colour
                            );

                            group.commands.print_chars(
                                text,
                                x + chart_block::WIDTH + SPACING_W,
                                y + CHAR_SPACING_H,
                                TEXT
                            );

                            y += chart_block::HEIGHT + CHAR_SPACING_H;
                        }
                        
                    }

                    if group.input.pressed_this_frame(Button::B) {
                        group.ctx.set_next_hot(HoldemChartButton);
                        $bundle.modal = Modal::Nothing;
                    }
                }
            }

            if let Some(action) = action_opt {
                let bet = match action {
                    Action::Fold => PotAction::Fold,
                    Action::Call => {
                        match call_leftover {
                            Some(new_amount) => {
                                state.table.moneys[current_i] = new_amount;
                                PotAction::Bet(call_remainder)
                            },
                            None => {
                                let bet = PotAction::Bet(
                                    state.table.moneys[current_i]
                                );
                                state.table.moneys[current_i] = 0;
                                bet
                            }
                        }
                    },
                    Action::Raise(raise_amount) => {
                        // The total bet needed to call
                        let call_amount = pot.call_amount();
                        let minimum_raise_total = call_amount + min_money_unit.get();
                        // The amount extra needed to call
                        let call_remainder = call_amount.saturating_sub(
                            pot.amount_for(current)
                        );
                        // The amount that would be leftover if the player was to call
                        let call_leftover = state.table.moneys[current_i]
                            .checked_sub(call_remainder);

                        match call_leftover {
                            Some(_) => {
                                match
                                    state.table.moneys[current_i]
                                    .checked_sub(raise_amount)
                                {
                                    Some(new_amount) => {
                                        state.table.moneys[current_i] = new_amount;
                                        PotAction::Bet(raise_amount)
                                    },
                                    None => {
                                        debug_assert!(
                                            false,
                                            "player {} raised {} with only {}",
                                            $bundle.current,
                                            raise_amount,
                                            state.table.moneys[current_i],
                                        );
                                        PotAction::Bet(raise_amount)
                                    }
                                }
                            },
                            None => {
                                let bet = PotAction::Bet(
                                    state.table.moneys[current_i]
                                );
                                state.table.moneys[current_i] = 0;
                                bet
                            }
                        }
                    },
                };

                pot.push_bet($bundle.current, bet);

                $bundle.current += 1;
                if $bundle.current >= hands.len().u8() {
                    $bundle.current = 0;
                }

                pot.round_outcome(&state.table.moneys)
            } else {
                RoundOutcome::Undetermined
            }
        })
    }

    macro_rules! next_bundle {
        ($bundle: ident =
            $hands: expr,
            $deck: expr,
            $dealer: expr,
            $pot: expr
        ) => {
            let hands = $hands;
            let deck = $deck;
            let dealer = $dealer;
            let player_count = hands.len();
            let mut pot = $pot;

            pot.reset_for_new_round();

            let current = if player_count == HandLen::Two {
                // When head-to-head, the dealer acts first.
                dealer
            } else {
                // Normally, the player after the dealer acts first.
                let mut index = dealer + 1;
                if index >= hands.len().u8() {
                    index = 0;
                }
                index
            };

            let $bundle = HoldemStateBundle {
                hands,
                deck,
                dealer,
                current,
                pot,
                selection: HoldemMenuSelection::default(),
                modal: Modal::default(),
            };
        }
    }

    macro_rules! finish_round {
        () => {
            #[cfg(debug_assertions)]
            let expected_user_count = {
                state.table.moneys
                    .iter()
                    .zip(
                        state.table.personalities
                            .iter()
                    )
                    .filter(|(&m, p)| m > 0 && p.is_none())
                    .count()
            };

            // Condense players down
            {
                let mut pairs: [(Money, Option<CpuPersonality>); MAX_PLAYERS as usize]
                    = <_>::default();

                let mut pair_index = 0;
                for i in 0..state.table.moneys.len() {
                    if state.table.moneys[i] == 0 {
                        continue
                    }
                    pairs[pair_index] = (
                        state.table.moneys[i],
                        state.table.personalities[i].take(),
                    );
                    pair_index += 1;
                }

                for i in 0..state.table.moneys.len() {
                    state.table.moneys[i] = 0;
                    state.table.personalities[i] = None;
                }

                for i in 0..state.table.moneys.len() {
                    let money = pairs[i].0;
                    if money == 0 {
                        break
                    }
                    let personality = pairs[i].1.take();
                    state.table.moneys[i] = money;
                    state.table.personalities[i] = personality;
                }
            }

            debug_assert_eq!(
                state.table.moneys
                    .iter()
                    .zip(
                        state.table.personalities
                            .iter()
                    )
                    .filter(|(&m, p)| m > 0 && p.is_none())
                    .count(),
                expected_user_count,
                "After condensing personalities user count did not match expected_user_count!"
            );

            let remaining_player_count = {
                let mut remaining_player_count = 0;

                // Assumes we just condensed the players
                for money in state.table.moneys.iter() {
                    if *money == 0 {
                        break
                    }
                    remaining_player_count += 1;
                }

                remaining_player_count
            };

            debug_assert!(remaining_player_count > 0);

            let mut only_cpus_left = true;
            // Assumes we just condensed the players
            for i in 0..state.table.moneys.len() {
                let money = state.table.moneys[i];
                if money == 0 {
                    break
                }
                if state.table.personalities[i].is_none() {
                    only_cpus_left = false;
                }
            }

            if only_cpus_left && state.table.skip == SkipState::Skip {
                // TODO? Actually simulate the remaining turns, maybe with a timeout?
                speaker.request_sfx(SFX::CardPlace);
                state.table.state = <_>::default();
            } else {
                match HandLen::try_from(remaining_player_count){
                    Ok(player_count) => {
                        let (hands, deck) = models::holdem::deal(rng, player_count);

                        let dealer = gen_hand_index(rng, player_count);

                        let mut pot = Pot::with_capacity(player_count, 16);

                        collect_blinds!(hands player_count dealer pot);

                        next_bundle!(bundle = hands, deck, dealer, pot);

                        speaker.request_sfx(SFX::CardPlace);
                        state.table.state = PreFlop {
                            bundle,
                        };
                    },
                    Err(_) => {
                        // TODO show a winner screen with more winner info.
                        if state.table.personalities[0].is_none() {
                            println!("User wins!");
                        } else {
                            println!("Cpu player wins!");
                        }

                        speaker.request_sfx(SFX::CardPlace);
                        state.table.state = <_>::default();
                    },
                };
            }
        }
    }

    const FULLSCREEN_MODAL_RECT: unscaled::Rect = unscaled::Rect {
        x: unscaled::X(0),
        y: unscaled::Y(0),
        w: command::WIDTH_W,
        h: command::HEIGHT_H,
    };

    macro_rules! award_now {
        ($hand_index: ident , $pot: expr) => {
            let i = usize::from($hand_index);
            state.table.moneys[i] = state.table.moneys[i]
                .saturating_add($pot.total());

            finish_round!();
        }
    }

    macro_rules! collect_blinds {
        ($hands: ident $(,)? $player_count: ident $(,)? $dealer: ident $(,)? $pot: ident) => {
            let hands = &$hands;
            let player_count = $player_count;
            let dealer = $dealer;
            let pot = &mut $pot;

            {
                let mut index = dealer;
                if player_count == HandLen::Two {
                    // When head-to-head, the dealer posts the small blind
                    // and the other player posts the big blind, so don't
                    // advance.
                } else {
                    index += 1;
                    if index >= hands.len().u8() {
                        index = 0;
                    }
                };

                let (new_total, subbed) =
                    match state.table.moneys[usize::from(index)].checked_sub(small_blind_amount.get()) {
                        Some(difference) => (difference, small_blind_amount.get()),
                        None => (0, state.table.moneys[usize::from(index)]),
                    };
                state.table.moneys[usize::from(index)] = new_total;
                pot.push_bet(index, PotAction::Bet(subbed));

                index += 1;
                if index >= hands.len().u8() {
                    index = 0;
                }

                let (new_total, subbed) =
                    match state.table.moneys[usize::from(index)].checked_sub(large_blind_amount.get()) {
                        Some(difference) => (difference, large_blind_amount.get()),
                        None => (0, state.table.moneys[usize::from(index)]),
                    };
                state.table.moneys[usize::from(index)] = new_total;
                pot.push_bet(index, PotAction::Bet(subbed));
            }
        }
    }

    match &mut state.table.state {
        Undealt {
            ref mut player_count,
            ref mut starting_money,
        } => {
            let group = new_group!();

            if do_button(
                group,
                ButtonSpec {
                    id: BackToTitleScreen,
                    rect: unscaled::Rect {
                        x: unscaled::X(0),
                        y: unscaled::Y(0),
                        w: unscaled::W(50),
                        h: unscaled::H(50),
                    },
                    text: b"back",
                }
            ) {
                cmd = ModeCmd::BackToTitleScreen;
            }

            let player_count_rect = unscaled::Rect {
                x: unscaled::X(100),
                y: unscaled::Y(100),
                w: unscaled::W(50),
                h: unscaled::H(100),
            };

            let player_count_text = player_count.text().as_bytes();

            {
                let xy = gfx::center_line_in_rect(
                    player_count_text.len() as _,
                    player_count_rect,
                );
                group.commands.print_chars(
                    player_count_text,
                    xy.x,
                    xy.y,
                    TEXT
                );
            }
            {
                let players_label = b"players";

                let xy = gfx::center_line_in_rect(
                    players_label.len() as _,
                    player_count_rect,
                );

                group.commands.print_chars(
                    players_label,
                    xy.x,
                    xy.y + gfx::CHAR_H,
                    TEXT
                );
            }

            ui::draw_quick_select(
                group,
                player_count_rect,
                PlayerCountSelect,
            );

            let starting_money_rect = unscaled::Rect {
                x: unscaled::X(150),
                y: unscaled::Y(100),
                w: unscaled::W(50),
                h: unscaled::H(100),
            };

            draw_money_in_rect!(group, starting_money, starting_money_rect);

            ui::draw_quick_select(
                group,
                starting_money_rect,
                StartingMoneySelect,
            );

            if do_button(
                group,
                ButtonSpec {
                    id: Submit,
                    rect: unscaled::Rect {
                        x: starting_money_rect.x + starting_money_rect.w,
                        y: unscaled::Y(100),
                        w: unscaled::W(50),
                        h: unscaled::H(100),
                    },
                    text: b"submit",
                }
            ) {
                for i in 0..player_count.usize() {
                    state.table.moneys[i] = *starting_money;
                }

                state.table.personalities[0] = None;
                // TODO Make each element of this array user selectable too.
                // Start at 1 to make the first player user controlled
                for i in 1..player_count.usize() {
                    state.table.personalities[i] = Some(CpuPersonality{});
                }

                let (hands, deck) = models::holdem::deal(rng, *player_count);

                let dealer = gen_hand_index(rng, *player_count);

                let mut pot = Pot::with_capacity(*player_count, 16);

                let player_count = *player_count;
                collect_blinds!(hands player_count dealer pot);

                next_bundle!(bundle = hands, deck, dealer, pot);

                speaker.request_sfx(SFX::CardPlace);
                state.table.state = PreFlop {
                    bundle,
                };
            } else {
                let menu = [BackToTitleScreen, PlayerCountSelect, StartingMoneySelect, Submit];

                match group.ctx.hot {
                    BackToTitleScreen => {
                        let menu_i = 0;

                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up | Dir::Left) => {},
                            Some(Dir::Down | Dir::Right) => {
                                group.ctx.set_next_hot(menu[menu_i + 1]);
                            }
                            None => {}
                        }
                    }
                    StartingMoneySelect => {
                        let menu_i = 2;
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up) => {
                                *starting_money = starting_money.saturating_add(min_money_unit.get());
                            },
                            Some(Dir::Down) => {
                                *starting_money = starting_money.saturating_sub(min_money_unit.get());
                                if *starting_money == 0 {
                                    *starting_money = min_money_unit.get();
                                }
                            },
                            Some(Dir::Left) => {
                                group.ctx.set_next_hot(menu[menu_i - 1]);
                            }
                            Some(Dir::Right) => {
                                group.ctx.set_next_hot(menu[menu_i + 1]);
                            }
                            None => {}
                        }
                    }
                    PlayerCountSelect => {
                        let menu_i = 1;
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up) => {
                                *player_count = player_count.saturating_add_1();
                            },
                            Some(Dir::Down) => {
                                *player_count = player_count.saturating_sub_1();
                            },
                            Some(Dir::Left) => {
                                group.ctx.set_next_hot(menu[menu_i - 1]);
                            }
                            Some(Dir::Right) => {
                                group.ctx.set_next_hot(menu[menu_i + 1]);
                            }
                            None => {}
                        }
                    }
                    Submit => {
                        let menu_i = menu.len() - 1;
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Left) => {
                                group.ctx.set_next_hot(menu[menu_i - 1]);
                            }
                            Some(Dir::Right) => {}
                            _ => {}
                        }
                    }
                    Zero => {
                        group.ctx.set_next_hot(PlayerCountSelect);
                    }
                    _ => {}
                }
            }
        },
        PreFlop { bundle } => {
            let group = new_group!();
            let outcome = do_holdem_hands!(group, bundle, None);

            match outcome {
                RoundOutcome::Undetermined => {},
                RoundOutcome::AdvanceToNext => {
                    let community_cards = bundle
                        .deck
                        .deal_community_cards()
                        .expect("Deck ran out!?");
                    next_bundle!(
                        new_bundle =
                            bundle.hands.clone(),
                            bundle.deck.clone(),
                            bundle.dealer,
                            bundle.pot.clone()
                    );
                    speaker.request_sfx(SFX::CardPlace);
                    state.table.state = PostFlop {
                        bundle: new_bundle,
                        community_cards
                    };
                },
                RoundOutcome::AwardNow(hand_index) => {
                    award_now!(hand_index, bundle.pot);
                },
            }
        },
        PostFlop { bundle, community_cards } => {
            let group = new_group!();

            group.commands.draw_holdem_community_cards(
                *community_cards,
                COMMUNITY_BASE_X,
                COMMUNITY_BASE_Y,
            );

            let outcome = do_holdem_hands!(group, bundle, Some(*community_cards));

            match outcome {
                RoundOutcome::Undetermined => {},
                RoundOutcome::AdvanceToNext => {
                    match *community_cards {
                        CommunityCards::Flop(flop) => {
                            bundle.deck.burn();
                            if let Some(turn) = bundle.deck.draw() {
                                *community_cards = CommunityCards::Turn(flop, turn);
                            } else {
                                debug_assert!(false, "Ran out of cards for turn!");
                            }

                            next_bundle!(
                                new_bundle =
                                    bundle.hands.clone(),
                                    bundle.deck.clone(),
                                    bundle.dealer,
                                    bundle.pot.clone()
                            );
                            speaker.request_sfx(SFX::CardPlace);
                            state.table.state = PostFlop {
                                bundle: new_bundle,
                                community_cards: *community_cards,
                            };
                        },
                        CommunityCards::Turn(flop, turn) => {
                            bundle.deck.burn();
                            if let Some(river) = bundle.deck.draw() {
                                *community_cards = CommunityCards::River(flop, turn, river);
                            } else {
                                debug_assert!(false, "Ran out of cards for river!");
                            }

                            next_bundle!(
                                new_bundle =
                                    bundle.hands.clone(),
                                    bundle.deck.clone(),
                                    bundle.dealer,
                                    bundle.pot.clone()
                            );
                            speaker.request_sfx(SFX::CardPlace);
                            state.table.state = PostFlop {
                                bundle: new_bundle,
                                community_cards: *community_cards,
                            };
                        }
                        CommunityCards::River(flop, turn, river) => {
                            speaker.request_sfx(SFX::CardPlace);
                            state.table.state = Showdown {
                                bundle: bundle.clone(),
                                full_board: [
                                    flop[0],
                                    flop[1],
                                    flop[2],
                                    turn,
                                    river
                                ],
                            };
                        }
                    }
                },
                RoundOutcome::AwardNow(hand_index) => {
                    award_now!(hand_index, bundle.pot);
                },
            }
        },
        Showdown { bundle, full_board } => {
            debug_assert!(bundle.pot.total() > 0);

            let group = new_group!();

            // If we'd be able to see something under the modal, sure.
            //let _outcome = do_holdem_hands!(group, bundle);

            // TODO draw a modal that shows who won how much, and have
            // a button to go on to the next game.

            group.commands.draw_nine_slice(
                gfx::NineSlice::Window,
                FULLSCREEN_MODAL_RECT
            );

            #[derive(Debug, Default)]
            struct Award {
                amount: Money,
                eval: evaluate::Eval,
            }
            type Awards = PerPlayer<[Award; MAX_POTS as usize]>;

            let awards: Awards = {
                debug_assert_eq!(
                    bundle.pot.eligibilities(&state.table.moneys)
                            .map(|(_, n)| n)
                            .sum::<Money>(),
                    bundle.pot.total(),
                    "Eligibilities did not match pot total!"
                );

                let mut awards = Awards::default();

                for (eligibile_players, amount) in bundle.pot.eligibilities(&state.table.moneys) {
                    let mut winner_count = 0;
                    let mut winners = [
                        (0, evaluate::Eval::WORST);
                        MAX_POTS as usize
                    ];

                    for player in eligibile_players.iter() {
                        let best_eval = {
                            let Some(hand) = bundle.hands.get(player) else {
                                debug_assert!(false, "Hand not found for {player}");
                                continue
                            };
                            evaluate::holdem_hand(
                                CommunityCards::from(*full_board),
                                *hand,
                            )
                        };

                        use core::cmp::Ordering::*;
                        match best_eval.cmp(&winners[0].1) {
                            Greater => {
                                winner_count = 1;
                                winners[winner_count - 1] = (player, best_eval);
                            },
                            Equal => {
                                winner_count += 1;
                                winners[winner_count - 1] = (player, best_eval);
                            },
                            Less => {
                                // next iteration
                            }
                        }
                    }

                    debug_assert!(winner_count > 0);

                    let award_amounts: PerPlayer<Money> = {
                        let mut award_amounts = PerPlayer::<Money>::default();

                        let mut remaining = amount;

                        debug_assert!(remaining % min_money_unit == 0);

                        // TODO? More efficient version of this?
                        // Will this actually ever be a bottleneck?
                        let mut i = 0;
                        while remaining > 0 {
                            remaining = remaining.saturating_sub(min_money_unit.get());
                            award_amounts[i] = award_amounts[i].saturating_add(min_money_unit.get());

                            i += 1;
                            if i >= usize::from(winner_count) {
                                i = 0;
                            }
                        }

                        award_amounts
                    };
                    for i in 0..winner_count {
                        let (winner_index, winner_eval) = winners[i];

                        let amount = award_amounts[i];

                        // Push an award on
                        for award in &mut awards[usize::from(winner_index)] {
                            if award.amount == 0 {
                                *award = Award {
                                    amount,
                                    eval: winner_eval,
                                };
                                break
                            }
                        }
                    }
                }

                debug_assert_eq!(
                    {
                        let mut total: Money = 0;

                        for award_array in awards.iter() {
                            for Award { amount, .. } in award_array {
                                total = total.saturating_add(*amount);
                            }
                        }

                        total
                    },
                    bundle.pot.total(),
                    "Awarded total did not match pot total!"
                );

                awards
            };

            {
                let mut y = unscaled::Y(gfx::CHAR_LINE_ADVANCE.get());
                for (i, award_array) in awards.iter().enumerate() {
                    let mut any_non_zero = false;
                    for Award { amount, .. } in award_array {
                        if *amount != 0 {
                            any_non_zero = true;
                            break
                        }
                    }
                    if !any_non_zero {
                        continue
                    }

                    let mut player_text = [0 as u8; 20];
                    player_text[0] = b'p';
                    player_text[1] = b'l';
                    player_text[2] = b'a';
                    player_text[3] = b'y';
                    player_text[4] = b'e';
                    player_text[5] = b'r';
                    player_text[6] = b' ';

                    let _cant_actually_fail = write!(
                        &mut player_text[7..],
                        "{i}",
                    );

                    group.commands.print_chars(
                        &player_text,
                        COMMUNITY_BASE_X - (pre_nul_len(&player_text) * gfx::CHAR_ADVANCE),
                        y,
                        TEXT
                    );

                    y += gfx::CHAR_LINE_ADVANCE;

                    for Award { amount, eval } in award_array {
                        if *amount == 0 {
                            break
                        }

                        stack_money_text!(amount_text = amount);

                        group.commands.print_chars(
                            &amount_text,
                            COMMUNITY_BASE_X - (pre_nul_len(&amount_text) * gfx::CHAR_ADVANCE),
                            y,
                            TEXT
                        );

                        stack_eval_text!(eval_text = eval);

                        group.commands.print_chars(
                            &eval_text ,
                            COMMUNITY_BASE_X + gfx::CHAR_ADVANCE,
                            y,
                            TEXT
                        );

                        y += gfx::CHAR_LINE_ADVANCE;
                    }

                    y += gfx::CHAR_LINE_ADVANCE;
                }
            }

            let w = unscaled::W(50);
            let h = unscaled::H(20);

            let submit_rect = unscaled::Rect {
                x: unscaled::X(0) + ((command::WIDTH_W/2) - (w/2)),
                y: unscaled::Y(0) + (command::HEIGHT_H - (h + SPACING_H)),
                w,
                h,
            };

            {
                let skip_rect = unscaled::Rect {
                    x: submit_rect.x - (w * 2),
                    y: submit_rect.y - submit_rect.h,
                    h: submit_rect.h * 2,
                    ..submit_rect
                };

                let skip_text: &[u8] = match state.table.skip {
                    SkipState::Skip => b"skip cpu only",
                    SkipState::Watch => b"watch cpu only",
                };

                {
                    let xy = gfx::center_line_in_rect(
                        skip_text.len() as _,
                        skip_rect,
                    );
                    group.commands.print_chars(
                        skip_text,
                        xy.x,
                        xy.y,
                        TEXT
                    );
                }

                ui::draw_quick_select(
                    group,
                    skip_rect,
                    SkipRemainderOfGameSelect,
                );
            }

            if do_button(
                group,
                ButtonSpec {
                    id: ShowdownSubmit,
                    rect: submit_rect,
                    text: b"submit",
                }
            ) {
                for (i, pots) in awards.iter().enumerate() {
                    for Award{ amount, .. } in pots {
                        state.table.moneys[i] = state.table.moneys[i].saturating_add(*amount);
                    }
                }

                finish_round!();
            } else {
                match group.ctx.hot {
                    SkipRemainderOfGameSelect => {
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up) => {
                                state.table.skip =
                                    match state.table.skip {
                                        SkipState::Skip => SkipState::Watch,
                                        SkipState::Watch => SkipState::Skip,
                                    };
                            },
                            Some(Dir::Down) => {
                                state.table.skip =
                                    match state.table.skip {
                                        SkipState::Skip => SkipState::Watch,
                                        SkipState::Watch => SkipState::Skip,
                                    };
                            },
                            Some(Dir::Left | Dir::Right) => {
                                group.ctx.set_next_hot(ShowdownSubmit);
                            }
                            None => {}
                        }
                    }
                    ShowdownSubmit => {
                         match input.dir_pressed_this_frame() {
                            Some(Dir::Left | Dir::Right) => {
                                group.ctx.set_next_hot(SkipRemainderOfGameSelect);
                            }
                            Some(Dir::Up | Dir::Down) | None => {}
                        }
                    }
                    _ => {
                        group.ctx.set_next_hot(ShowdownSubmit);
                    }
                }
            }
        },
    }

    cmd
}

fn pre_nul_len(
    text: &[u8],
) -> gfx::TextLength {
    let mut len = 0;
    for i in 0..text.len() as gfx::TextLength {
        // If it's max length, this being outside the `if`
        // ensures the length is accurate.
        len = i;
        if text[usize::from(i)] == b'\0' {
            break;
        }
    }
    len
}