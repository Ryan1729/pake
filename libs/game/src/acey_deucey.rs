use gfx::{Commands};
use models::{Card, ALL_CARDS, Deck, Money, NonZeroMoney, gen_deck};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};

use xs::Xs;

use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState};
#[macro_use]
use crate::ui::{self, draw_money_in_rect, ButtonSpec, Id::*, do_button};

type Posts = [Card; 2];

fn deal(rng: &mut Xs) -> (Posts, Deck) {
    let mut deck = gen_deck(rng);

    let mut posts = Posts::default();

    let (Some(card1), Some(card2)) = (deck.draw(), deck.draw())
        else {
            debug_assert!(false, "Couldn't draw two from fresh deck?!");
            return (posts, deck);
        };

    posts = [card1, card2];

    (posts, deck)
}

/// In some sense any number of players could play, but we want some maximum.
/// Each turn up to 3 cards may be dealt, so if more than 17 players play, then the
/// deck will need to be reshuffled every single round. This seems as good a place
/// to cap things as anywhere.
const MAX_PLAYERS: u8 = 17;

pub type HandIndex = u8;
pub const MAX_HAND_INDEX: u8 = MAX_PLAYERS - 1;

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
    Ten,
    Eleven,
    Twelve,
    Thirteen,
    Fourteen,
    Fifteen,
    Sixteen,
    Seventeen,
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
            Nine => Ten,
            Ten => Eleven,
            Eleven => Twelve,
            Twelve => Thirteen,
            Thirteen => Fourteen,
            Fourteen => Fifteen,
            Fifteen => Sixteen,
            Sixteen
            | Seventeen => Seventeen,
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
            Ten => Nine,
            Eleven => Ten,
            Twelve => Eleven,
            Thirteen => Twelve,
            Fourteen => Thirteen,
            Fifteen => Fourteen,
            Sixteen => Fifteen,
            Seventeen => Sixteen,
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
            Ten => "10",
            Eleven => "11",
            Twelve => "12",
            Thirteen => "13",
            Fourteen => "14",
            Fifteen => "15",
            Sixteen => "16",
            Seventeen => "17",
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
            Ten => 10,
            Eleven => 11,
            Twelve => 12,
            Thirteen => 13,
            Fourteen => 14,
            Fifteen => 15,
            Sixteen => 16,
            Seventeen => 17,
        }
    }

    pub fn usize(self) -> usize {
        usize::from(self.u8())
    }
}

pub fn gen_hand_index(rng: &mut Xs, player_count: PlayerCount) -> HandIndex {
    xs::range(rng, 0..player_count.u8() as _) as HandIndex
}

#[derive(Clone, Default)]
pub struct Seats {
    moneys: [Money; MAX_PLAYERS as usize],
    personalities: [Personality; MAX_PLAYERS as usize],
    skip: SkipState,
}

type Pot = Money;

#[derive(Clone)]
pub struct StateBundle {
    pub deck: Deck,
    pub posts: Posts,
    pub current: HandIndex,
    pub pot: Pot,
    pub player_count: PlayerCount,
}

#[derive(Clone)]
pub enum TableState {
    Undealt { player_count: PlayerCount, starting_money: Money },
    DealtPosts {
        bundle: StateBundle,
    },
    Reveal {
        bundle: StateBundle,
        third: Card,
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

    let min_money_unit: NonZeroMoney = NonZeroMoney::MIN.saturating_add(5 - 1);
    let initial_ante_amount: NonZeroMoney = min_money_unit.saturating_mul(
        min_money_unit
    );

    let mut cmd = ModeCmd::NoOp;

    macro_rules! do_acey_deucey {
        ($group: ident $(,)? $bundle: ident , $third_opt: expr) => {
            let group = $group;

            let player_count = $bundle.player_count;
            for i in 0..player_count.u8() {
                use unscaled::Inner;

                let money = state.table.seats.moneys[i as usize];

                let money_rect = unscaled::Rect {
                    x: unscaled::X(150),
                    y: unscaled::Y(Inner::from(i) * 50),
                    w: unscaled::W(50),
                    h: unscaled::H(100),
                };

                draw_money_in_rect!(group, money, money_rect);
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
                let player_count = *player_count;
                let mut pot: Pot = 0;

                for i in 0..player_count.usize() {
                    state.table.seats.moneys[i] = starting_money
                        .saturating_sub(initial_ante_amount.get());

                    pot = pot.saturating_add(initial_ante_amount.get());
                }

                state.table.seats.personalities[0] = None;
                // TODO Make each element of this array user selectable too.
                // Start at 1 to make the first player user controlled
                for i in 1..player_count.usize() {
                    state.table.seats.personalities[i] = Some(CpuPersonality{});
                }

                let (posts, deck) = deal(rng);

                let current = gen_hand_index(rng, player_count);

                speaker.request_sfx(SFX::CardPlace);
                state.table.state = DealtPosts {
                    bundle: StateBundle {
                        deck,
                        posts,
                        current,
                        pot,
                        player_count
                    }
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
        DealtPosts { bundle, } => {
            let group = new_group!();

            do_acey_deucey!(
                group,
                bundle,
                None
            );
        },
        Reveal { bundle, third } => {
            let group = new_group!();

            do_acey_deucey!(
                group,
                bundle,
                Some(third)
            );
        },
    }


    cmd
}