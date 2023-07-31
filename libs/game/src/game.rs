use models::{Card, holdem::{CommunityCards, Deck, Hand, gen_deck}};
use platform_types::{command, unscaled};
use xs::{Xs, Seed};

#[derive(Clone, Default)]
pub struct Splat {
    pub hand: Hand,
    pub x: unscaled::X,
    pub y: unscaled::Y,
    pub evaluation: String,
}

#[derive(Clone, Default)]
pub struct State {
    pub rng: Xs,
    pub deck: Deck,
    pub community_cards: CommunityCards,
    pub splats: Vec<Splat>,
}

impl State {
    pub fn new(seed: Seed) -> State {
        let mut rng = xs::from_seed(seed);

        let mut deck = gen_deck(&mut rng);

        deck.burn();
        let [Some(card1), Some(card2), Some(card3)] = 
            [deck.draw(), deck.draw(), deck.draw()] 
            else {
                debug_assert!(false, "Ran out of cards with fresh deck!?");
                return Self::default() 
            };

        State {
            deck,
            community_cards: CommunityCards::Flop([card1, card2, card3]),
            rng,
            .. <_>::default()
        }
    }

    pub fn add_splat(&mut self) {
        let rng = &mut self.rng;

        let [Some(card1), Some(card2)] = [self.deck.draw(), self.deck.draw()] 
            else { return };
        let hand = [card1, card2];
        let x = unscaled::X(xs::range(rng, 0..command::WIDTH as u32) as command::Inner);
        let y = unscaled::Y(xs::range(rng, 0..command::HEIGHT as u32) as command::Inner);

        let evaluation = evaluate::holdem_hand(
            self.community_cards,
            hand
        ).to_string();

        self.splats.push(Splat {
            hand,
            x,
            y,
            evaluation,
        });
    }
}
