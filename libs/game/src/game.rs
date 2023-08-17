#![allow(unused_imports)]

use gfx::{SPACING_H, SPACING_W, Commands, Highlighting::{Highlighted, Plain}};
use models::{Card, Money, holdem::{MAX_PLAYERS, Action, ActionKind, CommunityCards, Deck, Facing, Hand, HandIndex, HandLen, Hands, Pot, PotAction, gen_action, gen_deck, gen_hand_index}};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled};
use xs::{Xs, Seed};

use std::io::Write;

#[derive(Clone, Default)]
pub struct HoldemMenuSelection {
    pub action_kind: ActionKind,
    pub bet: Money,
}

#[derive(Clone)]
pub struct HoldemStateBundle {
    pub deck: Deck,
    pub hands: Hands,
    pub dealer: HandIndex,
    pub current: HandIndex,
    pub pot: Pot,
    pub selection: HoldemMenuSelection,
}

#[derive(Clone)]
pub enum HoldemState {
    Undealt { player_count: HandLen, starting_money: Money },
    PreFlop {
        bundle: HoldemStateBundle,
    },
    PostFlop {
        bundle: HoldemStateBundle,
        community_cards: CommunityCards,
    },
}

impl Default for HoldemState {
    fn default() -> Self {
        Self::Undealt {
            player_count: <_>::default(),
            starting_money: 500,
        }
    }
}

type Personality = Option<CpuPersonality>;

#[derive(Clone)]
struct CpuPersonality {
    // TODO
}

#[derive(Clone, Default)]
pub struct HoldemTable {
    state: HoldemState,
    moneys: [Money; MAX_PLAYERS as usize],
    personalities: [Personality; MAX_PLAYERS as usize],
}

#[derive(Clone, Default)]
pub struct State {
    pub rng: Xs,
    pub ctx: ui::Context,
    pub table: HoldemTable,
}

impl State {
    pub fn new(seed: Seed) -> State {
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
        Submit,
        PlayerCountSelect,
        StartingMoneySelect,
        HoldemHand(HandIndex),
        HoldemMenu(HoldemMenuId),
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
            6
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

pub fn update_and_render(
    commands: &mut Commands,
    state: &mut State,
    input: Input,
    speaker: &mut Speaker,
) {
    use HoldemState::*;
    use ui::Id::*;

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

    if input.gamepad != <_>::default() {
        speaker.request_sfx(SFX::CardPlace);
    }

    const COMMUNITY_BASE_X: unscaled::X = unscaled::X(150);
    const COMMUNITY_BASE_Y: unscaled::Y = unscaled::Y(150);

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

    const TEXT: PaletteIndex = 6;

    macro_rules! do_holdem_hands {
        ($group: ident $(,)? $bundle: ident) => {
            let group = $group;
            let hands = &$bundle.hands;
            let dealer = $bundle.dealer;
            let current = $bundle.current;
            let pot = &$bundle.pot;

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

                    if usize::from(current) == i {
                        group.commands.draw_holdem_hand_underlight(
                            at.x,
                            at.y
                        );
                    }

                    i += 1;
                }
            }

            {
                let mut i = 0;
                for hand in hands.iter() {
                    let at = coords[i];

                    let show_if_player_owned = match group.ctx.hot {
                        HoldemHand(index) => usize::from(index) == i,
                        HoldemMenu(_) => true,
                        _ => false,
                    } && usize::from(current) == i;

                    let facing = if show_if_player_owned
                    && state.table.personalities[usize::from(current)].is_none() {
                        Facing::Up(hand)
                    } else {
                        Facing::Down
                    };
                    group.commands.draw_holdem_hand(
                        facing,
                        at.x,
                        at.y,
                    );

                    i += 1;
                }
            }

            const ACTION_KIND: ui::HoldemMenuId = 0;

            let mut i = 0;
            for _ in hands.iter() {
                match group.ctx.hot {
                    HoldemHand(mut index) if usize::from(index) == i => {
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

                            if usize::from(current) == i {
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
                                    index = hands_len;
                                }
                                index -= 1;
                                group.ctx.set_next_hot(HoldemHand(index));
                        } if group.input.pressed_this_frame(Button::RIGHT) {
                            index += 1;
                            if index >= hands_len {
                                index = 0;
                            }
                            group.ctx.set_next_hot(HoldemHand(index));
                        } if group.input.pressed_this_frame(Button::A) {
                            group.ctx.set_next_hot(HoldemMenu(ACTION_KIND));
                        } else {
                            group.ctx.set_next_hot(HoldemHand(index));
                        }
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


            if let Zero = group.ctx.hot {
                group.ctx.set_next_hot(HoldemHand(0));
            }

            {
                let mut y = COMMUNITY_BASE_Y;
                for amount in pot.individual_pots(&state.table.moneys) {
                    stack_money_text!(main_pot_text = amount);

                    group.commands.print_chars(
                        &main_pot_text,
                        COMMUNITY_BASE_X - pre_nul_len(&main_pot_text) * gfx::CHAR_W,
                        y,
                        6
                    );

                    y += gfx::CHAR_LINE_ADVANCE;
                }

                // TODO confirm this looks okay with the maximum number of amounts
                // which would be some function of MAX_PLAYERS. Exactly MAX_PLAYERS?
            }

            let action_opt = match &state.table.personalities[usize::from(current)] {
                Some(_personality) => {
                    // TODO Base choice off of personality
                    Some(gen_action(
                        &mut state.rng,
                        state.table.moneys[usize::from(current)] + 1
                    ))
                },
                None => {
                    match group.ctx.hot {
                        HoldemMenu(menu_id) => {
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

                            stack_money_text!(money_text = state.table.moneys[usize::from(current)]);

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

                            {
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
                                        6
                                    );
                                }

                                ui::draw_quick_select(
                                    group,
                                    action_kind_rect,
                                    HoldemMenu(ACTION_KIND),
                                );
                                // TODO allow player to select any legal action
                            }

                            if group.input.pressed_this_frame(Button::B) {
                                group.ctx.set_next_hot(HoldemHand(current));
                            } else {

                            }

                            None
                        }
                        _ => {
                            None
                        }
                    }
                }
            };

            if let Some(action) = action_opt {
                // TODO Confirm that all raises are legal.

                $bundle.current += 1;
                if $bundle.current >= hands.len().u8() {
                    $bundle.current = 0;
                }

                let is_done = if hands.len() == HandLen::Two {
                    // When head-to-head, the dealer acts first.
                    $bundle.current != dealer
                } else {
                    $bundle.current == dealer
                };

                dbg!(is_done);
            }
        }
    }
    match &mut state.table.state {
        Undealt {
            ref mut player_count,
            ref mut starting_money,
        } => {
            let group = new_group!();

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
                    6
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
                    6
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

            stack_money_text!(starting_money_text = starting_money);

            let xy = gfx::center_line_in_rect(
                pre_nul_len(&starting_money_text),
                starting_money_rect,
            );

            group.commands.print_chars(
                &starting_money_text,
                xy.x,
                xy.y,
                6
            );

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

                // TODO Make each element of this array user selectable too.
                // Start at 1 to make the first player user controlled
                for i in 1..player_count.usize() {
                    state.table.personalities[i] = Some(CpuPersonality{});
                }

                let (hands, deck) = models::holdem::deal(&mut state.rng, *player_count);

                let dealer = gen_hand_index(&mut state.rng, *player_count);

                let mut pot = Pot::with_capacity(16);

                let large_blind_amount = 10;
                let small_blind_amount = 5;
                let mut blinds = 0;
                {
                    let mut index = dealer;
                    if *player_count == HandLen::Two {
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
                        match state.table.moneys[usize::from(index)].checked_sub(small_blind_amount) {
                            Some(difference) => (difference, small_blind_amount),
                            None => (0, state.table.moneys[usize::from(index)]),
                        };
                    state.table.moneys[usize::from(index)] = new_total;
                    pot.push_bet(index, PotAction::Bet(subbed));

                    index += 1;
                    if index >= hands.len().u8() {
                        index = 0;
                    }

                    let (new_total, subbed) =
                        match state.table.moneys[usize::from(index)].checked_sub(large_blind_amount) {
                            Some(difference) => (difference, large_blind_amount),
                            None => (0, state.table.moneys[usize::from(index)]),
                        };
                    state.table.moneys[usize::from(index)] = new_total;
                    pot.push_bet(index, PotAction::Bet(subbed));
                }

                let current = if *player_count == HandLen::Two {
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

                state.table.state = PreFlop {
                    bundle: HoldemStateBundle {
                        hands,
                        deck,
                        dealer,
                        current,
                        pot,
                        selection: HoldemMenuSelection::default(),
                    },
                };
            } else {
                let menu = [PlayerCountSelect, StartingMoneySelect, Submit];

                match group.ctx.hot {
                    StartingMoneySelect => {
                        let menu_i = 1;
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up) => {
                                *starting_money = starting_money.saturating_add(5);
                            },
                            Some(Dir::Down) => {
                                *starting_money = starting_money.saturating_sub(5);
                                if *starting_money == 0 {
                                    *starting_money = 5;
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
                        let menu_i = 0;
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up) => {
                                *player_count = player_count.saturating_add_1();
                            },
                            Some(Dir::Down) => {
                                *player_count = player_count.saturating_sub_1();
                            },
                            Some(Dir::Left) => {}
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
                        group.ctx.set_next_hot(menu[0]);
                    }
                    _ => {}
                }
            }
        },
        PreFlop { bundle } => {
            let group = new_group!();
            do_holdem_hands!(group, bundle);
        },
        PostFlop { bundle, community_cards } => {
            let group = new_group!();

            group.commands.draw_holdem_community_cards(
                *community_cards,
                COMMUNITY_BASE_X,
                COMMUNITY_BASE_Y,
            );

            do_holdem_hands!(group, bundle);
        },
    }
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