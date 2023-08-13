use models::{Card, Rank, Suit, holdem, get_rank, get_suit, suits};

use platform_types::{Command, PALETTE, sprite, unscaled, command::{self, Rect}, PaletteIndex, FONT_BASE_Y, FONT_WIDTH};

#[derive(Copy, Clone, Default)]
pub enum Highlighting {
    #[default]
    Plain,
    Highlighted,
}

#[derive(Default)]
pub struct Commands {
    commands: Vec<Command>,
}

impl Commands {
    pub fn slice(&self) -> &[Command] {
        &self.commands
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    pub fn sspr(
        &mut self,
        sprite_xy: sprite::XY,
        rect: command::Rect,
    ) {
        self.commands.push(
            Command {
                sprite_xy,
                rect,
                colour_override: 0,
            }
        );
    }

    pub fn print_char(
        &mut self,
        character: u8, 
        x: unscaled::X,
        y: unscaled::Y,
        colour: PaletteIndex
    ) {
        fn get_char_xy(sprite_number: u8) -> sprite::XY {
            type Inner = sprite::Inner;
            let sprite_number = Inner::from(sprite_number);
            const CH_SIZE: Inner = CHAR_SIZE as Inner;
            const SPRITES_PER_ROW: Inner = FONT_WIDTH as Inner / CH_SIZE;
        
            sprite::XY {
                x: sprite::X(
                    (sprite_number % SPRITES_PER_ROW) * CH_SIZE
                ),
                y: sprite::Y(
                    FONT_BASE_Y as Inner + 
                    (sprite_number / SPRITES_PER_ROW) * CH_SIZE
                ),
            }
        }

        let sprite_xy = get_char_xy(character);
        self.commands.push(
            Command {
                sprite_xy,
                rect: Rect::from_unscaled(unscaled::Rect {
                    x,
                    y,
                    w: CHAR_W,
                    h: CHAR_H,
                }),
                colour_override: PALETTE[colour as usize],
            }
        );
    }

    pub fn print_chars(
        &mut self,
        characters: &[u8], 
        x: unscaled::X,
        y: unscaled::Y,
        colour: PaletteIndex
    ) {
        let mut at_x = x;
        for &character in characters {
            self.print_char(character, at_x, y, colour);
            at_x += CHAR_ADVANCE;
        }
    }

    // TODO? Randomize these for visual interest? {
    const HOLDEM_HAND_X_OFFSET: unscaled::W = unscaled::w_const_div(card::WIDTH, 2);
    const HOLDEM_HAND_Y_OFFSET: unscaled::H = unscaled::H(0);
    // }

    pub fn draw_holdem_hand_underlight(
        &mut self,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        let (new_x, clipped_w) = match x.checked_sub(SPACING_W) {
            Some(n_x) => (n_x, unscaled::W(0)),
            None => (unscaled::X(0), unscaled::W(x.get())),
        };
        let (new_y, clipped_h) = match y.checked_sub(SPACING_H) {
            Some(n_y) => (n_y, unscaled::H(0)),
            None => (unscaled::Y(0), unscaled::H(y.get())),
        };

        self.draw_nine_slice(
            NineSlice::Highlight,
            unscaled::Rect {
                x: new_x,
                y: new_y,
                w: (SPACING_W + Self::HOLDEM_HAND_X_OFFSET + card::WIDTH + SPACING_W) - clipped_w,
                h: (SPACING_H + Self::HOLDEM_HAND_Y_OFFSET + card::HEIGHT + SPACING_H) - clipped_h,
            },
        );
    }

    pub fn draw_holdem_hand(
        &mut self,
        facing: holdem::Facing,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        match facing {
            holdem::Facing::Down => {
                self.draw_card_back(x, y);
                self.draw_card_back(x + Self::HOLDEM_HAND_X_OFFSET, y + Self::HOLDEM_HAND_Y_OFFSET);
            }
            holdem::Facing::Up(hand) => {
                self.draw_card(hand[0], x, y);
                self.draw_card(hand[1], x + Self::HOLDEM_HAND_X_OFFSET, y + Self::HOLDEM_HAND_Y_OFFSET);
            },
        }
    }

    pub fn draw_holdem_community_cards(
        &mut self,
        cards: holdem::CommunityCards,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        let mut at_x = x;
        macro_rules! step {
            ($card: ident) => {
                self.draw_card($card, at_x, y);
                at_x += card::WIDTH;
            }
        }
        match cards {
            holdem::CommunityCards::Flop(flop) => {
                for card in flop {
                    step!(card);
                }
            }
            holdem::CommunityCards::Turn(flop, turn) => {
                for card in flop {
                    step!(card);
                }
                step!(turn);
            }
            holdem::CommunityCards::River(flop, turn, river) => {
                for card in flop {
                    step!(card);
                }
                step!(turn);
                step!(river);
            }
        }
    }

    pub fn draw_card(
        &mut self,
        card: Card,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        type Inner = sprite::Inner;
        let suit = get_suit(card);
        let rank = get_rank(card);

        self.draw_card_sprite(
            rank as Inner,
            suit as Inner,
            x,
            y,
        );
    }

    pub fn draw_card_back(
        &mut self,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        self.draw_card_sprite(
            13,
            1,
            x,
            y,
        );
    }

    fn draw_card_sprite(
        &mut self,
        sx: sprite::Inner,
        sy: sprite::Inner,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        type Inner = sprite::Inner;
        self.sspr(
            sprite::XY {
                x: sprite::X(card::BASE_X as Inner + sx * card::WIDTH.get()),
                y: sprite::Y(card::BASE_Y as Inner + sy * card::HEIGHT.get()),
            },
            Rect::from_unscaled(unscaled::Rect {
                x,
                y,
                w: card::WIDTH,
                h: card::HEIGHT,
            })
        );
    }

    pub fn draw_up_chevron(
        &mut self,
        highlighting: Highlighting,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        type Inner = sprite::Inner;
        self.sspr(
            sprite::XY {
                x: sprite::X(chevron::BASE_X as Inner + highlighting as Inner * chevron::WIDTH.get() * 2),
                y: sprite::Y(chevron::BASE_Y),
            },
            Rect::from_unscaled(unscaled::Rect {
                x,
                y,
                w: chevron::WIDTH,
                h: chevron::HEIGHT,
            })
        );
    }

    pub fn draw_down_chevron(
        &mut self,
        highlighting: Highlighting,
        x: unscaled::X,
        y: unscaled::Y
    ) {
        type Inner = sprite::Inner;
        self.sspr(
            sprite::XY {
                x: sprite::X(chevron::BASE_X as Inner + chevron::WIDTH.get() + highlighting as Inner * chevron::WIDTH.get() * 2),
                y: sprite::Y(chevron::BASE_Y),
            },
            Rect::from_unscaled(unscaled::Rect {
                x,
                y,
                w: chevron::WIDTH,
                h: chevron::HEIGHT,
            })
        );
    }
}

#[derive(Clone, Copy)]
pub enum NineSlice {
    Window,
    Button,
    ButtonHot,
    ButtonPressed,
    Highlight,
}

impl NineSlice {
    pub const CELL_W: unscaled::W = unscaled::W(8);
    pub const CELL_H: unscaled::H = unscaled::H(8);

    pub const GRID_W: unscaled::W = unscaled::W(24);
    pub const GRID_H: unscaled::H = unscaled::H(24);

    const BASE: sprite::XY = sprite::XY {
        x: sprite::X(FONT_WIDTH as _),
        y: sprite::Y(0),
    };

    fn top_left(self) -> sprite::XY {
        NineSlice::BASE 
        + NineSlice::GRID_W
        * match self {
            NineSlice::Window => 0,
            NineSlice::Button => 1,
            NineSlice::ButtonHot => 2,
            NineSlice::ButtonPressed => 3,
            NineSlice::Highlight => 4,
        }
    }
}

impl Commands {
    pub fn draw_nine_slice(
        &mut self,
        nine_slice: NineSlice,
        unscaled::Rect { x, y, w, h }: unscaled::Rect,
    ) {
        const WIDTH: unscaled::W = NineSlice::CELL_W;
        const HEIGHT: unscaled::H = NineSlice::CELL_H;

        macro_rules! r {
            ($x: ident, $y: ident $(,)?) => {
                Rect::from_unscaled(unscaled::Rect {
                    x: $x,
                    y: $y,
                    w: WIDTH,
                    h: HEIGHT,
                })
            };
        }

        let top_left: sprite::XY = nine_slice.top_left();

        let top: sprite::XY = top_left + WIDTH;
        let top_right: sprite::XY = top + WIDTH;

        let middle_left: sprite::XY = top_left + HEIGHT;
        let middle: sprite::XY = top + HEIGHT;
        let middle_right: sprite::XY = top_right + HEIGHT;

        let bottom_left: sprite::XY = middle_left + HEIGHT;
        let bottom: sprite::XY = middle + HEIGHT;
        let bottom_right: sprite::XY = middle_right + HEIGHT;

        let after_left_corner = x.saturating_add(WIDTH);
        let before_right_corner = x.saturating_add(w).saturating_sub(WIDTH);

        let below_top_corner = y.saturating_add(HEIGHT);
        let above_bottom_corner = y.saturating_add(h).saturating_sub(HEIGHT);

        macro_rules! step_by {
            (
                for $element: ident in $start: ident .. $end: ident 
                step_by $by: ident 
                $body: block
            ) => ({
                let mut $element = $start;
                while $element < $end {
                    $body

                    $element += $by;
                }
            })
        }

        step_by!(
            for fill_y in below_top_corner..above_bottom_corner
            step_by HEIGHT {
                step_by!(
                    for fill_x in after_left_corner..before_right_corner
                    step_by WIDTH {
                        self.sspr(
                            middle,
                            r!(fill_x, fill_y),
                        );
                    }
                )
            }
        );

        step_by!(
            for fill_x in after_left_corner..before_right_corner
            step_by WIDTH {
                self.sspr(
                    top,
                    r!(fill_x, y),
                );
    
                self.sspr(
                    bottom,
                    r!(fill_x, above_bottom_corner),
                );
            }
        );

        step_by!(
            for fill_y in below_top_corner..above_bottom_corner
            step_by HEIGHT {
                self.sspr(
                    middle_left,
                    r!(x, fill_y),
                );
    
                self.sspr(
                    middle_right,
                    r!(before_right_corner, fill_y),
                );
            }
        );

        self.sspr(
            top_left,
            r!(x, y),
        );

        self.sspr(
            top_right,
            r!(before_right_corner, y),
        );

        self.sspr(
            bottom_left,
            r!(x, above_bottom_corner),
        );

        self.sspr(
            bottom_right,
            r!(before_right_corner, above_bottom_corner),
        );
    }
}

pub mod card {
    use super::*;

    use unscaled::{W, H, w_const_add, w_const_sub, h_const_add, h_const_sub};

    pub const WIDTH: W = W(42);
    pub const HEIGHT: H = H(60);

    pub const BASE_X: u16 = 436;
    pub const BASE_Y: u16 = 0;

    pub const LEFT_RANK_EDGE_W: W = W(3);
    pub const LEFT_RANK_EDGE_H: H = H(3);

    pub const LEFT_SUIT_EDGE_W: W = W(1);
    pub const LEFT_SUIT_EDGE_H: H = H(10);

    pub const RIGHT_RANK_EDGE_W: W = w_const_sub(
        WIDTH, 
        w_const_add(LEFT_RANK_EDGE_W, CHAR_W)
    );
    pub const RIGHT_RANK_EDGE_H: H = h_const_sub(
        HEIGHT, 
        h_const_add(LEFT_RANK_EDGE_H, CHAR_H)
    );

    pub const RIGHT_SUIT_EDGE_W: W = w_const_sub(
        WIDTH, 
        w_const_add(LEFT_SUIT_EDGE_W, CHAR_W)
    );
    pub const RIGHT_SUIT_EDGE_H: H = h_const_sub(
        HEIGHT, 
        h_const_add(LEFT_SUIT_EDGE_H, CHAR_H)
    );
}

pub mod chevron {
    use super::*;

    use unscaled::{W, H};

    pub const WIDTH: W = W(24);
    pub const HEIGHT: H = H(12);

    pub const BASE_X: u16 = 128;
    pub const BASE_Y: u16 = 24;
}

pub const TEN_CHAR: u8 = 27;

pub const CLUB_CHAR: u8 = 31;
pub const DIAMOND_CHAR: u8 = 29;
pub const HEART_CHAR: u8 = 30;
pub const SPADE_CHAR: u8 = 28;

pub fn get_suit_colour_and_char(suit: Suit) -> (u8, u8) {
    const RED_INDEX: u8 = 2;
    const PURPLE_INDEX: u8 = 4;
    const BLACK_INDEX: u8 = 7;

    match suit {
        suits::CLUBS => (BLACK_INDEX, CLUB_CHAR),
        suits::DIAMONDS => (RED_INDEX, DIAMOND_CHAR),
        suits::HEARTS => (RED_INDEX, HEART_CHAR),
        suits::SPADES => (BLACK_INDEX, SPADE_CHAR),
        _ => (PURPLE_INDEX, 33), //purple "!"
    }
}

pub fn get_rank_char(card: Card) -> u8 {
    get_rank_char_from_rank(get_rank(card))
}

pub fn get_rank_char_from_rank(rank: Rank) -> u8 {
    match rank {
        0 => b'a',
        1 => b'2',
        2 => b'3',
        3 => b'4',
        4 => b'5',
        5 => b'6',
        6 => b'7',
        7 => b'8',
        8 => b'9',
        9 => TEN_CHAR,
        10 => b'j',
        11 => b'q',
        12 => b'k',
        _ => b'!',
    }
}




pub const CHAR_SPACING: u8 = 2;
pub const CHAR_SPACING_W: unscaled::W = unscaled::W(CHAR_SPACING as _);
pub const CHAR_SPACING_H: unscaled::H = unscaled::H(CHAR_SPACING as _);
pub const CHAR_SIZE: u8 = 8;
pub const CHAR_ADVANCE: unscaled::W = unscaled::W(4);
pub const CHAR_W: unscaled::W = unscaled::W(CHAR_SIZE as _);
pub const CHAR_H: unscaled::H = unscaled::H(CHAR_SIZE as _);
pub const CHAR_LINE_ADVANCE: unscaled::H = unscaled::H(
    CHAR_SIZE as unscaled::Inner
    + CHAR_SPACING as unscaled::Inner
);

pub const SPACING: u8 = CHAR_SIZE;
pub const SPACING_W: unscaled::W = unscaled::W(SPACING as _);
pub const SPACING_H: unscaled::H = unscaled::H(SPACING as _);

pub const FONT_FLIP: u8 = 128;

pub type TextLength = unscaled::Inner;

pub fn center_line_in_rect(
    text_length: TextLength,
    rect: unscaled::Rect
) -> unscaled::XY {
    let unscaled::Rect { x, y, w, h } = rect;

    let mut xy = unscaled::XY {
        x: x + (w / 2),
        y: y + (h / 2),
    };

    xy.x -= unscaled::W((CHAR_ADVANCE * text_length).get() / 2);
    xy.y -= unscaled::H(CHAR_H.get() / 2);

    xy
}