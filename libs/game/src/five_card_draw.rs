use look_up::{
    five_card::{hand_win_probability},
};
use gfx::{card, pre_nul_len, Commands, SPACING_W, SPACING_H};
use models::{Action, ActionKind, ActionSpec, AllowedKindMode, BetKind, Card, CardBitset, RoundOutcome, ALL_CARDS, INITIAL_ANTE_AMOUNT, MIN_MONEY_UNIT, Deck, Money, MoneyInner, MoneyMove, NonZeroMoney, NonZeroMoneyInner, Pot, PotAction, Rank, gen_action, gen_deck, get_rank, ranks};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};
use probability::{EvalCount};
use probability::{FIFTY_PERCENT, SEVENTY_FIVE_PERCENT, EIGHTY_SEVEN_POINT_FIVE_PERCENT, Probability};

use std::io::Write;

use xs::Xs;

use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState};
use crate::ui::{self, draw_money_in_rect, stack_money_text, ButtonSpec, Id::*, do_button};

pub const MIN_PLAYERS: u8 = 2;
// At 9 players that's 45 of 52 cards at the start, so each player only gets to draw
// one card. That seems like the reasonable upper limit.
pub const MAX_PLAYERS: u8 = 9;

/// The index for a `Card` in a `Hand`.
// TODO will we need this?
//pub type CardIndex = u8;
type Hand = [Card; 5];

/// The index for a `Hand` in `Hands`, not for indexing into a `Hand`.
pub type HandIndex = u8;
pub const MAX_HAND_INDEX: u8 = MAX_PLAYERS - 1;

pub type HandsLen = u8;

type Hands = [Hand; MAX_PLAYERS as usize];

fn deal(rng: &mut Xs, player_count: PlayerCount) -> (Hands, Deck) {
    let mut deck = gen_deck(rng);

    let mut hands = Hands::default();

    let count = player_count.usize();

    for hand in (&mut hands[0..count]).iter_mut() {
        let (
            Some(card1),
            Some(card2),
            Some(card3),
            Some(card4),
            Some(card5),
        ) = (
            deck.draw(),
            deck.draw(),
            deck.draw(),
            deck.draw(),
            deck.draw(),
        )
            else { continue };
        *hand = [card1, card2, card3, card4, card5];
    }

    (hands, deck)
}


#[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum PlayerCount {
    #[default]
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

impl PlayerCount {
    pub fn saturating_add_1(self) -> Self {
        use PlayerCount::*;
        match self {
            Two => Three,
            Three => Four,
            Four => Five,
            Five => Six,
            Six => Seven,
            Seven => Eight,
            Eight => Nine,
            Nine => Nine,
        }
    }

    pub fn saturating_sub_1(self) -> Self {
        use PlayerCount::*;
        match self {
            Two
            | Three => Two,
            Four => Three,
            Five => Four,
            Six => Five,
            Seven => Six,
            Eight => Seven,
            Nine => Eight,
        }
    }

    pub fn text(self) -> &'static str {
        use PlayerCount::*;
        match self {
            Two => "2",
            Three => "3",
            Four => "4",
            Five => "5",
            Six => "6",
            Seven => "7",
            Eight => "8",
            Nine => "9",
        }
    }

    pub fn u8(self) -> u8 {
        use PlayerCount::*;
        match self {
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
            Six => 6,
            Seven => 7,
            Eight => 8,
            Nine => 9,
        }
    }

    pub fn usize(self) -> usize {
        usize::from(self.u8())
    }
}

impl TryFrom<u8> for PlayerCount {
    type Error = &'static str;

    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        use PlayerCount::*;
        match byte {
            0 | 1 => Err("player count was too small"),
            2 => Ok(Two),
            3 => Ok(Three),
            4 => Ok(Four),
            5 => Ok(Five),
            6 => Ok(Six),
            7 => Ok(Seven),
            8 => Ok(Eight),
            9 => Ok(Nine),
            _ => Err("player count was to big"),
        }
    }
}

pub fn gen_hand_index(rng: &mut Xs, player_count: PlayerCount) -> HandIndex {
    xs::range(rng, 0..player_count.u8() as _) as HandIndex
}

#[derive(Clone)]
pub struct Seats {
    pub moneys: [Money; MAX_PLAYERS as usize],
    pub personalities: [Personality; MAX_PLAYERS as usize],
    pub skip: SkipState,
    // TODO Increase these as the game goes on {
    pub ante: NonZeroMoneyInner,
    // }
}

impl Default for Seats {
    fn default() -> Self {
        Self {
            moneys: <_>::default(),
            personalities: <_>::default(),
            skip: <_>::default(),
            ante: MIN_MONEY_UNIT,
        }
    }
}

#[derive(Clone, Default)]
pub struct MenuSelection {
    pub action_kind: ActionKind,
    pub bet: MoneyInner,
}

#[derive(Clone)]
pub struct StateBundle {
    pub deck: Deck,
    pub hands: Hands,
    pub current: HandIndex,
    pub pot: Pot,
    pub player_count: PlayerCount,
    pub selection: MenuSelection,
}

#[derive(Clone)]
pub enum TableState {
    Undealt { player_count: PlayerCount, starting_money: MoneyInner },
    FirstRound {
        bundle: StateBundle,
    },
    Drawing {
        bundle: StateBundle,
    },
    SecondRound {
        bundle: StateBundle,
    },
    Showdown {
        bundle: StateBundle,
    },
}

impl Default for TableState {
    fn default() -> Self {
        Self::Undealt {
            player_count: <_>::default(),
            starting_money: 500,
        }
    }
}

#[derive(Clone, Default)]
pub struct Table {
    pub seats: Seats,
    pub state: TableState,
}

impl Table {
    pub fn selected(
        rng: &mut Xs,
        player_count: PlayerCount,
        mut moneys: [Money; MAX_PLAYERS as usize],
    ) -> Self {
        let mut pot: Pot = Pot::with_capacity(player_count.u8(), 16);

        let ante = MIN_MONEY_UNIT;

        for i in 0..player_count.u8() {
            pot.push_bet_of_kind(
                i, 
                PotAction::Bet(
                    moneys[usize::from(i)]
                        .take(ante.get())
                ),
                BetKind::Ante,
            );
        }

        let mut personalities: [Personality; MAX_PLAYERS as usize] = <_>::default();

        personalities[0] = None;
        // TODO Make each element of this array user selectable too.
        // Start at 1 to make the first player user controlled
        for i in 1..player_count.usize() {
            personalities[i] = Some(CpuPersonality{});
        }

        let (hands, deck) = deal(rng, player_count);

        let selected = gen_hand_index(rng, player_count);

        let current = if moneys[usize::from(selected)] == 0 {
            let mut index = selected + 1;
            while {
                if index >= player_count.u8() {
                    index = 0;
                }

                index != selected
                && moneys[usize::from(index)] == 0
            } {
                index += 1;
            }

            index
        } else {
            selected
        };

        // TODO handle case where the pot has all the money in it!
        Self {
            seats: Seats {
                moneys,
                personalities,
                skip: <_>::default(),
                ante,
            },
            state: TableState::FirstRound {
                bundle: StateBundle {
                    deck,
                    hands,
                    current,
                    pot,
                    player_count,
                    selection: <_>::default(),
                },
            },
        }
    }
}

pub struct State<'state> {
    pub rng: &'state mut Xs,
    pub ctx: &'state mut ui::Context,
    pub table: &'state mut Table
}

pub fn update_and_render(
    commands: &mut Commands,
    state: State<'_>,
    input: Input,
    speaker: &mut Speaker,
) -> ModeCmd {
    use TableState::*;
    use ui::Id::*;

    let rng = state.rng;

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

    let mut cmd = ModeCmd::NoOp;

    macro_rules! do_five_card_draw {
        ($group: ident $(,)? $bundle: ident) => ({
            let group = $group;
            let hands = &$bundle.hands;
            let current = $bundle.current;
            let current_i = usize::from(current);
            let player_count = $bundle.player_count;
            let pot = &mut $bundle.pot;

            use platform_types::unscaled::xy;
            // TODO Avoid overlapping hands
            let mut coords: [unscaled::XY; MAX_PLAYERS as usize] = [
                xy!(0 0) ; MAX_PLAYERS as usize
            ];

            // TODO derive this from a `gfx` const that we will know to keep in sync
            // with any changes to the way cards are arranged when drawing?
            let hand_width = gfx::Commands::FIVE_CARD_HAND_WIDTH.get() + SPACING_W.get();

            {
                let mut i = 0u8;
                'outer: for y in 0..3 {
                    for x in 0..3 {
                        coords[usize::from(i)] = xy!(
                            x * hand_width,
                            y * (gfx::card::HEIGHT.get() + SPACING_H.get())
                            + SPACING_H.get()
                        );

                        i += 1;
                        if i >= MAX_PLAYERS {
                            break 'outer;
                        }
                    }
                }
            }

            let hands_len: HandsLen = player_count.u8();

            {
                let mut i = 0;
                for _ in 0..hands_len {
                    let at = coords[i];

                    if current_i == i {
                        group.commands.draw_five_card_hand_underlight(
                            at.x,
                            at.y
                        );
                    }

                    i += 1;
                }
            }

            {
                let mut i: HandIndex = 0;
                for hand in &hands[0..(hands_len as usize)] {
                    let at = coords[usize::from(i)];

                    let show_if_player_owned = match group.ctx.hot {
                        FiveCardDrawHand(index) => index == i,
                        FiveCardDrawMenu(_) => true,
                        _ => false,
                    } && current == i;

                    use gfx::FiveCardFacing;
                    let facing = if show_if_player_owned
                    && state.table.seats.personalities[current_i].is_none() {
                        FiveCardFacing::Up(*hand)
                    } else {
                        FiveCardFacing::Down
                    };

                    group.commands.draw_five_card_hand(
                        facing,
                        at.x,
                        at.y,
                    );

                    i += 1;
                }
            }

            {
                let mut i = 0;
                for _ in hands.iter() {
                    let at = coords[i];

                    match group.ctx.hot {
                        FiveCardDrawHand(index) if usize::from(index) == i => {
                            group.commands.draw_five_card_hand_selected(
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

            // The total bet needed to call
            let call_amount = pot.call_amount();
            let minimum_raise_total = call_amount + MIN_MONEY_UNIT.get();
            // The amount extra needed to call
            let call_remainder = call_amount.saturating_sub(
                pot.amount_for(current)
            );
            // The amount that would be leftover if the player was to call
            let call_leftover = state.table.seats.moneys[current_i]
                .as_inner()
                .checked_sub(call_remainder);

            let allowed_kind_mode =
                if call_remainder > 0 {
                    AllowedKindMode::All
                } else if call_leftover.unwrap_or(0) > 0 {
                    AllowedKindMode::NoFolding
                } else {
                    AllowedKindMode::AllIn
                };

            const ACTION_KIND: ui::FiveCardDrawMenuId = 0;
            const MONEY_AMOUNT: ui::FiveCardDrawMenuId = 1;
            const SUBMIT: ui::FiveCardDrawMenuId = 2;
            const MENU_KIND_ONE_PAST_MAX: ui::FiveCardDrawMenuId = 3;

            let mut i = 0;
            for _ in 0..hands_len {
                match group.ctx.hot {
                    FiveCardDrawHand(mut index) if usize::from(index) == i => {
                        stack_money_text!(money_text = state.table.seats.moneys[i]);

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
                                group.ctx.set_next_hot(FiveCardDrawHand(hands_len - 1));
                            } else {
                                index -= 1;
                                group.ctx.set_next_hot(FiveCardDrawHand(index));
                            }
                        } else if group.input.pressed_this_frame(Button::RIGHT) {
                            index += 1;
                            if index >= hands_len {
                                group.ctx.set_next_hot(FiveCardDrawHand(0));
                            } else {
                                group.ctx.set_next_hot(FiveCardDrawHand(index));
                            }
                        } else if group.input.pressed_this_frame(Button::A) {
                            group.ctx.set_next_hot(FiveCardDrawMenu(ACTION_KIND));
                        } else {
                            group.ctx.set_next_hot(FiveCardDrawHand(index));
                        }

                        break
                    }
                    _ => {}
                }

                i += 1;
            }

            if let Zero = group.ctx.hot {
                group.ctx.set_next_hot(FiveCardDrawHand(0));
            }

            const POT_BASE_X: unscaled::X = unscaled::X(150);
            const POT_BASE_Y: unscaled::Y = unscaled::Y(225);

            {
                let mut y = POT_BASE_Y;
                for amount in pot.individual_pots(&state.table.seats.moneys) {
                    stack_money_text!(main_pot_text = amount);

                    group.commands.print_chars(
                        &main_pot_text,
                        POT_BASE_X - pre_nul_len(&main_pot_text) * gfx::CHAR_ADVANCE,
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
            if $bundle.selection.bet > state.table.seats.moneys[current_i] {
                $bundle.selection.bet = state.table.seats.moneys[current_i]
                    .as_inner();
            }

            let action_opt = match (
                pot.has_folded(current),
                &state.table.seats.personalities[current_i]
            ) {
                (true, _) => Some(Action::Fold),
                (false, Some(_personality)) => {
                    // TODO Base choice of action off of personality

                    let hand = hands.get(current_i)
                                .map(|&h| h)
                                .unwrap_or_default();

                    let probability = hand_win_probability(hand);
                    let mut action = if probability >= SEVENTY_FIVE_PERCENT {
                        let multiple = MoneyInner::from(xs::range(rng, 6..12));
                        Action::Raise(
                            minimum_raise_total 
                            + state.table.seats.ante.get().saturating_mul(multiple)
                        )
                    } else if probability >= FIFTY_PERCENT {
                        if xs::range(rng, 0..5) == 0 {
                            // Don't be perfectly predictable!
                            gen_action(
                                rng,
                                ActionSpec {
                                    one_past_max_money: NonZeroMoneyInner::MIN
                                        .saturating_add(
                                            state.table.seats.moneys[current_i]
                                            .as_inner()
                                        ),
                                    min_money_unit: MIN_MONEY_UNIT,
                                    minimum_raise_total,
                                }
                            )
                        } else {
                            Action::Call
                        }
                    } else {
                        Action::Fold
                    };

                    match action {
                        Action::Fold => {
                            if call_remainder == 0 {
                                action = Action::Call;
                            }
                        },
                        Action::Call => {},
                        Action::Raise(raise_amount) => {
                            let inner = state.table.seats.moneys[current_i]
                                .as_inner();
                            if inner
                                .checked_sub(raise_amount)
                                .is_none() {
                                action = Action::Raise(inner);
                            }
                        },
                    }

                    Some(action)
                },
                (false, None) => {
                    match group.ctx.hot {
                        FiveCardDrawMenu(menu_id) => {
                            stack_money_text!(money_text = state.table.seats.moneys[current_i]);

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
                                let x = MENU_RECT.x + SPACING_W * 10;
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
                                        FiveCardDrawMenu(ACTION_KIND),
                                    );
                                } else {
                                    group.ctx.set_next_hot(FiveCardDrawMenu(SUBMIT));
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
                                            FiveCardDrawMenu(MONEY_AMOUNT),
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
                                        id: FiveCardDrawMenu(SUBMIT),
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
                                group.ctx.set_next_hot(FiveCardDrawHand(current));
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

                                group.ctx.set_next_hot(FiveCardDrawMenu(new_id));
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

                                group.ctx.set_next_hot(FiveCardDrawMenu(new_id));
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
                                            $bundle.selection.bet = $bundle.selection.bet.saturating_add(MIN_MONEY_UNIT.get());
                                        } else if group.input.pressed_this_frame(Button::DOWN) {
                                            $bundle.selection.bet = $bundle.selection.bet.saturating_sub(MIN_MONEY_UNIT.get());
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

            if let Some(action) = action_opt {
                let bet = match action {
                    Action::Fold => PotAction::Fold,
                    Action::Call => {
                        match call_leftover {
                            Some(new_amount) => {
                                PotAction::Bet(
                                    state.table.seats.moneys[current_i]
                                        .take_all_but(new_amount)
                                )
                            },
                            None => {
                                PotAction::Bet(
                                    state.table.seats.moneys[current_i].take_all()
                                )
                            }
                        }
                    },
                    Action::Raise(raise_amount) => {
                        // The total bet needed to call
                        let call_amount = pot.call_amount();

                        // The amount extra needed to call
                        let call_remainder = call_amount.saturating_sub(
                            pot.amount_for(current)
                        );
                        // The amount that would be leftover if the player was to call
                        let call_leftover = state.table.seats.moneys[current_i]
                            .as_inner()
                            .checked_sub(call_remainder);

                        match call_leftover {
                            Some(_) => {
                                match
                                    state.table.seats.moneys[current_i]
                                    .as_inner()
                                    .checked_sub(raise_amount)
                                {
                                    Some(new_amount) => {
                                        PotAction::Bet(
                                            state.table.seats.moneys[current_i]
                                                .take_all_but(new_amount)
                                        )
                                    },
                                    None => {
                                        debug_assert!(
                                            false,
                                            "player {} raised {} with only {}",
                                            $bundle.current,
                                            raise_amount,
                                            state.table.seats.moneys[current_i],
                                        );
                                        PotAction::Bet(
                                            state.table.seats.moneys[current_i]
                                            .take_all()
                                        )
                                    }
                                }
                            },
                            None => {
                                PotAction::Bet(
                                    state.table.seats.moneys[current_i]
                                    .take_all()
                                )
                            }
                        }
                    },
                };

                pot.push_bet($bundle.current, bet);

                $bundle.current += 1;
                if $bundle.current >= player_count.u8() {
                    $bundle.current = 0;
                }

                pot.round_outcome(&state.table.seats.moneys)
            } else {
                RoundOutcome::Undetermined
            }
        })
    }

    macro_rules! next_bundle {
        ($bundle: ident =
            $deck: expr,
            $current: expr,
            $player_count: expr,
            $pot: expr
        ) => {
            let mut deck = $deck;
            let previous_index = $current;
            let player_count = $player_count;

            let current = {
                let mut index = previous_index + 1;
                while {
                    if index >= player_count.u8() {
                        index = 0;
                    }

                    index != previous_index
                    && state.table.seats.moneys[usize::from(index)] == 0
                } {
                    index += 1;
                }

                index
            };

            let (posts, deck) = if let (Some(card1), Some(card2)) = (deck.draw(), deck.draw()) {
                ([card1, card2], deck)
            } else {
                deal(rng)
            };

            let mut $bundle = StateBundle {
                deck,
                posts,
                current,
                pot: $pot.take_all(),
                player_count,
                selection: MenuSelection::default(),
                round: Round::AfterOne,
            };

            let pot_has_all_the_money: bool = 
                state.table.seats.moneys.iter()
                    .all(|m| m.as_inner() == 0);

            if pot_has_all_the_money {
                // This is traditionally played against "the house", so there all the
                // money collecting there is a feature, not a bug.
                // Maybe we'll make that matter later, but for now it's just a 
                // disappointing outcome, so split up the money in case this is 
                // dealer's choice, and go back.
                $bundle.pot.split_among(
                    &mut state.table.seats.moneys[..],
                    usize::from(previous_index)
                );

                cmd = ModeCmd::BackToTitleScreen;
            }
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
                speaker.request_sfx(SFX::CardPlace);

                let player_count = *player_count;

                let mut moneys = [0; MAX_PLAYERS as usize];
                for i in 0..player_count.usize() {
                    moneys[i] = *starting_money;
                }
                let moneys = Money::array_from_inner_array(moneys);

                *state.table = Table::selected(
                    rng,
                    player_count,
                    moneys,
                );
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
                                *starting_money = starting_money.saturating_add(MIN_MONEY_UNIT.get());
                            },
                            Some(Dir::Down) => {
                                *starting_money = starting_money.saturating_sub(MIN_MONEY_UNIT.get());
                                if *starting_money == 0 {
                                    *starting_money = MIN_MONEY_UNIT.get();
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
        FirstRound {
            bundle,
        } => {
            let group = new_group!();
            let outcome = do_five_card_draw!(group, bundle);

            match outcome {
                RoundOutcome::Undetermined => {},
                RoundOutcome::AdvanceToNext => {
                    // TODO why does this happen before the player can interact?
                    todo!("AdvanceToNext from FirstRound");
                },
                RoundOutcome::AwardNow(_) => {
                    todo!("AwardNow(_) from FirstRound");
                },
            }
        }
        Drawing {
            bundle,
        } => {
            let group = new_group!();
            let outcome = do_five_card_draw!(group, bundle);

            match outcome {
                RoundOutcome::Undetermined => {},
                RoundOutcome::AdvanceToNext => {
                    todo!("AdvanceToNext from Drawing");
                },
                RoundOutcome::AwardNow(_) => {
                    todo!("AwardNow(_) from Drawing");
                },
            }
        }
        SecondRound {
            bundle,
        } => {
            let group = new_group!();
            let outcome = do_five_card_draw!(group, bundle);

            match outcome {
                RoundOutcome::Undetermined => {},
                RoundOutcome::AdvanceToNext => {
                    todo!("AdvanceToNext from SecondRound");
                },
                RoundOutcome::AwardNow(_) => {
                    todo!("AwardNow(_) from SecondRound");
                },
            }
        }
        Showdown {
            bundle,
        } => {
            let group = new_group!();
            let outcome = do_five_card_draw!(group, bundle);

            match outcome {
                RoundOutcome::Undetermined => {},
                RoundOutcome::AdvanceToNext => {
                    todo!("AdvanceToNext from Showdown");
                },
                RoundOutcome::AwardNow(_) => {
                    todo!("AwardNow(_) from Showdown");
                },
            }
        }
    }

    cmd
}