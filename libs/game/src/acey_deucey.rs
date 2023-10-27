use gfx::{card, Commands, SPACING_W, SPACING_H};
use models::{Card, ALL_CARDS, Deck, Money, NonZeroMoney, gen_deck, get_rank};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};

use xs::Xs;

use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState};
use crate::ui::{self, draw_money_in_rect, stack_money_text, ButtonSpec, Id::*, do_button};

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

#[derive(Copy, Clone, Debug, Default)]
pub enum Action {
    #[default]
    Pass,
    Bet(NonZeroMoney)
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ActionKind {
    #[default]
    Pass,
    Bet,
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

const MIN_MONEY_UNIT: NonZeroMoney = NonZeroMoney::MIN.saturating_add(5 - 1);
const INITIAL_ANTE_AMOUNT: NonZeroMoney = MIN_MONEY_UNIT.saturating_mul(
    MIN_MONEY_UNIT
);

#[derive(Clone)]
pub struct MenuSelection {
    pub action_kind: ActionKind,
    pub bet: NonZeroMoney,
}

impl Default for MenuSelection {
    fn default() -> Self {
        Self {
            action_kind: ActionKind::default(),
            bet: MIN_MONEY_UNIT,
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
    Undealt { player_count: PlayerCount, starting_money: Money },
    DealtPosts {
        bundle: StateBundle,
    },
    Reveal {
        bundle: StateBundle,
        third: Card,
        bet: NonZeroMoney,
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

    let mut cmd = ModeCmd::NoOp;

    macro_rules! do_acey_deucey {
        ($group: ident $(,)? $bundle: ident , $third_opt: expr) => {
            let player_count = $bundle.player_count;
            let pot = $bundle.pot;
            let posts = $bundle.posts;
            let current = $bundle.current;

            for i in 0..player_count.u8() {
                use unscaled::Inner;

                let money = state.table.seats.moneys[i as usize];

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

            draw_money_in_rect!($group, pot, pot_rect);

            $group.commands.draw_card(
                posts[0],
                // Need an extra `card::WIDTH` because the sprite is drawn from the
                // top left corner
                command::MID_X - (card::WIDTH * 2 + card::WIDTH / 2),
                command::MID_Y,
            );

            $group.commands.draw_card(
                posts[1],
                command::MID_X + (card::WIDTH + card::WIDTH / 2),
                command::MID_Y,
            );

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
            let pot = $pot;

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
                pot,
                player_count,
                selection: MenuSelection::default(),
                round: Round::AfterOne,
            };
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
                let player_count = *player_count;
                let mut pot: Pot = 0;

                for i in 0..player_count.usize() {
                    state.table.seats.moneys[i] = starting_money
                        .saturating_sub(INITIAL_ANTE_AMOUNT.get());

                    pot = pot.saturating_add(INITIAL_ANTE_AMOUNT.get());
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
                        player_count,
                        selection: <_>::default(),
                        round: <_>::default(),
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

            // TODO handle connectors, pairs, and aces.

            let current_i = usize::from(bundle.current);

            let action_opt = match &state.table.seats.personalities[current_i] {
                Some(_) => {
                    // TODO have cpu player actually calculate the probabilty here
                    // and decide what to do based on that
                    Some(Action::Bet(INITIAL_ANTE_AMOUNT))
                }
                None => {
                    stack_money_text!(money_text = state.table.seats.moneys[current_i]);

                    const ACTION_KIND: ui::AceyDeuceyMenuId = 0;
                    const MONEY_AMOUNT: ui::AceyDeuceyMenuId = 1;
                    const SUBMIT: ui::AceyDeuceyMenuId = 2;
                    const MENU_KIND_ONE_PAST_MAX: ui::AceyDeuceyMenuId = 3;

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

                    let player_action_opt = match group.ctx.hot {
                        AceyDeuceyMenu(menu_id) => {
                            let x = MENU_RECT.x + SPACING_W * 10;
                            let y = MENU_RECT.y + SPACING_H;

                            let action_kind_rect = unscaled::Rect {
                                x,
                                y,
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
                                            bundle.selection.bet = bundle.selection.bet.saturating_add(MIN_MONEY_UNIT.get());
                                        } else if group.input.pressed_this_frame(Button::DOWN) {
                                            let new_value = bundle.selection.bet.get().saturating_sub(MIN_MONEY_UNIT.get());
                                            if let Some(new_bet) = NonZeroMoney::new(new_value) {
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
            };

            // You can't bet more than you have
            if bundle.selection.bet.get() > state.table.seats.moneys[current_i] {
                if let Some(new_bet) = NonZeroMoney::new(
                    state.table.seats.moneys[current_i]
                ) {
                    bundle.selection.bet = new_bet;
                }
            }

            let pot_limit = match bundle.round {
                Round::One => bundle.pot / 2,
                Round::AfterOne => bundle.pot,
            };

            // You can't bet more than the pot limit
            if bundle.selection.bet.get() > pot_limit {
                if let Some(new_bet) = NonZeroMoney::new(
                    pot_limit
                ) {
                    bundle.selection.bet = new_bet;
                }
            }

            match action_opt {
                Some(Action::Pass) => {
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
                None => {}
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

            enum Outcome {
                Loss,
                Win,
            }
            use Outcome::*;

            let outcome = {
                let ranks = [
                    get_rank(bundle.posts[0]),
                    get_rank(bundle.posts[1]),
                ];
                let min_rank = core::cmp::min(ranks[0], ranks[1]);
                let max_rank = core::cmp::max(ranks[0], ranks[1]);
                let third_rank = get_rank(*third);
                if min_rank < third_rank
                && third_rank < max_rank {
                    Win
                } else {
                    Loss
                }
            };

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
                        state.table.seats.moneys[current_i] =
                            state.table.seats.moneys[current_i]
                                .saturating_sub(bet.get());
                        bundle.pot = bundle.pot.saturating_add(bet.get());
                    }
                    Win => {
                        bundle.pot = bundle.pot.saturating_sub(bet.get());
                        state.table.seats.moneys[current_i] =
                            state.table.seats.moneys[current_i]
                                .saturating_add(bet.get());
                    }
                }

                if bundle.pot == 0 {
                    // TODO show a winner screen with more winner info.
                    if state.table.seats.personalities[0].is_none() {
                        println!("User wins!");
                    } else {
                        println!("Cpu player wins!");
                    }

                    group.speaker.request_sfx(SFX::CardPlace);
                    state.table.state = <_>::default();
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