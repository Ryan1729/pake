#![allow(unused_imports)]

use gfx::{CHAR_SPACING_W, SPACING_H, Commands};
use models::{holdem::{HandIndex}};
use platform_types::{Button, Dir, Input, Speaker, SFX, command, unscaled, TEXT};

use xs::{Xs, Seed};

// TODO? should this just be in models?
mod shared_game_types {
    pub type Personality = Option<CpuPersonality>;

    #[derive(Clone, Debug)]
    pub struct CpuPersonality {
        // TODO
    }

    #[derive(Clone, Default, PartialEq)]
    pub enum SkipState {
        #[default]
        Watch,
        Skip,
    }

    #[derive(Clone, Copy, Default)]
    pub enum ModeCmd {
        #[default]
        NoOp,
        BackToTitleScreen
    }
}
use shared_game_types::{ModeCmd};

mod holdem;

mod acey_deucey;

#[derive(Clone, Copy, Default)]
pub enum ModeName {
    #[default]
    Holdem,
    AceyDeucey
}

impl ModeName {
    fn text(self) -> &'static str {
        use ModeName::*;
        match self {
            Holdem => "hold'em",
            AceyDeucey => "acey-deucey",
        }
    }

    fn up(&mut self) {
        use ModeName::*;
        *self = match self {
            Holdem => AceyDeucey,
            AceyDeucey => Holdem,
        };
    }

    fn down(&mut self) {
        use ModeName::*;
        *self = match self {
            Holdem => AceyDeucey,
            AceyDeucey => Holdem,
        };
    }
}

#[derive(Clone)]
pub enum Mode {
    Title(ModeName),
    Holdem(holdem::Table),
    AceyDeucey(acey_deucey::Table)
}

impl Default for Mode {
    fn default() -> Self {
        Self::Title(<_>::default())
    }
}

#[derive(Clone, Default)]
pub struct State {
    pub rng: Xs,
    pub ctx: ui::Context,
    pub mode: Mode
}

impl State {
    pub fn new(seed: Seed) -> State {
        // 22 Players, User dealt a pair of 8s, beaten by a 8-high straight.
        //let seed = [177, 142, 173, 15, 242, 60, 217, 65, 49, 80, 175, 162, 108, 73, 4, 62];
        // 22 Players, User dealt a pair of Aces, wins with Aces over Queens.
        // let seed = [148, 99, 192, 160, 91, 61, 217, 65, 108, 157, 212, 200, 23, 73, 4, 62];
        let rng = xs::from_seed(seed);

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
        TitleBeginButton,
        GameSelect,
        BackToTitleScreen,
        Submit,
        PlayerCountSelect,
        StartingMoneySelect,
        HoldemHand(HandIndex),
        HoldemMenu(HoldemMenuId),
        HoldemChartButton,
        SkipRemainderOfGameSelect,
        ShowdownSubmit,
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
            TEXT
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
    state.ctx.frame_init();

    let mut cmd = ModeCmd::default();

    let mode = &mut state.mode;
    match mode {
        Mode::Title(mode_name) => {
            let title_cmd = title_update_and_render(
                commands,
                TitleState {
                    ctx: &mut state.ctx,
                    mode_name,
                },
                input,
                speaker,
            );

            match title_cmd {
                TitleCmd::NoOp => {},
                TitleCmd::StartMode(name) => {
                    *mode = match name {
                        ModeName::Holdem => {
                            Mode::Holdem(<_>::default())
                        },
                        ModeName::AceyDeucey => {
                            Mode::AceyDeucey(<_>::default())
                        },
                    };
                },
            }
        }
        Mode::Holdem(table) => {
            cmd = holdem::update_and_render(
                commands,
                holdem::State {
                    rng: &mut state.rng,
                    ctx: &mut state.ctx,
                    table,
                },
                input,
                speaker,
            );
        }
        Mode::AceyDeucey(table) => {
            cmd = acey_deucey::update_and_render(
                commands,
                acey_deucey::State {
                    rng: &mut state.rng,
                    ctx: &mut state.ctx,
                    table,
                },
                input,
                speaker,
            );
        }
    }

    match cmd {
        ModeCmd::NoOp => {},
        ModeCmd::BackToTitleScreen => {
            state.mode = Mode::Title(ModeName::Holdem);
        }
    }
}

struct TitleState<'state> {
    ctx: &'state mut ui::Context,
    mode_name: &'state mut ModeName,
}

enum TitleCmd {
    NoOp,
    StartMode(ModeName),
}

fn title_update_and_render(
    commands: &mut Commands,
    state: TitleState<'_>,
    input: Input,
    speaker: &mut Speaker,
) -> TitleCmd {
    let mut cmd = TitleCmd::NoOp;

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

    let group = new_group!();

    const TITLE_X: unscaled::X = unscaled::x_const_add_w(
        unscaled::X(0),
        unscaled::w_const_div(
            unscaled::w_const_sub(command::WIDTH_W, gfx::title::WIDTH),
            2
        )
    );
    const TITLE_Y: unscaled::Y = unscaled::Y(15);
    group.commands.draw_title(
        TITLE_X,
        TITLE_Y,
    );

    let mut y = TITLE_Y + gfx::title::HEIGHT;

    {
        y += SPACING_H;

        const CONTROLS_W: unscaled::W = unscaled::W(100);
        const CONTROLS_H: unscaled::H = unscaled::H(100);
        const CONTROLS_X: unscaled::X = unscaled::x_const_add_w(
            unscaled::X(0),
            unscaled::w_const_div(
                unscaled::w_const_sub(command::WIDTH_W, CONTROLS_W),
                2
            )
        );

        let controls_rect = unscaled::Rect {
            x: CONTROLS_X,
            y,
            w: CONTROLS_W,
            h: CONTROLS_H,
        };

        let controls_text = b"this game uses the z, x, and arrow keys";

        let xy = gfx::center_line_in_rect(
            controls_text.len() as _,
            controls_rect,
        );
        group.commands.print_chars(
            controls_text,
            xy.x,
            xy.y,
            TEXT
        );
    }

    // TODO in debug and/or a feature only, take a CLI arg or similar to
    // select a mode and skip the title screen, without user input

    let button_w = unscaled::W(50);
    let button_h = unscaled::H(50);

    let base_x = unscaled::X(0) + ((command::WIDTH_W) - unscaled::W(150)) / 2;
    let base_y = unscaled::Y(0) + command::HEIGHT_H - (button_h * 2);

    let game_select_rect = unscaled::Rect {
        x: base_x,
        y: base_y,
        w: unscaled::W(50),
        h: unscaled::H(50),
    };

    {
        let game_select_text = state.mode_name.text().as_bytes();

        let xy = gfx::center_line_in_rect(
            game_select_text.len() as _,
            game_select_rect,
        );

        group.commands.print_chars(
            game_select_text,
            xy.x,
            xy.y,
            TEXT
        );
    }

    ui::draw_quick_select(
        group,
        game_select_rect,
        GameSelect,
    );

    if do_button(
        group,
        ButtonSpec {
            id: TitleBeginButton,
            rect: unscaled::Rect {
                x: base_x + game_select_rect.w + unscaled::W(50),
                y: base_y,
                w: button_w,
                h: button_h,
            },
            text: b"begin",
        }
    ) {
        cmd = TitleCmd::StartMode(*state.mode_name);
    }

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    group.commands.print_chars(
        VERSION.as_bytes(),
        unscaled::X(0) + CHAR_SPACING_W,
        unscaled::Y(0) + (command::HEIGHT_H - gfx::CHAR_H),
        TEXT
    );

    match group.ctx.hot {
        GameSelect => {
            match input.dir_pressed_this_frame() {
                Some(Dir::Up) => {
                    state.mode_name.up();
                }
                Some(Dir::Down) => {
                    state.mode_name.down();
                }
                Some(Dir::Left)
                | Some(Dir::Right) => {
                    group.ctx.set_next_hot(TitleBeginButton);
                }
                None => {}
            }
        }
        TitleBeginButton => {
            match input.dir_pressed_this_frame() {
                Some(Dir::Left)
                | Some(Dir::Right) => {
                    group.ctx.set_next_hot(GameSelect);
                }
                Some(Dir::Up)
                | Some(Dir::Down)
                | None => {}
            }
        }
        _ => {}
    }

    if let Zero = group.ctx.hot {
        group.ctx.set_next_hot(GameSelect);
    }

    cmd
}
