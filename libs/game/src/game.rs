#![allow(unused_imports)]
#![deny(unreachable_patterns)]

use gfx::{CHAR_SPACING_W, SPACING_H, Commands};
use models::{OVERALL_MAX_PLAYER_COUNT, PlayerCount, holdem::{HandIndex}};
use platform_types::{Button, Dir, Input, Speaker, SFX, command, unscaled, TEXT};

use xs::{Xs, Seed};

macro_rules! compile_time_assert {
    ($assertion: expr) => (
        #[allow(unknown_lints, clippy::eq_op)]
        // Based on the const_assert macro from static_assertions;
        const _: [(); 0 - !{$assertion} as usize] = [];
    )
}

// TODO? should this just be in models?
mod shared_game_types {
    use models::NonZeroMoneyInner;

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

    #[derive(Clone, Copy, Default, PartialEq, Eq)]
    pub enum ModeCmd {
        #[default]
        NoOp,
        BackToTitleScreen,
        // Relevant for dealer's choice mode
        FinishedRound,
    }
}
use shared_game_types::{ModeCmd};

mod dealers_choice;

mod holdem;

mod acey_deucey;

mod five_card_draw;

macro_rules! all_up_down_impl {
    ($item_name: ident) => {
        impl $item_name {
            fn wrapping_up(self) -> Self {
                let mut index = self.index_of();
                if index == 0 {
                    index = Self::ALL.len() - 1;
                } else {
                    index = index.saturating_sub(1);
                }

                Self::ALL[index]
            }

            fn wrapping_down(self) -> Self {
                let mut index = self.index_of();
                index = index.saturating_add(1);
                if index >= Self::ALL.len() {
                    index = 0;
                }

                Self::ALL[index]
            }

            fn index_of(self) -> usize {
                let mut i = 0;
                for game in Self::ALL {
                    if game == self { break }
                    i += 1;
                }
                i
            }
        }
    }
}

macro_rules! mode_def {
    (
        {
            $mode_name: ident
            $mode: ident
            $sub_game: ident
            $sub_game_state: ident
            $sub_game_bitset: ident
            $sub_game_bits: ident
        }
        $dealers_choice: ident => (
            $dealers_choice_name: literal,
            $dealers_choice_path: ident
        ),
        [
            $($sub_games: ident =>
                (
                    $sub_games_text: literal,
                    $sub_games_path: ident
                )
            ),+
            $(,)?
        ]
    ) => {
        #[derive(Clone, Copy, Default, PartialEq, Eq)]
        pub enum $mode_name {
            #[default]
            $dealers_choice,
            $($sub_games),+
        }

        impl $mode_name {
            pub const COUNT: u8 = {
                let mut count = 0;

                let _ = Self::$dealers_choice;
                count += 1;

                $({
                    let _ = Self::$sub_games;
                    count += 1;
                })+

                count
            };

            pub const ALL: [Self; Self::COUNT as usize] = [
                Self::$dealers_choice,
                $(Self::$sub_games),+
            ];

            pub fn text(self) -> &'static str {
                use $mode_name::*;
                match self {
                    $dealers_choice => $dealers_choice_name,
                    $($sub_games => $sub_games_text),+
                }
            }

            pub fn new_mode(self) -> $mode {
                match self {
                    $mode_name::DealersChoice => {
                        $mode::DealersChoice(<_>::default())
                    },
                    $mode_name::Holdem => {
                        $mode::Holdem(<_>::default())
                    },
                    $mode_name::AceyDeucey => {
                        $mode::AceyDeucey(<_>::default())
                    },
                    $mode_name::FiveCardDraw => {
                        $mode::FiveCardDraw(<_>::default())
                    },
                }
            }
        }

        all_up_down_impl!{
            $mode_name
        }

        #[derive(Clone)]
        pub enum $mode {
            Title(ModeName),
            $dealers_choice($dealers_choice_path::Table),
            $($sub_games($sub_games_path::Table)),+
        }

        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        pub enum $sub_game {
            #[default]
            $($sub_games),+
        }

        impl $sub_game {
            pub const fn min_player_count(self) -> PlayerCount {
                use $sub_game::*;
                match self {
                    $($sub_games => $sub_games_path::MIN_PLAYERS),+
                }
            }

            pub const fn max_player_count(self) -> PlayerCount {
                use $sub_game::*;
                match self {
                    $($sub_games => $sub_games_path::MAX_PLAYERS),+
                }
            }

            pub fn text(self) -> &'static [u8] {
                use $sub_game::*;
                match self {
                    $($sub_games => $mode_name::$sub_games),+
                }.text().as_bytes()
            }

            pub const COUNT: u8 = {
                let mut count = 0;

                $({
                    let _ = Self::$sub_games;
                    count += 1;
                })+

                count
            };

            pub const ALL: [Self; Self::COUNT as usize] = [
                $(Self::$sub_games),+
            ];
        }

        all_up_down_impl!{
            $sub_game
        }

        #[derive(Clone, Default)]
        pub enum $sub_game_state {
            #[default]
            Choosing,
            $($sub_games($sub_games_path::Table)),+
        }

        type $sub_game_bits = u8;

        compile_time_assert!{
            $sub_game_bits::BITS as usize >= $sub_game::ALL.len()
        }

        #[derive(Clone, Copy, Debug, Default)]
        pub struct $sub_game_bitset($sub_game_bits);

        impl $sub_game_bitset {
            fn bit(game: $sub_game) -> $sub_game_bits {
                1 << (game.index_of())
            }
        }
    }
}

mode_def!{
    {ModeName Mode SubGame SubGameState SubGameBitset SubGameBits}
    DealersChoice => ("dealer's choice", dealers_choice),
    [
        Holdem => ("texas hold'em", holdem),
        AceyDeucey => ("acey-deucey", acey_deucey),
        FiveCardDraw => ("five-card draw", five_card_draw),
    ]
}

impl Default for Mode {
    fn default() -> Self {
        Self::Title(<_>::default())
    }
}

impl SubGameBitset {
    fn contains(self, game: SubGame) -> bool {
        let bit = Self::bit(game);

        self.0 & bit == bit
    }

    fn toggle(&mut self, game: SubGame) {
        let bit = Self::bit(game);

        self.0 ^= bit;
    }

    fn len(self) -> u32 {
        self.0.count_ones()
    }

    fn iter(self) -> impl Iterator<Item = SubGame> {
        let mut index = 0;
        std::iter::from_fn(move || {
            while usize::from(index) < SubGame::ALL.len() {
                let game = SubGame::ALL[index];

                index += 1;

                if self.contains(game) {
                    return Some(game);
                }
            }

            None
        })
    }
}

#[test]
fn iter_over_full_is_all() {
    let full = SubGameBitset((-1i128) as _);

    let actual: Vec<_> = full.iter().collect();

    assert_eq!(actual, SubGame::ALL.to_vec());
}

#[test]
fn iter_works_on_these_examples() {
    let actual: Vec<_> = SubGameBitset(0).iter().collect();

    assert_eq!(actual, []);

    let actual: Vec<_> = SubGameBitset(0b1).iter().collect();

    assert_eq!(actual, [SubGame::Holdem]);

    let actual: Vec<_> = SubGameBitset(0b10).iter().collect();

    assert_eq!(actual, [SubGame::AceyDeucey]);

    let actual: Vec<_> = SubGameBitset(0b11).iter().collect();

    assert_eq!(actual, [SubGame::Holdem, SubGame::AceyDeucey]);
}

// Keep this for the compile-time asserts
#[allow(dead_code)]
const CALCULATED_OVERALL_MAX_PLAYER_COUNT: PlayerCount = {
    let mut i = 0;
    let mut output = 0;
    while i < SubGame::ALL.len() {
        let max_player_count = SubGame::ALL[i].max_player_count();
        if max_player_count > output {
            output = max_player_count;
        }

        i += 1;
    }
    output
};

compile_time_assert!{
    CALCULATED_OVERALL_MAX_PLAYER_COUNT == 22
}

// We want to avoid having `models` rely on SubGame, but `models` needs to know what
// `OVERALL_MAX_PLAYER_COUNT` is. This seesmed like the best option among other 
// alternatives.
compile_time_assert!{
    OVERALL_MAX_PLAYER_COUNT == CALCULATED_OVERALL_MAX_PLAYER_COUNT
}

#[derive(Clone, Default)]
pub struct State {
    pub rng: Xs,
    pub ctx: ui::Context,
    pub mode: Mode
}

impl State {
    pub fn new(seed: Seed) -> State {
        // Hold'em
        // 22 Players, User dealt a pair of 8s, beaten by a 8-high straight.
        //let seed = [177, 142, 173, 15, 242, 60, 217, 65, 49, 80, 175, 162, 108, 73, 4, 62];
        // 22 Players, User dealt a pair of Aces, wins with Aces over Queens.
        // let seed = [148, 99, 192, 160, 91, 61, 217, 65, 108, 157, 212, 200, 23, 73, 4, 62];
        // Acey-Deucey
        // 2 players
        // Player gets dealt a pair of 8s early on (~3 rounds)
        // let seed = [145, 236, 211, 148, 118, 77, 217, 65, 97, 41, 161, 87, 46, 60, 4, 62];
        // 2 players
        // Ace to player eventually ~16 rounds
        // let seed = [58, 107, 196, 116, 32, 80, 217, 65, 179, 226, 72, 65, 13, 58, 4, 62];

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
    pub type AceyDeuceyMenuId = u8;
    pub type FiveCardDrawMenuId = u8;

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
        HoldemHand(holdem::HandIndex),
        HoldemMenu(HoldemMenuId),
        HoldemChartButton,
        SkipRemainderOfGameSelect,
        ShowdownSubmit,
        AceyDeuceyMenu(AceyDeuceyMenuId),
        NextDeal,
        AcceptBurn,
        HighLowSelect,
        HighLowSubmit,
        AcknowledgeCPUPass,
        FiveCardDrawHand(five_card_draw::HandIndex),
        FiveCardDrawMenu(FiveCardDrawMenuId),
        SubGameCheckbox(SubGame),
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

    /// Returns whether the checked state should be toggled.
    pub(crate) fn do_checkbox<'commands, 'ctx, 'speaker, 'text>(
        group: &mut Group<'commands, 'ctx, 'speaker>,
        x: unscaled::X,
        y: unscaled::Y,
        id: Id,
        is_checked: bool
    ) -> bool {
        use gfx::CheckboxMode as cm;

        let result = button_press(group, id);

        if group.ctx.active == id && group.input.gamepad.contains(Button::A) {
            group.commands.draw_checkbox(x, y, cm::Pressed(is_checked));
        } else if group.ctx.hot == id {
            group.commands.draw_checkbox(x, y, cm::Hot(is_checked));
        } else {
            group.commands.draw_checkbox(x, y, cm::Cold(is_checked));
        }

        result
    }

    #[macro_export]
    macro_rules! _stack_money_text {
        ($text:ident = $money: expr) => {
            let mut money_text = [0 as u8; 20];
            {
                use std::io::Write;
                money_text[0] = b'$';
                let _cant_actually_fail = write!(
                    &mut money_text[1..],
                    "{}",
                    $money
                );
            }

            let $text = money_text;
        }
    }
    pub use _stack_money_text as stack_money_text;

    #[macro_export]
    macro_rules! _draw_money_in_rect {
        ($group:ident, $money: expr, $rect: expr) => {
            $crate::ui::stack_money_text!(text = $money);

            {
                let xy = gfx::center_line_in_rect(
                    gfx::pre_nul_len(&text),
                    $rect,
                );
                $group.commands.print_chars(
                    &text,
                    xy.x,
                    xy.y,
                    TEXT
                );
            }
        }
    }

    pub use _draw_money_in_rect as draw_money_in_rect;
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
                    *mode = name.new_mode();
                },
            }
        }
        Mode::DealersChoice(table) => {
            cmd = dealers_choice::update_and_render(
                commands,
                dealers_choice::State {
                    rng: &mut state.rng,
                    ctx: &mut state.ctx,
                    table,
                },
                input,
                speaker,
            );
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
        Mode::FiveCardDraw(table) => {
            cmd = five_card_draw::update_and_render(
                commands,
                five_card_draw::State {
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
        ModeCmd::NoOp
        // We expect FinishedRound to have been handled earlier.
        | ModeCmd::FinishedRound => {},
        ModeCmd::BackToTitleScreen => {
            state.mode = Mode::Title(ModeName::default());
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
                    *state.mode_name = state.mode_name.wrapping_up();
                }
                Some(Dir::Down) => {
                    *state.mode_name = state.mode_name.wrapping_down();
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

    #[cfg(feature = "skip-to")]
    {
        let mut args = std::env::args();
        args.next(); // exe name

        while let Some(arg) = args.next() {
            match arg.as_str() {
                // select a mode and skip the title screen, without user input
                "--skip-to" => {
                    if let Some(arg) = args.next() {
                        // We currently prefer this to a FromStr impl because
                        // this way we keep everyting inside the feature cfg
                        // TODO? put inside mode_cfg macro, inside a cfg as well?
                        match arg.as_str() {
                            "holdem" => {
                                cmd = TitleCmd::StartMode(ModeName::Holdem);
                            }
                            "acey-deucey" => {
                                cmd = TitleCmd::StartMode(ModeName::AceyDeucey);
                            }
                            "five-card-draw" => {
                                cmd = TitleCmd::StartMode(ModeName::FiveCardDraw);
                            }
                            "dealers-choice" => {
                                cmd = TitleCmd::StartMode(ModeName::DealersChoice);
                            }
                            arg => {
                                panic!("Unrecognized arg for skip-to: {arg:?}");
                            }
                        }
                    } else {
                        panic!("--skip-to needs an addtional arg!");
                    }
                }
                arg => {
                    panic!("Unrecognized arg: {arg:?}");
                }
            }
        }
    }

    cmd
}
