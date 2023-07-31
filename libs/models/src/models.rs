use xs::Xs;

pub const RANK_COUNT: u8 = 13;
pub const SUIT_COUNT: u8 = 4;
pub const DECK_SIZE: u8 = RANK_COUNT * SUIT_COUNT;

pub type Card = u8;

#[cfg(any())]
pub fn gen_card(rng: &mut Xs) -> Card {
    xs::range(rng, 0..DECK_SIZE as _) as Card
}

pub type Suit = u8;

pub mod suits {
    use super::*;

    pub const CLUBS: Suit = 0;
    pub const DIAMONDS: Suit = 1;
    pub const HEARTS: Suit = 2;
    pub const SPADES: Suit = 3;
}

pub fn get_suit(card: Card) -> Suit {
    card / RANK_COUNT
}

pub type Rank = u8;

pub fn get_rank(card: Card) -> Rank {
    card % RANK_COUNT
}

pub mod holdem {
    use super::*;

    pub type Hand = [Card; 2];

    type CardIndex = u8;

    #[derive(Clone, Debug)]
    pub struct Deck {
        cards: [Card; DECK_SIZE as usize],
        index: CardIndex,
    }

    impl Default for Deck {
        fn default() -> Self {
            Self {
                cards: [0; DECK_SIZE as usize],
                index: 0,
            }
        }
    }

    impl Deck {
        pub fn draw(&mut self) -> Option<Card> {
            if self.index >= DECK_SIZE {
                None
            } else {
                let output = Some(self.cards[self.index as usize]);

                self.index += 1;

                output
            }
        }

        pub fn burn(&mut self) {
            self.draw();
        }
    }

    pub fn gen_deck(rng: &mut Xs) -> Deck {
        let mut output = Deck::default();
        for i in 1..DECK_SIZE {
            output.cards[i as usize] = i;
        }
        xs::shuffle(rng, &mut output.cards);

        output
    }

    pub type Flop = [Card; 3];

    #[derive(Clone, Copy)]
    pub enum CommunityCards {
        Flop(Flop),
        Turn(Flop, Card),
        River(Flop, Card, Card),
    }

    impl Default for CommunityCards {
        fn default() -> Self {
            Self::Flop(<_>::default())
        }
    }
}

