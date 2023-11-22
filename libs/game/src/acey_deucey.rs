use gfx::{card, pre_nul_len, Commands, SPACING_W, SPACING_H};
use models::{Card, CardBitset, ALL_CARDS, Deck, Money, MoneyInner, MoneyMove, NonZeroMoney, NonZeroMoneyInner, Rank, gen_deck, get_rank, ranks};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};
use probability::{EvalCount};

use std::io::Write;

use xs::Xs;

use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState, INITIAL_ANTE_AMOUNT, MIN_MONEY_UNIT};
use crate::ui::{self, draw_money_in_rect, stack_money_text, ButtonSpec, Id::*, do_button};

type Posts = [Card; 2];

/// Extended with a slot for high aces.
type HighLowRank = Rank;

fn get_ranks(posts: Posts, first_post: HighLow) -> [HighLowRank; 2] {
    let mut first_rank = get_rank(posts[0]);
    if first_rank == ranks::ACE {
        match first_post {
            HighLow::High => {
                first_rank = ranks::HIGH_ACE;
            }
            HighLow::Low => {}
        }
    }

    let mut second_rank = get_rank(posts[1]);
    // Second one is automatically high.
    if second_rank == ranks::ACE {
        second_rank = ranks::HIGH_ACE;
    }

    [
        first_rank,
        second_rank,
    ]
}

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
pub const MIN_PLAYERS: u8 = 2;
pub const MAX_PLAYERS: u8 = 17;

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
            10 => Ok(Ten),
            11 => Ok(Eleven),
            12 => Ok(Twelve),
            13 => Ok(Thirteen),
            14 => Ok(Fourteen),
            15 => Ok(Fifteen),
            16 => Ok(Sixteen),
            17 => Ok(Seventeen),
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

const CONNECTORS_AMOUNT: NonZeroMoneyInner = MIN_MONEY_UNIT;
const CONNECTORS_BURN: Action = Action::Burn(CONNECTORS_AMOUNT);
const PAIR_AMOUNT: NonZeroMoneyInner = MIN_MONEY_UNIT.saturating_add(MIN_MONEY_UNIT.get());
const PAIR_BURN: Action = Action::Burn(PAIR_AMOUNT);

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

#[derive(Clone, Copy, Default)]
pub enum HighLow {
    #[default]
    Low,
    High,
}

impl HighLow {
    fn next_up(self) -> HighLow {
        use HighLow::*;
        match self {
            High => Low,
            Low => High,
        }
    }

    fn next_down(self) -> HighLow {
        use HighLow::*;
        match self {
            High => Low,
            Low => High,
        }
    }

    fn text(self) -> &'static [u8] {
        use HighLow::*;
        match self {
            High => b"high",
            Low => b"low",
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum Ace {
    #[default]
    Undecided,
    Decided(HighLow),
}

#[derive(Clone)]
pub struct MenuSelection {
    pub action_kind: ActionKind,
    pub bet: NonZeroMoneyInner,
    pub ace: Ace,
    pub temp_high_low: HighLow,
    pub cpu_passed: bool,
}

impl Default for MenuSelection {
    fn default() -> Self {
        Self {
            action_kind: ActionKind::default(),
            bet: MIN_MONEY_UNIT,
            ace: Ace::default(),
            temp_high_low: <_>::default(),
            cpu_passed: <_>::default(),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum Round {
    #[default]
    One,
    AfterOne,
}

#[derive(Clone)]
pub struct StateBundle {
    pub deck: Deck,
    pub posts: Posts,
    pub current: HandIndex,
    pub pot: Pot,
    pub player_count: PlayerCount,
    pub selection: MenuSelection,
    pub round: Round,
}

#[derive(Clone)]
pub enum TableState {
    Undealt { player_count: PlayerCount, starting_money: MoneyInner },
    DealtPosts {
        bundle: StateBundle,
    },
    Reveal {
        bundle: StateBundle,
        third: Card,
        bet: NonZeroMoneyInner,
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

        let mut personalities: [Personality; MAX_PLAYERS as usize] = <_>::default();//;

        personalities[0] = None;
        // TODO Make each element of this array user selectable too.
        // Start at 1 to make the first player user controlled
        for i in 1..player_count.usize() {
            personalities[i] = Some(CpuPersonality{});
        }

        let (posts, deck) = deal(rng);

        let current = gen_hand_index(rng, player_count);

        Self {
            seats: Seats {
                moneys,
                personalities,
                skip: <_>::default(),
            },
            state: TableState::DealtPosts {
                bundle: StateBundle {
                    deck,
                    posts,
                    current,
                    pot,
                    player_count,
                    selection: <_>::default(),
                    round: <_>::default(),
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

    macro_rules! do_acey_deucey {
        ($group: ident $(,)? $bundle: ident , $third_opt: expr) => {
            let player_count = $bundle.player_count;
            let posts = $bundle.posts;
            let current = $bundle.current;

            for i in 0..player_count.u8() {
                use unscaled::Inner;

                let money = state.table.seats.moneys[i as usize].as_inner();

                let w = unscaled::W(25);
                let h = unscaled::H(15);

                let money_rect = unscaled::Rect {
                    x: unscaled::X(0) + SPACING_W,
                    y: unscaled::Y(0) + SPACING_H + h * Inner::from(i),
                    w,
                    h,
                };

                if let None = state.table.seats.personalities[i as usize] {
                    $group.commands.draw_nine_slice(
                        gfx::NineSlice::Highlight,
                        money_rect,
                    );
                }

                if i == current {
                    $group.commands.draw_selected(
                        money_rect.x + money_rect.w / 2,
                        money_rect.y,
                    );
                }

                draw_money_in_rect!($group, money, money_rect);
            }

            let w = unscaled::W(50);
            let h = unscaled::H(20);

            let pot_rect = unscaled::Rect {
                x: unscaled::X(0) + command::MID_W - (w/2),
                y: unscaled::Y(0) + SPACING_H,
                w,
                h,
            };

            draw_money_in_rect!($group, $bundle.pot, pot_rect);

            let CARD_1_X: unscaled::X =
                // Need an extra `card::WIDTH` because the sprite is drawn from the
                // top left corner
                command::MID_X - (card::WIDTH * 2 + card::WIDTH / 2);
            let CARD_1_Y: unscaled::Y = command::MID_Y;

            $group.commands.draw_card(
                posts[0],
                CARD_1_X,
                CARD_1_Y,
            );

            let CARD_2_X: unscaled::X = command::MID_X + (card::WIDTH + card::WIDTH / 2);
            let CARD_2_Y: unscaled::Y = command::MID_Y;

            match (get_rank(posts[0]) == ranks::ACE, $bundle.selection.ace) {
                (true, Ace::Undecided) => {
                    $group.commands.draw_card_back(
                        CARD_2_X,
                        CARD_2_Y,
                    );
                }
                (false, _)
                | (true, Ace::Decided(_)) => {
                    $group.commands.draw_card(
                        posts[1],
                        CARD_2_X,
                        CARD_2_Y,
                    );
                }
            }

            if let Some(third) = $third_opt {
                $group.commands.draw_card(
                    third,
                    command::MID_X - (card::WIDTH / 2),
                    command::MID_Y,
                );
            }
        }
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

            let $bundle = StateBundle {
                deck,
                posts,
                current,
                pot: $pot.take_all(),
                player_count,
                selection: MenuSelection::default(),
                round: Round::AfterOne,
            };
        }
    }

    #[derive(PartialEq, Eq)]
    enum Outcome {
        Loss,
        Win,
    }
    use Outcome::*;
    
    fn calc_outcome(
        posts: Posts,
        third: Card,
        ace: Ace,
    ) -> Outcome {
        use Outcome::*;
        let ranks = get_ranks(
            posts,
            match ace {
                Ace::Undecided => HighLow::default(),
                Ace::Decided(high_low) => high_low,
            }
        );
        let min_rank = core::cmp::min(ranks[0], ranks[1]);
        let max_rank = core::cmp::max(ranks[0], ranks[1]);
        let third_rank = get_rank(third);
        if min_rank < third_rank
        && third_rank < max_rank {
            Win
        } else {
            Loss
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
        DealtPosts { bundle, } => {
            let group = new_group!();

            do_acey_deucey!(
                group,
                bundle,
                None
            );

            let current_i = usize::from(bundle.current);

            const MENU_UI_BASE_X: unscaled::X = unscaled::x_const_add_w(MENU_RECT.x, unscaled::w_const_mul(SPACING_W, 10));
            const MENU_UI_BASE_Y: unscaled::Y = unscaled::y_const_add_h(MENU_RECT.y, SPACING_H);

            macro_rules! draw_menu_rect_with_money {
                () => {
                    group.commands.draw_nine_slice(
                        gfx::NineSlice::Button,
                        MENU_RECT
                    );

                    {
                        stack_money_text!(money_text = state.table.seats.moneys[current_i]);

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
                }
            }

            if bundle.selection.cpu_passed {
                draw_menu_rect_with_money!();

                let ack_button_rect = unscaled::Rect {
                    x: MENU_UI_BASE_X,
                    y: MENU_UI_BASE_Y,
                    w: unscaled::W(64),
                    h: MENU_RECT.h - SPACING_H * 2,
                };

                if do_button(
                    group,
                    ButtonSpec {
                        id: AcknowledgeCPUPass,
                        rect: ack_button_rect,
                        text: b"acknowledged",
                    }
                ) {
                    next_bundle!(
                        new_bundle =
                            bundle.deck.clone(),
                            bundle.current,
                            bundle.player_count,
                            bundle.pot
                    );

                    state.table.state = DealtPosts {
                        bundle: new_bundle,
                    };
                }

                let ack_button_rect_far_x =
                    ack_button_rect.x + ack_button_rect.w;

                let top_line_rect = unscaled::Rect {
                    x: ack_button_rect_far_x + SPACING_W,
                    w: MENU_RECT.w - (
                        ack_button_rect_far_x - MENU_RECT.x
                    ),
                    ..ack_button_rect
                };

                {
                    let mut passed_text = [0 as u8; 32];

                    let _cant_actually_fail = write!(
                        &mut passed_text[..],
                        "player {current_i} passed",
                    );

                    let xy = gfx::center_line_in_rect(
                        pre_nul_len(&passed_text),
                        top_line_rect,
                    );

                    group.commands.print_chars(
                        &passed_text,
                        xy.x,
                        xy.y + gfx::CHAR_H,
                        TEXT
                    );
                }

                if let Zero = group.ctx.hot {
                    group.ctx.set_next_hot(AcknowledgeCPUPass);
                }
            } else {
                match {
                    if get_rank(bundle.posts[0]) == ranks::ACE {
                        match bundle.selection.ace {
                            Ace::Undecided => None,
                            Ace::Decided(high_low) => Some(high_low),
                        }
                    } else {
                        Some(HighLow::default())
                    }
                } {
                    None => {
                        match
                            &state.table.seats.personalities[
                                current_i
                            ]
                        {
                            // TODO? Have the CPU player count cards enough to
                            // know to choose high sometimes?
                            Some(_) => {
                                bundle.selection.ace = Ace::Decided(HighLow::Low);
                            },
                            None => {
                                draw_menu_rect_with_money!();
    
                                let high_low_rect = unscaled::Rect {
                                    x: MENU_UI_BASE_X,
                                    y: MENU_UI_BASE_Y,
                                    w: unscaled::W(50),
                                    h: MENU_RECT.h - SPACING_H * 2,
                                };
    
                                let high_low_text = bundle.selection.temp_high_low.text();
    
                                {
                                    let xy = gfx::center_line_in_rect(
                                        high_low_text.len() as _,
                                        high_low_rect,
                                    );
                                    group.commands.print_chars(
                                        high_low_text,
                                        xy.x,
                                        xy.y,
                                        TEXT
                                    );
                                }
    
                                ui::draw_quick_select(
                                    group,
                                    high_low_rect,
                                    HighLowSelect,
                                );
    
                                if do_button(
                                    group,
                                    ButtonSpec {
                                        id: HighLowSubmit,
                                        rect: unscaled::Rect {
                                            x: high_low_rect.x + high_low_rect.w + high_low_rect.w,
                                            ..high_low_rect
                                        },
                                        text: b"submit",
                                    }
                                ) {
                                    bundle.selection.ace = Ace::Decided(bundle.selection.temp_high_low);
                                }
    
                                if group.input.pressed_this_frame(Button::LEFT)
                                || group.input.pressed_this_frame(Button::RIGHT) {
                                    match group.ctx.hot {
                                        HighLowSubmit => {
                                            group.ctx.set_next_hot(HighLowSelect);
                                        }
                                        HighLowSelect => {
                                            group.ctx.set_next_hot(HighLowSubmit);
                                        }
                                        _ => {}
                                    }
                                } else {
                                    match group.ctx.hot {
                                        HighLowSelect => {
                                            if group.input.pressed_this_frame(Button::UP) {
                                                bundle.selection.temp_high_low = bundle.selection.temp_high_low.next_up();
                                            } else if group.input.pressed_this_frame(Button::DOWN) {
                                                bundle.selection.temp_high_low = bundle.selection.temp_high_low.next_down();
                                            }
                                        }
                                        _ => {}
                                    }
                                }
    
                                if let Zero = group.ctx.hot {
                                    group.ctx.set_next_hot(HighLowSelect);
                                }
                            }
                        }
                    },
                    Some(high_low) => {
                        enum PostsKind {
                            Open,
                            Connectors,
                            Pair,
                        }
                        use PostsKind::*;
                        let posts_kind = {
                            let ranks = get_ranks(
                                bundle.posts,
                                high_low
                            );
    
                            if ranks[0] == ranks[1] {
                                Pair
                            } else
                            if ranks[0] == ranks[1] + 1
                            || ranks[1] == ranks[0] + 1 {
                                Connectors
                            } else {
                                Open
                            }
                        };
    
                        macro_rules! do_burn_menu {
                            ($amount: ident) => ({
                                draw_menu_rect_with_money!();
    
                                let burn_button_rect = unscaled::Rect {
                                    x: MENU_UI_BASE_X,
                                    y: MENU_UI_BASE_Y,
                                    w: unscaled::W(50),
                                    h: MENU_RECT.h - SPACING_H * 2,
                                };
    
                                let player_action_opt = if do_button(
                                    group,
                                    ButtonSpec {
                                        id: AcceptBurn,
                                        rect: burn_button_rect,
                                        text: b"get burned",
                                    }
                                ) {
                                    Some(Action::Burn($amount))
                                } else {
                                    None
                                };
    
                                let burn_button_rect_far_x =
                                    burn_button_rect.x + burn_button_rect.w;
    
                                let top_line_rect = unscaled::Rect {
                                    x: burn_button_rect_far_x + SPACING_W,
                                    w: MENU_RECT.w - (
                                        burn_button_rect_far_x - MENU_RECT.x
                                    ),
                                    h: burn_button_rect.h / 3,
                                    ..burn_button_rect
                                };
    
                                {
                                    let description_line = b"you cannot bet and will instead will be burned for";
    
                                    let xy = gfx::center_line_in_rect(
                                        description_line.len() as _,
                                        top_line_rect,
                                    );
    
                                    group.commands.print_chars(
                                        description_line,
                                        xy.x,
                                        xy.y + gfx::CHAR_H,
                                        TEXT
                                    );
                                }
                                {
                                    let bottom_line_rect = unscaled::Rect {
                                        y: top_line_rect.y + top_line_rect.h,
                                        ..top_line_rect
                                    };
    
                                    stack_money_text!(money_text = $amount);
    
                                    let description_line = &money_text;
    
                                    let xy = gfx::center_line_in_rect(
                                        pre_nul_len(description_line),
                                        bottom_line_rect,
                                    );
    
                                    group.commands.print_chars(
                                        description_line,
                                        xy.x,
                                        xy.y + gfx::CHAR_H,
                                        TEXT
                                    );
                                }
    
                                if let Zero = group.ctx.hot {
                                    group.ctx.set_next_hot(AcceptBurn);
                                }
    
                                player_action_opt
                            })
                        }
    
                        let action_opt = match
                            (
                                &state.table.seats.personalities[current_i],
                                posts_kind
                            )
                        {
                            (Some(_), Open) => {
                                let mut remaining_cards = CardBitset::full();
    
                                // TODO? Count cards so we can remove more?
                                remaining_cards.remove(bundle.posts[0]);
                                remaining_cards.remove(bundle.posts[1]);
    
                                let mut eval_count = EvalCount {
                                    total: 0,
                                    win_count: 0,
                                };
                                for card in remaining_cards.iter() {
                                    eval_count.total += 1;
    
                                    if calc_outcome(bundle.posts, card, bundle.selection.ace)
                                    == Outcome::Win {
                                        eval_count.win_count += 1;
                                    }
                                }
                                
                                if eval_count.probability() >= probability::FIFTY_PERCENT {
                                    Some(Action::Bet(INITIAL_ANTE_AMOUNT))
                                } else {
                                    Some(Action::Pass)
                                }
                            }
                            (Some(_), Connectors) => Some(CONNECTORS_BURN),
                            (Some(_), Pair) => Some(PAIR_BURN),
                            (None, Open) => {
                                const ACTION_KIND: ui::AceyDeuceyMenuId = 0;
                                const MONEY_AMOUNT: ui::AceyDeuceyMenuId = 1;
                                const SUBMIT: ui::AceyDeuceyMenuId = 2;
                                const MENU_KIND_ONE_PAST_MAX: ui::AceyDeuceyMenuId = 3;
    
                                draw_menu_rect_with_money!();
    
                                let player_action_opt = match group.ctx.hot {
                                    AceyDeuceyMenu(menu_id) => {
                                        let action_kind_rect = unscaled::Rect {
                                            x: MENU_UI_BASE_X,
                                            y: MENU_UI_BASE_Y,
                                            w: unscaled::W(50),
                                            h: MENU_RECT.h - SPACING_H * 2,
                                        };
    
                                        let action_kind_text = bundle.selection.action_kind.text();
    
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
    
                                        ui::draw_quick_select(
                                            group,
                                            action_kind_rect,
                                            AceyDeuceyMenu(ACTION_KIND),
                                        );
    
                                        let money_rect = unscaled::Rect {
                                            x: action_kind_rect.x + action_kind_rect.w,
                                            ..action_kind_rect
                                        };
    
                                        match bundle.selection.action_kind {
                                            ActionKind::Bet => {
                                                draw_money_in_rect!(group, bundle.selection.bet, money_rect);
    
                                                ui::draw_quick_select(
                                                    group,
                                                    money_rect,
                                                    AceyDeuceyMenu(MONEY_AMOUNT),
                                                );
                                            }
                                            ActionKind::Pass => {}
                                        }
    
                                        let player_action_opt = if do_button(
                                            group,
                                            ButtonSpec {
                                                id: AceyDeuceyMenu(SUBMIT),
                                                rect: unscaled::Rect {
                                                    x: action_kind_rect.x + action_kind_rect.w + action_kind_rect.w,
                                                    ..action_kind_rect
                                                },
                                                text: b"submit",
                                            }
                                        ) {
                                            Some(match bundle.selection.action_kind {
                                                ActionKind::Pass => Action::Pass,
                                                ActionKind::Bet => Action::Bet(bundle.selection.bet),
                                            })
                                        } else {
                                            None
                                        };
    
                                        if group.input.pressed_this_frame(Button::LEFT) {
                                            let mut new_id = menu_id;
                                            new_id = match new_id.checked_sub(1) {
                                                Some(new_id) => new_id,
                                                None => MENU_KIND_ONE_PAST_MAX - 1,
                                            };
    
                                            if new_id == MONEY_AMOUNT
                                            && bundle.selection.action_kind != ActionKind::Bet {
                                                new_id = match new_id.checked_sub(1) {
                                                    Some(new_id) => new_id,
                                                    None => MENU_KIND_ONE_PAST_MAX - 1,
                                                };
                                            }
    
                                            group.ctx.set_next_hot(AceyDeuceyMenu(new_id));
                                        } else if group.input.pressed_this_frame(Button::RIGHT) {
                                            let mut new_id = menu_id;
                                            new_id += 1;
                                            if new_id >= MENU_KIND_ONE_PAST_MAX {
                                                new_id = 0;
                                            }
    
                                            if new_id == MONEY_AMOUNT
                                            && bundle.selection.action_kind != ActionKind::Bet {
                                                new_id += 1;
                                                if new_id >= MENU_KIND_ONE_PAST_MAX {
                                                    new_id = 0;
                                                }
                                            }
    
                                            group.ctx.set_next_hot(AceyDeuceyMenu(new_id));
                                        } else {
                                            match menu_id {
                                                ACTION_KIND => {
                                                    if group.input.pressed_this_frame(Button::UP) {
                                                        bundle.selection.action_kind = bundle.selection.action_kind.next_up();
                                                    } else if group.input.pressed_this_frame(Button::DOWN) {
                                                        bundle.selection.action_kind = bundle.selection.action_kind.next_down();
                                                    }
                                                }
                                                MONEY_AMOUNT => {
                                                    if group.input.pressed_this_frame(Button::UP) {
                                                        bundle.selection.bet = bundle.selection.bet
                                                            .saturating_add(MIN_MONEY_UNIT.get());
                                                    } else if group.input.pressed_this_frame(Button::DOWN) {
                                                        let new_value = bundle.selection.bet.get()
                                                            .saturating_sub(MIN_MONEY_UNIT.get());
                                                        if let Some(new_bet) = NonZeroMoneyInner::new(new_value) {
                                                            bundle.selection.bet = new_bet;
                                                        }
                                                    }
                                                }
                                                _ => {}
                                            }
                                        }
    
                                        player_action_opt
                                    }
                                    _ => None,
                                };
    
                                if let Zero = group.ctx.hot {
                                    group.ctx.set_next_hot(AceyDeuceyMenu(ACTION_KIND));
                                }
    
                                player_action_opt
                            }
                            (None, Connectors) => {
                                do_burn_menu!(CONNECTORS_AMOUNT)
                            }
                            (None, Pair) => {
                                do_burn_menu!(PAIR_AMOUNT)
                            }
                        };
    
                        // You can't bet more than you have
                        if bundle.selection.bet.get() > state.table.seats.moneys[current_i] {
                            if let Some(new_bet) = NonZeroMoneyInner::new(
                                state.table.seats.moneys[current_i].as_inner()
                            ) {
                                bundle.selection.bet = new_bet;
                            }
                        }
    
                        let pot_limit = match bundle.round {
                            Round::One => bundle.pot.as_inner() / 2,
                            Round::AfterOne => bundle.pot.as_inner(),
                        };
    
                        // You can't bet more than the pot limit
                        if bundle.selection.bet.get() > pot_limit {
                            if let Some(new_bet) = NonZeroMoneyInner::new(
                                pot_limit
                            ) {
                                bundle.selection.bet = new_bet;
                            }
                        }
    
                        match action_opt {
                            Some(Action::Pass) => {
                                match &state.table.seats.personalities[current_i] {
                                    Some(_) => {
                                        bundle.selection.cpu_passed = true;
                                    }
                                    None => {
                                        next_bundle!(
                                            new_bundle =
                                                bundle.deck.clone(),
                                                bundle.current,
                                                bundle.player_count,
                                                bundle.pot
                                        );
            
                                        state.table.state = DealtPosts {
                                            bundle: new_bundle,
                                        };
                                    }
                                }   
                            }
                            Some(Action::Bet(bet)) => {
                                let third = loop {
                                    if let Some(third) = bundle.deck.draw() {
                                        break third;
                                    } else {
                                        bundle.deck = gen_deck(rng);
                                    }
                                };
    
                                state.table.state = Reveal {
                                    bundle: bundle.clone(),
                                    third,
                                    bet,
                                };
                            }
                            Some(Action::Burn(amount)) => {
                                MoneyMove {
                                    from: &mut state.table.seats.moneys[current_i],
                                    to: &mut bundle.pot,
                                    amount,
                                }.perform();

                                next_bundle!(
                                    new_bundle =
                                        bundle.deck.clone(),
                                        bundle.current,
                                        bundle.player_count,
                                        bundle.pot
                                );
    
                                state.table.state = DealtPosts {
                                    bundle: new_bundle,
                                };
                            }
                            None => {}
                        }
                    },
                }
            }
        },
        Reveal { bundle, third, bet } => {
            let group = new_group!();

            do_acey_deucey!(
                group,
                bundle,
                Some(*third)
            );

            group.commands.draw_nine_slice(
                gfx::NineSlice::Button,
                MENU_RECT
            );

            let outcome = calc_outcome(bundle.posts, *third, bundle.selection.ace);

            {
                let mut outcome_text = [0u8; 20];
                use std::io::Write;

                match outcome {
                    Loss => {
                        let _cant_actually_fail = write!(
                            &mut outcome_text[..],
                            "player {} lost!",
                            bundle.current
                        );
                    }
                    Win => {
                        let _cant_actually_fail = write!(
                            &mut outcome_text[..],
                            "player {} won!",
                            bundle.current
                        );
                    }
                }

                let x = MENU_RECT.x + SPACING_W;
                let mut y = MENU_RECT.y + SPACING_H;
                group.commands.print_chars(
                    &outcome_text,
                    x,
                    y,
                    TEXT
                );
                y += gfx::CHAR_LINE_ADVANCE;
            }

            let x = MENU_RECT.x + SPACING_W * 10;
            let y = MENU_RECT.y + SPACING_H;

            let next_rect = unscaled::Rect {
                x,
                y,
                w: unscaled::W(50),
                h: MENU_RECT.h - SPACING_H * 2,
            };

            if do_button(
                group,
                ButtonSpec {
                    id: NextDeal,
                    rect: next_rect,
                    text: b"next",
                }
            ) {
                let current_i = usize::from(bundle.current);
                match outcome {
                    Loss => {
                        MoneyMove {
                            from: &mut state.table.seats.moneys[current_i],
                            to: &mut bundle.pot,
                            amount: *bet,
                        }.perform();
                    }
                    Win => {
                        MoneyMove {
                            from: &mut bundle.pot,
                            to: &mut state.table.seats.moneys[current_i],
                            amount: *bet,
                        }.perform();
                    }
                }

                // TODO handle case where the pot has all the money in it!
                if bundle.pot == 0 {
                    // TODO show a winner screen with more winner info.
                    if state.table.seats.personalities[0].is_none() {
                        println!("User wins!");
                    } else {
                        println!("Cpu player wins!");
                    }

                    group.speaker.request_sfx(SFX::CardPlace);
                    state.table.state = <_>::default();
                    cmd = ModeCmd::FinishedRound;
                } else {
                    next_bundle!(
                        new_bundle =
                            bundle.deck.clone(),
                            bundle.current,
                            bundle.player_count,
                            bundle.pot
                    );

                    state.table.state = DealtPosts {
                        bundle: new_bundle,
                    };
                }
            }

            if let Zero = group.ctx.hot {
                group.ctx.set_next_hot(NextDeal);
            }
        },
    }


    cmd
}