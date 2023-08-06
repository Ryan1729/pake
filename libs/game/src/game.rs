#![allow(unused_imports)]

use gfx::{Commands, Highlighting::{Highlighted, Plain}};
use models::{Card, holdem::{MAX_PLAYERS, CommunityCards, Deck, Hand, HandLen, Hands, gen_deck}};
use platform_types::{Button, Dir, Input, Speaker, SFX, command, unscaled};
use xs::{Xs, Seed};

use std::io::Write;

#[derive(Clone)]
pub enum HoldemState {
    Undealt { player_count: HandLen, starting_money: Money },
    PreFlop {
        deck: Deck,
        hands: Hands,
    },
    PostFlop {
        deck: Deck,
        hands: Hands,
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

type Money = u32;

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

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub enum Id {
        #[default]
        Zero,
        Submit,
        PlayerCountSelect,
        StartingMoneySelect,
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

    macro_rules! draw_holdem_hands {
        ($hands: ident) => {
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
                        );

                        i += 1;
                        if i >= 22 {
                            break 'outer;
                        }
                    }
                }
            }
            
            let mut i = 0;
            for hand in $hands.iter() {
                let at = coords[i];
                commands.draw_holdem_hand(
                    hand,
                    at.x,
                    at.y,
                );

                i += 1;
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

            let mut starting_money_text = [0 as u8; 20];
            starting_money_text[0] = b'$';
            let _cant_actually_fail = write!(
                &mut starting_money_text[1..],
                "{starting_money}"
            );

            let xy = gfx::center_line_in_rect(
                {
                    let mut len = 0;
                    for i in 0..starting_money_text.len() as unscaled::Inner {
                        // If it's max length, this being outside the `if`
                        // ensures the length is accurate.
                        len = i;
                        if starting_money_text[usize::from(i)] == b'\0' {
                            break;
                        }
                    }
                    len
                },
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
                state.table.state = PreFlop {
                    hands,
                    deck
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
                }
            }
        },
        PreFlop { hands, deck: _ } => {
            draw_holdem_hands!(hands);
        },
        PostFlop { hands, deck: _, community_cards } => {
            commands.draw_holdem_community_cards(
                *community_cards,
                unscaled::X(150),
                unscaled::Y(150),
            );

            draw_holdem_hands!(hands);
        },
    }
}
