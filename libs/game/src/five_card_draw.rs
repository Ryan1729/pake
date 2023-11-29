use gfx::{card, pre_nul_len, Commands, SPACING_W, SPACING_H};
use models::{Card, CardBitset, ALL_CARDS, INITIAL_ANTE_AMOUNT, MIN_MONEY_UNIT, Deck, Money, MoneyInner, MoneyMove, NonZeroMoney, NonZeroMoneyInner, Rank, gen_deck, get_rank, ranks};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};
use probability::{EvalCount};

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
            _ => Err("player count was too big"),
        }
    }
}

pub fn gen_hand_index(rng: &mut Xs, player_count: PlayerCount) -> HandIndex {
    xs::range(rng, 0..player_count.u8() as _) as HandIndex
}

#[derive(Clone, Default)]
pub struct Seats {
    pub moneys: [Money; MAX_PLAYERS as usize],
    pub personalities: [Personality; MAX_PLAYERS as usize],
    pub skip: SkipState,
}

type Pot = Money;

#[derive(Clone, Debug, Default)]
pub enum Action {
    #[default]
    Pass,
    Bet(NonZeroMoneyInner),
    Burn(NonZeroMoneyInner),
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ActionKind {
    #[default]
    Pass,
    Bet,
    // We don't currently have a case where we need Burn here, so we leave it out.
}

impl ActionKind {
    pub fn text(self) -> &'static [u8] {
        use ActionKind::*;
        match self {
            Pass => b"pass",
            Bet => b"bet",
        }
    }

    pub fn next_up(self) -> Self {
        use ActionKind::*;
        match self {
            Pass => Bet,
            Bet => Pass,
        }
    }

    pub fn next_down(self) -> Self {
        use ActionKind::*;
        match self {
            Pass => Bet,
            Bet => Pass,
        }
    }
}

#[derive(Clone)]
pub struct MenuSelection {
    pub action_kind: ActionKind,
    pub bet: NonZeroMoneyInner,
}

impl Default for MenuSelection {
    fn default() -> Self {
        Self {
            action_kind: ActionKind::default(),
            bet: MIN_MONEY_UNIT,
        }
    }
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
        let mut pot: Pot = Money::ZERO;

        for i in 0..player_count.usize() {
            MoneyMove {
                from: &mut moneys[i],
                to: &mut pot,
                amount: INITIAL_ANTE_AMOUNT,
            }.perform();
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

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum RoundOutcome {
        Undetermined,
        AdvanceToNext,
    }

    macro_rules! do_five_card_draw {
        ($group: ident $(,)? $bundle: ident) => ({
            let group = $group;
            let hands = &$bundle.hands;
            let current = $bundle.current;
            let current_i = usize::from(current);
            let player_count = $bundle.player_count;

            use platform_types::unscaled::xy;
            // TODO Avoid overlapping hands
            let mut coords: [unscaled::XY; MAX_PLAYERS as usize] = [
                xy!(0 0) ; MAX_PLAYERS as usize
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
                        if i >= MAX_PLAYERS {
                            break 'outer;
                        }
                    }
                }
            }

            let hands_len: HandsLen = player_count.u8();

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

            RoundOutcome::Undetermined
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
                    todo!("AdvanceToNext from FirstRound");
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
            }
        }
    }

    cmd
}