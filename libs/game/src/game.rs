use models::{Card, holdem::{Deck, Hand, gen_deck}};
use platform_types::{command, unscaled};
use xs::{Xs, Seed};

#[derive(Clone, Default)]
pub struct Splat {
    pub hand: Hand,
    pub x: unscaled::X,
    pub y: unscaled::Y,
}

#[derive(Clone, Default)]
pub struct State {
    pub rng: Xs,
    pub deck: Deck,
    pub splats: Vec<Splat>,
}

impl State {
    pub fn new(seed: Seed) -> State {
        let mut rng = xs::from_seed(seed);

        State {
            deck: gen_deck(&mut rng),
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

        self.splats.push(Splat {
            hand,
            x,
            y,
        });
    }
}
