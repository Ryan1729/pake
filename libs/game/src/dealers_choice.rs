use gfx::{card, checkbox, pre_nul_len, Commands, SPACING_W, SPACING_H};
use models::{Money, MoneyInner, MoneyMove, NonZeroMoney, NonZeroMoneyInner, MIN_MONEY_UNIT};
use platform_types::{Button, Dir, Input, PaletteIndex, Speaker, SFX, command, unscaled, TEXT};

use std::io::Write;

use xs::Xs;

use crate::{acey_deucey, five_card_draw, holdem, PlayerCount, SubGame, SubGameState, SubGameBitset, OVERALL_MAX_PLAYER_COUNT};
use crate::shared_game_types::{CpuPersonality, Personality, ModeCmd, SkipState};
use crate::ui::{self, draw_money_in_rect, stack_money_text, ButtonSpec, Id::*, do_button, do_checkbox};

type Moneys = [Money; OVERALL_MAX_PLAYER_COUNT as usize];

#[derive(Clone)]
pub enum TableState {
    Undealt { player_count: PlayerCount, starting_money: MoneyInner },
    Playing { 
        player_count: PlayerCount,
        moneys: Moneys,
        sub_game_state: SubGameState,
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
    //pub seats: Seats,
    pub state: TableState,
    // TODO default to all selected
    // TODO make a button to toggle all bits
    pub chooseable_games: SubGameBitset,
}

pub struct State<'state> {
    pub rng: &'state mut Xs,
    pub ctx: &'state mut ui::Context,
    pub table: &'state mut Table,
}

fn clamp_player_count(
    player_count: &mut PlayerCount,
    sub_games: SubGameBitset,
) {
    for game in sub_games.iter() {
        // TODO handle possible case of minimum of one game being larger than the
        // maximum of another, if that actually comes up.
        *player_count = core::cmp::max(*player_count, game.min_player_count());
        *player_count = core::cmp::min(*player_count, game.max_player_count());
    }
}

#[test]
fn clamp_player_count_works_on_this_found_example() {
    let mut player_count = holdem::MAX_PLAYERS;

    assert!(acey_deucey::MAX_PLAYERS < holdem::MAX_PLAYERS, "pre-condition failure");
    
    let acey_deucey_set = SubGameBitset(0b10);
    clamp_player_count(&mut player_count, acey_deucey_set);

    assert_eq!(player_count, acey_deucey::MAX_PLAYERS);
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

            let game_set_rect = unscaled::Rect {
                x: unscaled::X(50),
                y: unscaled::Y(100),
                w: unscaled::W(75),
                h: unscaled::H(100),
            };

            {
                let label_h = unscaled::W(50);
                let line_h = gfx::CHAR_H + SPACING_H;
                let mut y = game_set_rect.y;
                for game in SubGame::ALL {
                    if do_checkbox(
                        group,
                        game_set_rect.x, 
                        y,
                        SubGameCheckbox(game),
                        state.table.chooseable_games.contains(game),
                    ) {
                        state.table.chooseable_games.toggle(game);
                        clamp_player_count(player_count, state.table.chooseable_games);
                    }
                    
                    let label_rect = unscaled::Rect {
                        x: game_set_rect.x + checkbox::WIDTH + SPACING_W,
                        y: y - (SPACING_H / 2) + unscaled::H(1),
                        w: label_h,
                        h: line_h,
                    };

                    let game_text = game.text();

                    {
                        let xy = gfx::center_line_in_rect(
                            game_text.len() as _,
                            label_rect,
                        );
                        group.commands.print_chars(
                            game_text,
                            xy.x,
                            xy.y,
                            TEXT
                        );
                    }

                    y += line_h;
                }
            }

            let player_count_rect = unscaled::Rect {
                x: game_set_rect.x + game_set_rect.w,
                w: unscaled::W(50),
                ..game_set_rect
            };
    
            let mut player_count_text = [0 as u8; 20];
            {
                use std::io::Write;
                let _cant_actually_fail = write!(
                    &mut player_count_text[..],
                    "{player_count}",
                );
            }

            {
                let xy = gfx::center_line_in_rect(
                    pre_nul_len(&player_count_text),
                    player_count_rect,
                );
                group.commands.print_chars(
                    &player_count_text,
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
                x: player_count_rect.x + player_count_rect.w,
                ..player_count_rect
            };

            draw_money_in_rect!(group, starting_money, starting_money_rect);

            ui::draw_quick_select(
                group,
                starting_money_rect,
                StartingMoneySelect,
            );

            let is_valid_to_submit = state.table.chooseable_games.len() > 1;

            if is_valid_to_submit 
            && do_button(
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
                let mut moneys = [0; OVERALL_MAX_PLAYER_COUNT as usize];
                for i in 0..usize::from(player_count) {
                    moneys[i] = *starting_money;
                }
                let moneys = Money::array_from_inner_array(moneys);

                state.table.state = Playing {
                    player_count,
                    moneys,
                    sub_game_state: <_>::default(),
                };
            } else {
                let menu = [BackToTitleScreen, SubGameCheckbox(SubGame::default()), PlayerCountSelect, StartingMoneySelect, Submit];

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
                    SubGameCheckbox(game) => {
                        let menu_i = 1;
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up) => {
                                group.ctx.set_next_hot(SubGameCheckbox(game.wrapping_up()));
                            },
                            Some(Dir::Down) => {
                                group.ctx.set_next_hot(SubGameCheckbox(game.wrapping_down()));
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
                        let menu_i = 2;
                        match input.dir_pressed_this_frame() {
                            Some(Dir::Up) => {
                                *player_count = player_count.saturating_add(1);
                            },
                            Some(Dir::Down) => {
                                *player_count = player_count.saturating_sub(1);
                            },
                            Some(Dir::Left) => {
                                group.ctx.set_next_hot(menu[menu_i - 1]);
                            }
                            Some(Dir::Right) => {
                                group.ctx.set_next_hot(menu[menu_i + 1]);
                            }
                            None => {}
                        }

                        clamp_player_count(player_count, state.table.chooseable_games);
                    }
                    StartingMoneySelect => {
                        let menu_i = 3;
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
                        group.ctx.set_next_hot(menu[1]);
                    }
                    _ => {}
                }
            }
        }
        Playing { 
            ref mut player_count,
            ref mut moneys,
            ref mut sub_game_state,
        } => {
            use SubGameState::*;
            match sub_game_state {
                Choosing => {
                    // TODO actual choosing
                    let sub_game = SubGame::Holdem;

                    match sub_game {
                        SubGame::Holdem => {
                            let player_count: holdem::PlayerCount =
                                match holdem::PlayerCount::try_from(*player_count) {
                                    Ok(player_count) => player_count,
                                    Err(()) => {
                                        debug_assert!(
                                            false
                                        );
                                        return ModeCmd::BackToTitleScreen;
                                    }
                                };

                            let mut holdem_moneys = 
                                [Money::ZERO; holdem::MAX_PLAYERS as usize];
                            for i in 0..player_count.usize() {
                                holdem_moneys[i] = moneys[i].take_all();
                            }

                            *sub_game_state = Holdem(
                                holdem::Table::selected(
                                    rng,
                                    player_count,
                                    holdem_moneys,
                                )
                            );
                        }
                        SubGame::AceyDeucey => {
                            let player_count: acey_deucey::PlayerCount =
                                match acey_deucey::PlayerCount::try_from(*player_count) {
                                    Ok(player_count) => player_count,
                                    Err(error) => {
                                        debug_assert!(
                                            false,
                                            "{error}"
                                        );
                                        return ModeCmd::BackToTitleScreen;
                                    }
                                };

                            let mut acey_deucey_moneys = 
                                [Money::ZERO; acey_deucey::MAX_PLAYERS as usize];
                            for i in 0..player_count.usize() {
                                acey_deucey_moneys[i] = moneys[i].take_all();
                            }

                            *sub_game_state = AceyDeucey(
                                acey_deucey::Table::selected(
                                    rng,
                                    player_count,
                                    acey_deucey_moneys,
                                )
                            );
                        }
                        SubGame::FiveCardDraw => {
                            let player_count: five_card_draw::PlayerCount =
                                match five_card_draw::PlayerCount::try_from(*player_count) {
                                    Ok(player_count) => player_count,
                                    Err(error) => {
                                        debug_assert!(
                                            false,
                                            "{error}"
                                        );
                                        return ModeCmd::BackToTitleScreen;
                                    }
                                };

                            let mut five_card_draw_moneys = 
                                [Money::ZERO; five_card_draw::MAX_PLAYERS as usize];
                            for i in 0..player_count.usize() {
                                five_card_draw_moneys[i] = moneys[i].take_all();
                            }

                            *sub_game_state = FiveCardDraw(
                                five_card_draw::Table::selected(
                                    rng,
                                    player_count,
                                    five_card_draw_moneys,
                                )
                            );
                        }
                    }
                }
                Holdem(ref mut table) => {
                    cmd = holdem::update_and_render(
                        commands,
                        holdem::State {
                            rng,
                            ctx: state.ctx,
                            table
                        },
                        input,
                        speaker,
                    );

                    if cmd == ModeCmd::FinishedRound {
                        for i in 0..usize::from(*player_count) {
                            moneys[i] = table.seats.moneys[i].take_all();
                        }

                        *sub_game_state = Choosing;
                    }
                }
                AceyDeucey(ref mut table) => {
                    cmd = acey_deucey::update_and_render(
                        commands,
                        acey_deucey::State {
                            rng,
                            ctx: state.ctx,
                            table
                        },
                        input,
                        speaker,
                    );

                    if cmd == ModeCmd::FinishedRound {
                        for i in 0..usize::from(*player_count) {
                            moneys[i] = table.seats.moneys[i].take_all();
                        }

                        *sub_game_state = Choosing;
                    }
                }
                FiveCardDraw(ref mut table) => {
                    cmd = five_card_draw::update_and_render(
                        commands,
                        five_card_draw::State {
                            rng,
                            ctx: state.ctx,
                            table
                        },
                        input,
                        speaker,
                    );

                    if cmd == ModeCmd::FinishedRound {
                        for i in 0..usize::from(*player_count) {
                            moneys[i] = table.seats.moneys[i].take_all();
                        }

                        *sub_game_state = Choosing;
                    }
                }
            }
            
        },
    }

    cmd
}