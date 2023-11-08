use gfx::{card, pre_nul_len, Commands, SPACING_W, SPACING_H};
use models::{Money, NonZeroMoney};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};

use std::io::Write;

use xs::Xs;

use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState};
use crate::ui::{self, draw_money_in_rect, stack_money_text, ButtonSpec, Id::*, do_button};

// TODO restrict selection to minimum of selected games' max player count
type PlayerCount = u8;

#[derive(Clone)]
pub enum TableState {
    Undealt { player_count: PlayerCount, starting_money: Money },
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
    //pub seats: Seats,
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

            if let Zero = group.ctx.hot {
                group.ctx.set_next_hot(BackToTitleScreen);
            }
        }
    }

    cmd
}