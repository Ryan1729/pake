use gfx::{Commands};
use models::{Card, ALL_CARDS, Money, NonZeroMoney};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};

use xs::Xs;

use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState};
use crate::ui::{self, ButtonSpec, Id::*, do_button};

/// In some sense any number of players could play, but we want some maximum.
/// Each turn up to 3 cards may be dealt, so if more than 17 players play, then the
/// deck will need to be reshuffled every single round. This seems as good a place
/// to cap things as anywhere.
const MAX_PLAYERS: u8 = 17;

#[derive(Clone, Default)]
pub struct Seats {
    moneys: [Money; MAX_PLAYERS as usize],
    personalities: [Personality; MAX_PLAYERS as usize],
    skip: SkipState,
}

#[derive(Clone, Default)]
pub struct Table {
    seats: Seats,
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
    let mut cmd = ModeCmd::NoOp;

    cmd = ModeCmd::BackToTitleScreen;

    cmd
}