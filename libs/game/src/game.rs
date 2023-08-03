#![allow(unused_imports)]

use gfx::{Commands, Highlighting::{Highlighted, Plain}};
use models::{Card, holdem::{CommunityCards, Deck, Hand, HandLen, Hands, gen_deck}};
use platform_types::{Button, Dir, Input, Speaker, SFX, command, unscaled};
use xs::{Xs, Seed};

#[derive(Clone)]
pub enum HoldemTable {
    Undealt { player_count: HandLen },
    PreFlop {
        hands: Hands,
    },
    PostFlop {
        hands: Hands,
        community_cards: CommunityCards,
    },
}

impl Default for HoldemTable {
    fn default() -> Self {
        Self::Undealt {
            player_count: <_>::default(),
        }
    }
}

#[derive(Clone, Default)]
pub struct State {
    pub rng: Xs,
    pub ctx: ui::Context,
    pub table: HoldemTable,
    pub deck: Deck,
}

impl State {
    pub fn new(seed: Seed) -> State {
        let mut rng = xs::from_seed(seed);

        let deck = gen_deck(&mut rng);

        //deck.burn();
        //let [Some(card1), Some(card2), Some(card3)] = 
            //[deck.draw(), deck.draw(), deck.draw()] 
            //else {
                //debug_assert!(false, "Ran out of cards with fresh deck!?");
                //return Self::default() 
            //};
        //community_cards: CommunityCards::Flop([card1, card2, card3]),

        State {
            deck,
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
        PlayerCountSelect,
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
    use HoldemTable::*;
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
            // TODO
        }
    }
    match &mut state.table {
        Undealt { ref mut player_count } => {
            let group = new_group!();

            group.ctx.set_next_hot(PlayerCountSelect);

            let player_count_rect = unscaled::Rect {
                x: unscaled::X(150),
                y: unscaled::Y(100),
                w: unscaled::W(50),
                h: unscaled::H(100),
            };

            // TODO avoid allocating for this.
            let player_count_string = player_count.to_string();
            let player_count_text = player_count_string.as_bytes();

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

            ui::draw_quick_select(
                group,
                player_count_rect,
                PlayerCountSelect,
            );

            match state.ctx.hot {
                PlayerCountSelect => {
                    match input.dir_pressed_this_frame() {
                        Some(Dir::Up) => {
                            *player_count = player_count.saturating_add(1);
                        },
                        Some(Dir::Down) => {
                            *player_count = player_count.saturating_sub(1);
                        },
                        _ => {}
                    }
                    
                }
                Zero => {}
            }
            
        },
        PreFlop { hands } => {
            draw_holdem_hands!(hands);
        },
        PostFlop { hands, community_cards } => {
            commands.draw_holdem_community_cards(
                *community_cards,
                unscaled::X(150),
                unscaled::Y(150),
            );

            draw_holdem_hands!(hands);
        },
    }
}
