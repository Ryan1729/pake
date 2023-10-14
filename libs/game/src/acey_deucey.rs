use gfx::{Commands};
use models::{Card, ALL_CARDS, Money, NonZeroMoney};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};

use xs::Xs;

use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState};
use crate::ui::{self, ButtonSpec, Id::*, do_button};

type Posts = [Card; 2];

/// In some sense any number of players could play, but we want some maximum.
/// Each turn up to 3 cards may be dealt, so if more than 17 players play, then the
/// deck will need to be reshuffled every single round. This seems as good a place
/// to cap things as anywhere.
const MAX_PLAYERS: u8 = 17;

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

#[derive(Clone, Default)]
pub struct Seats {
    moneys: [Money; MAX_PLAYERS as usize],
    personalities: [Personality; MAX_PLAYERS as usize],
    skip: SkipState,
}

#[derive(Clone)]
pub enum TableState {
    Undealt { player_count: PlayerCount, starting_money: Money },
    DealtPosts {
        posts: Posts,
    },
    Reveal {
        posts: Posts,
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

    match state.table.state {
        Undealt { player_count, starting_money } => {
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

            match group.ctx.hot {
                Zero => {
                    group.ctx.set_next_hot(BackToTitleScreen);
                }
                _ => {}
            }
        },
        DealtPosts { posts } => {},
        Reveal { posts, third } => {},
    }
    

    cmd
}