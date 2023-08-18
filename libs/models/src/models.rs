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

pub const fn get_suit(card: Card) -> Suit {
    card / RANK_COUNT
}

pub type Rank = u8;

pub const fn get_rank(card: Card) -> Rank {
    card % RANK_COUNT
}

pub type Money = u32;

pub mod holdem {
    use super::*;

    pub type Hand = [Card; 2];

    #[derive(Copy, Clone, Debug, Default)]
    pub enum Action {
        #[default]
        Fold,
        Call,
        Raise(Money)
    }

    pub fn gen_action(rng: &mut Xs, one_past_max_money: Money) -> Action {
        use Action::*;

        match xs::range(rng, 0..3) {
            0 => Fold,
            1 => Call,
            _ => Raise(
                xs::range(rng, 1..core::cmp::max(2, one_past_max_money)) as Money
            )
        }
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub enum ActionKind {
        #[default]
        Fold,
        Call,
        Raise,
    }

    impl ActionKind {
        pub fn text(self) -> &'static [u8] {
            use ActionKind::*;
            match self {
                Fold => b"fold",
                Call => b"call",
                Raise => b"raise",
            }
        }
    }

    #[derive(Copy, Clone, Default)]
    pub enum Facing {
        #[default]
        Down,
        Up(Hand),
    }

    /// With 52 cards, and 5 community cards, and 3 burn cards,
    /// that leaves 44 cards left over so the maximum amount of
    /// possible hands is 22.
    pub const MAX_PLAYERS: u8 = 22;

    pub type PerPlayer<A> = [A; MAX_PLAYERS as usize];

    pub type HandIndex = u8;

    pub fn gen_hand_index(rng: &mut Xs, player_count: HandLen) -> HandIndex {
        xs::range(rng, 0..player_count.u8() as _) as HandIndex
    }

    #[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
    pub enum HandLen {
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
        Eightteen,
        Nineteen,
        Twenty,
        TwentyOne,
        TwentyTwo,
    }

    impl HandLen {
        pub fn saturating_add_1(self) -> Self {
            use HandLen::*;
            match self {
                Two => Three,
                Three => Four,
                Four => Five,
                Five => Six,
                Six => Seven,
                Seven => Eight,
                Eight => Nine,
                Nine => Ten,
                Ten => Eleven,
                Eleven => Twelve,
                Twelve => Thirteen,
                Thirteen => Fourteen,
                Fourteen => Fifteen,
                Fifteen => Sixteen,
                Sixteen => Seventeen,
                Seventeen => Eightteen,
                Eightteen => Nineteen,
                Nineteen => Twenty,
                Twenty => TwentyOne,
                TwentyOne
                | TwentyTwo => TwentyTwo,
            }
        }

        pub fn saturating_sub_1(self) -> Self {
            use HandLen::*;
            match self {
                Two
                | Three => Two,
                Four => Three,
                Five => Four,
                Six => Five,
                Seven => Six,
                Eight => Seven,
                Nine => Eight,
                Ten => Nine,
                Eleven => Ten,
                Twelve => Eleven,
                Thirteen => Twelve,
                Fourteen => Thirteen,
                Fifteen => Fourteen,
                Sixteen => Fifteen,
                Seventeen => Sixteen,
                Eightteen => Seventeen,
                Nineteen => Eightteen,
                Twenty => Nineteen,
                TwentyOne => Twenty,
                TwentyTwo => TwentyOne,
            }
        }

        pub fn text(self) -> &'static str {
            use HandLen::*;
            match self {
                Two => "2",
                Three => "3",
                Four => "4",
                Five => "5",
                Six => "6",
                Seven => "7",
                Eight => "8",
                Nine => "9",
                Ten => "10",
                Eleven => "11",
                Twelve => "12",
                Thirteen => "13",
                Fourteen => "14",
                Fifteen => "15",
                Sixteen => "16",
                Seventeen => "17",
                Eightteen => "18",
                Nineteen => "19",
                Twenty => "20",
                TwentyOne => "21",
                TwentyTwo => "22",
            }
        }

        pub fn u8(self) -> u8 {
            use HandLen::*;
            match self {
                Two => 2,
                Three => 3,
                Four => 4,
                Five => 5,
                Six => 6,
                Seven => 7,
                Eight => 8,
                Nine => 9,
                Ten => 10,
                Eleven => 11,
                Twelve => 12,
                Thirteen => 13,
                Fourteen => 14,
                Fifteen => 15,
                Sixteen => 16,
                Seventeen => 17,
                Eightteen => 18,
                Nineteen => 19,
                Twenty => 20,
                TwentyOne => 21,
                TwentyTwo => 22,
            }
        }

        pub fn usize(self) -> usize {
            usize::from(self.u8())
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct Hands {
        hands: PerPlayer<Hand>,
        len: HandLen,
    }

    impl Hands {
        pub fn iter(&self) -> impl Iterator<Item = Hand> {
            self.hands.into_iter().take(self.len.usize())
        }

        pub fn len(&self) -> HandLen {
            self.len
        }
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub enum PotAction {
        #[default]
        Fold,
        Bet(Money)
    }

    #[derive(Clone, Debug, Default)]
    pub struct Pot {
        // TODO? Is there a way to get a firm upper bound for the number of actions
        // per round? Maybe we could impose a (generous) raise limit then calculate
        // an upper bound based on that?
        // Assuming any heap allocations still make sense, since this will be append
        // only and then dropped all at once, an arena could work here to reduce the
        // number of allocations. Without any known speed concerns, or another use
        // case for an arena, bringing in that dependency doesn't currently seem
        // worth it.
        pub actions: PerPlayer<Vec<PotAction>>,
    }

    impl Pot {
        pub fn with_capacity(capacity: usize) -> Self {
            let mut output = Pot::default();

            for vec in &mut output.actions {
                *vec = Vec::with_capacity(capacity);
            }

            output
        }

        pub fn push_bet(&mut self, index: HandIndex, bet: PotAction) {
            self.actions[usize::from(index)].push(bet);
        }

        pub fn is_round_complete(&self, current_money: &PerPlayer<Money>) -> bool {
            // TODO correct this to be checking if the pot
            // has everyone either all in, or with matching bets

            let amounts = self.amounts();
            let mut previous_amount = None;
            for i in 0..amounts.len() {
                // A player is all in or not playing, if they have 0 money left.
                if current_money[i] == 0 {
                    continue;
                }

                if let Some(previous) = previous_amount {
                    if previous != amounts[i] {
                        return false;
                    }
                } else {
                    previous_amount = Some(amounts[i]);
                }
            }

            true
        }

        pub fn individual_pots(
            &self,
            current_money: &PerPlayer<Money>
        ) -> impl Iterator<Item = Money> {
            // A side pot exists if there is a higher amount than someone who is
            // still in, has bet. (TODO? filter out in-progress bets?)

            let mut amounts = self.amounts();

            let current_money = current_money.clone();

            std::iter::from_fn(move || {
                loop {
                    if amounts == [0; MAX_PLAYERS as usize] {
                        return None
                    }

                    let mut min_all_in = Money::MAX;
                    for i in 0..amounts.len() {
                        // A player is all in if they have 0 money left, 
                        // and actually bet something.
                        if current_money[i] == 0 {
                            if amounts[i] > 0 && amounts[i] < min_all_in {
                                min_all_in = amounts[i];
                            }
                        }
                    }

                    let mut contributor_count: Money = 0;
                    let mut output: Money = 0;
                    for i in 0..amounts.len() {
                        if amounts[i] > 0 {
                            contributor_count += 1;
                            match amounts[i].checked_sub(min_all_in) {
                                Some(new_amount) => {
                                    output = output.saturating_add(min_all_in);
                                    amounts[i] = new_amount;
                                },
                                None => {
                                    output = output.saturating_add(amounts[i]);
                                    amounts[i] = 0;
                                }
                            }
                        }
                    }
                    // Side pots with one player in them are "trivial" and not
                    // desired to be returned
                    if contributor_count > 1 && output != 0 {
                        return Some(output)
                    }
                }
            })
        }

        fn amounts(&self) -> PerPlayer<Money> {
            let mut outputs: PerPlayer<Money> = [0; MAX_PLAYERS as usize];
            for i in 0..MAX_PLAYERS as usize {
                let output = &mut outputs[i];
                for action in &self.actions[i] {
                    match action {
                        PotAction::Fold => break,
                        PotAction::Bet(bet) => {
                            *output = output.saturating_add(*bet);
                        }
                    }
                }
            }
            outputs
        }
    }

    #[cfg(test)]
    mod individual_pots_works {
        use super::*;
        #[derive(Debug)]
        struct Spec {
            bet: Money,
            is_all_in: bool,
        }

        fn bet(bet: Money) -> Spec {
            Spec {
                bet,
                is_all_in: false,
            }
        }

        fn all_in(bet: Money) -> Spec {
            Spec {
                bet,
                is_all_in: true,
            }
        }

        // Short for assert
        macro_rules! a {
            ($specs: expr, $expected: expr) => {
                let specs = $specs;
                let expected = $expected;

                let mut pot = Pot::default();

                let mut moneys = [0; MAX_PLAYERS as usize];

                for (i, spec) in specs.iter().enumerate() {
                    pot.push_bet(
                        HandIndex::try_from(i).unwrap(),
                        PotAction::Bet(spec.bet),
                    );

                    moneys[i] = if spec.is_all_in {
                        0
                    } else {
                        1
                    };
                }

                let actual: Vec<Money> = pot.individual_pots(&moneys).collect();

                let expected: Vec<Money> = expected.into_iter().collect();

                assert_eq!(actual, expected);
            }
        }

        #[test]
        fn on_these_examples() {
            a!([bet(5), bet(10)], [15]);
            a!([all_in(300), all_in(500)], [600]);
            a!([all_in(300), all_in(500), all_in(800)], [900, 400]);
            a!([all_in(800), all_in(500), all_in(300)], [900, 400]);
            a!([all_in(500), all_in(300), all_in(800)], [900, 400]);
            a!([all_in(300), all_in(500), bet(800)], [900, 400]);
            a!([all_in(300), all_in(500), bet(800), bet(800)], [300 * 4, 200 * 3, 300 * 2]);
            a!([all_in(300), all_in(500), bet(900), bet(900)], [300 * 4, 200 * 3, 400 * 2]);
        }
    }
    

    pub fn deal(
        rng: &mut Xs,
        player_count: HandLen,
    ) -> (Hands, Deck) {
        let mut deck = gen_deck(rng);

        let mut hands = Hands::default();

        let count = player_count.usize();

        for hand in (&mut hands.hands[0..count]).iter_mut() {
            let (Some(card1), Some(card2)) = (deck.draw(), deck.draw())
                else { continue };
            *hand = [card1, card2];
        }

        hands.len = player_count;

        (hands, deck)
        //deck.burn();
        //let [Some(card1), Some(card2), Some(card3)] =
            //[deck.draw(), deck.draw(), deck.draw()]
            //else {
                //debug_assert!(false, "Ran out of cards with fresh deck!?");
                //return Self::default()
            //};
        //community_cards: CommunityCards::Flop([card1, card2, card3]),
    }

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

