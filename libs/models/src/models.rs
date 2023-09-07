use xs::Xs;
use core::num::NonZeroU32;

macro_rules! compile_time_assert {
    ($assertion: expr) => (
        #[allow(unknown_lints, clippy::eq_op)]
        // Based on the const_assert macro from static_assertions;
        const _: [(); 0 - !{$assertion} as usize] = [];
    )
}

pub const RANK_COUNT: u8 = 13;
pub const SUIT_COUNT: u8 = 4;
pub const DECK_SIZE: u8 = RANK_COUNT * SUIT_COUNT;

pub type Card = u8;

pub const ALL_CARDS: [Card; DECK_SIZE as usize] = {
    let mut all_cards = [0; DECK_SIZE as usize];

    let mut c = 0;
    while c < DECK_SIZE {
        all_cards[c as usize] = c;
        c += 1;
    }

    all_cards
};

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
pub type NonZeroMoney = NonZeroU32;

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

    #[derive(Debug)]
    pub struct ActionSpec {
        pub one_past_max_money: NonZeroMoney,
        pub min_money_unit: NonZeroMoney,
        pub call_amount: Money,
    }

    pub fn gen_action(
        rng: &mut Xs,
        ActionSpec { one_past_max_money, min_money_unit, call_amount }: ActionSpec
    ) -> Action {
        use Action::*;

        match xs::range(rng, 0..3) {
            0 => Fold,
            1 => Call,
            _ => {
                // TODO? Maybe just take max_money as a param?
                let max_money = one_past_max_money.get() - 1;

                if call_amount > max_money {
                    // Go all in
                    Call
                } else {
                    let max_in_units = max_money/min_money_unit.get();
                    let call_in_units = call_amount/min_money_unit.get();
                    let output_in_units = xs::range(rng, call_in_units..core::cmp::max(call_in_units, max_in_units).saturating_add(1)) as Money;
                    let output_in_money = output_in_units.saturating_mul(min_money_unit.get());

                    Raise(output_in_money)
                }
            }
        }
    }

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
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

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub enum AllowedKindMode {
        #[default]
        All,
        NoFolding,
    }

    impl ActionKind {
        pub fn next_up(self, mode: AllowedKindMode) -> Self {
            use ActionKind::*;
            use AllowedKindMode::*;
            match mode {
                All => match self {
                    Fold => Call,
                    Call => Raise,
                    Raise => Fold,
                },
                NoFolding => match self {
                    Fold => Call,
                    Call => Raise,
                    Raise => Call,
                },
            }
        }

        pub fn next_down(self, mode: AllowedKindMode) -> Self {
            use ActionKind::*;
            use AllowedKindMode::*;
            match mode {
                All => match self {
                    Fold => Raise,
                    Call => Fold,
                    Raise => Call,
                },
                NoFolding => match self {
                    Fold => Raise,
                    Call => Raise,
                    Raise => Call,
                },
            }
            
        }
    }

    #[derive(Copy, Clone, Default)]
    pub enum Facing {
        #[default]
        Down,
        Up(Hand),
    }

    /// Does not necessarily contain a valid number of players for a round.
    /// For a type with that guarentee see `HandLen`.
    pub type PlayerAmount = u8;

    /// With 52 cards, and 5 community cards, and 3 burn cards,
    /// that leaves 44 cards left over so the maximum amount of
    /// possible hands is 22.
    pub const MAX_PLAYERS: PlayerAmount = 22;
    // TODO? Is the amount of possible pots MAX_PLAYERS - 1? Or even lower?
    pub const MAX_POTS: u8 = MAX_PLAYERS;

    pub type PerPlayer<A> = [A; MAX_PLAYERS as usize];

    type PerPlayerBits = u32;
    #[derive(Clone, Copy, Debug, Default)]
    pub struct PerPlayerBitset(PerPlayerBits);

    compile_time_assert!{
        PerPlayerBits::BITS >= MAX_PLAYERS as u32
    }

    impl PerPlayerBitset {
        pub fn len(&self) -> PlayerAmount {
            self.0.count_ones() as PlayerAmount
        }

        pub fn set(&mut self, index: HandIndex){
            if index > MAX_PLAYERS { return }
            self.0 |= 1 << PerPlayerBits::from(index);
        }

        pub fn iter(self) -> PerPlayerBitsetIter {
            PerPlayerBitsetIter {
                set: self,
                index: 0,
            }
        }
    }

    pub struct PerPlayerBitsetIter {
        set: PerPlayerBitset,
        index: HandIndex,
    }

    impl Iterator for PerPlayerBitsetIter {
        type Item = HandIndex;

        fn next(&mut self) -> Option<Self::Item> {
            while self.index < MAX_PLAYERS {
                if (self.set.0 & 1 << self.index) != 0 {
                    let output = self.index;

                    self.index += 1;

                    return Some(output);
                }

                self.index += 1;
            }

            None
        }
    }

    pub type HandIndex = u8;
    pub const MAX_HAND_INDEX: u8 = MAX_PLAYERS - 1;

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

        pub fn amount(self) -> PlayerAmount {
            self.u8()
        }

        pub fn usize(self) -> usize {
            usize::from(self.u8())
        }
    }

    impl TryFrom<u8> for HandLen {
        type Error = ();

        fn try_from(byte: u8) -> Result<Self, Self::Error> {
            use HandLen::*;
            match byte {
                2 => Ok(Two),
                3 => Ok(Three),
                4 => Ok(Four),
                5 => Ok(Five),
                6 => Ok(Six),
                7 => Ok(Seven),
                8 => Ok(Eight),
                9 => Ok(Nine),
                10 => Ok(Ten),
                11 => Ok(Eleven),
                12 => Ok(Twelve),
                13 => Ok(Thirteen),
                14 => Ok(Fourteen),
                15 => Ok(Fifteen),
                16 => Ok(Sixteen),
                17 => Ok(Seventeen),
                18 => Ok(Eightteen),
                19 => Ok(Nineteen),
                20 => Ok(Twenty),
                21 => Ok(TwentyOne),
                22 => Ok(TwentyTwo),
                _ => Err(())
            }
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

        pub fn get(&self, index: HandIndex) -> Option<&Hand> {
            self.hands.get(usize::from(index))
        }
    }

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    pub enum PotAction {
        #[default]
        Fold,
        Bet(Money)
    }

    #[derive(Clone, Debug)]
    pub struct Pot {
        // TODO? Is there a way to get a firm upper bound for the number of actions
        // per round? Maybe we could impose a (generous) raise limit then calculate
        // an upper bound based on that?
        // Assuming any heap allocations still make sense, since this will be append
        // only and then dropped all at once, an arena could work here to reduce the
        // number of allocations. Without any known speed concerns, or another use
        // case for an arena, bringing in that dependency doesn't currently seem
        // worth it.
        actions: PerPlayer<Vec<PotAction>>,
        player_count: HandLen,
        has_gone_this_round: PerPlayerBitset,
    }

    impl Pot {
        pub fn with_capacity(player_count: HandLen, capacity: usize) -> Self {
            let mut output = Pot{
                actions: <_>::default(),
                player_count,
                has_gone_this_round: <_>::default(),
            };

            for vec in &mut output.actions {
                *vec = Vec::with_capacity(capacity);
            }

            output
        }

        pub fn push_bet(&mut self, index: HandIndex, bet: PotAction) {
            self.has_gone_this_round.set(index);
            self.actions[usize::from(index)].push(bet);
        }

        pub fn reset_for_new_round(&mut self) {
            self.has_gone_this_round = <_>::default();
        }

        pub fn has_folded(&self, index: HandIndex) -> bool {
            self.has_folded_i(usize::from(index))
        }

        fn has_folded_i(&self, index: usize) -> bool {
            self.actions[index].iter().any(|a| *a == PotAction::Fold)
        }
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum RoundOutcome {
        Undetermined,
        AdvanceToNext,
        /// This happens if everyone but one player folds.
        AwardNow(HandIndex),
    }

    impl Pot {
        pub fn round_outcome(&self, current_money: &PerPlayer<Money>) -> RoundOutcome {
            use RoundOutcome::*;

            if self.has_gone_this_round.len() < self.player_count.amount() {
                // Rounds aren't complete until everyone has had a chance to move.
                return Undetermined;
            }

            let amounts = self.amounts();
            let mut previous_amount = None;
            for i in 0..amounts.len() {
                // A player is all in or not playing, if they have 0 money left.
                if current_money[i] == 0 {
                    continue;
                }

                if self.has_folded_i(i) {
                    continue;
                }

                if let Some(previous) = previous_amount {
                    if previous != amounts[i] {
                        return Undetermined;
                    }
                } else {
                    previous_amount = Some(amounts[i]);
                }
            }

            let call_amount = self.call_amount();
            let is_complete = previous_amount
                .map(|amount| amount >= call_amount)
                // If everyone is all-in, then the round is done.
                .unwrap_or(true);

            if is_complete {
                // TODO? avoid this seemingly extra loop?
                let mut unfolded_count = 0;
                let mut last_unfolded_index = 0;

                for index in 0..MAX_HAND_INDEX {
                    let i = usize::from(index);

                    if amounts[i] == 0
                    && current_money[i] == 0 {
                        // All in with nothing? Must not be playing.
                        continue;
                    }

                    if !self.has_folded(index) {
                        unfolded_count += 1;
                        last_unfolded_index = index;
                    }
                }

                if unfolded_count == 1 {
                    AwardNow(last_unfolded_index)
                } else {
                    AdvanceToNext
                }
            } else {
                Undetermined
            }
        }

        pub fn total(&self) -> Money {
            self.amounts()
                .iter()
                .sum()
        }

        /// Returns the total that a player must have bet, in order to call 
        /// including previous bets. That is, the maxiumum amount bet by any player.
        pub fn call_amount(&self) -> Money {
            self.amounts()
                .iter()
                .max()
                .copied()
                .unwrap_or_default()
        }

        pub fn eligibilities(
            &self,
            current_money: &PerPlayer<Money>,
        ) -> impl Iterator<Item = (PerPlayerBitset, Money)> {
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

                    let mut contributors = PerPlayerBitset::default();
                    let mut output: Money = 0;
                    for index in 0..MAX_PLAYERS {
                        let i = usize::from(index);
                        if amounts[i] > 0 {
                            contributors.set(index);
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

                    if contributors.len() > 0 && output != 0 {
                        return Some((contributors, output))
                    }
                }
            })
        }

        pub fn individual_pots(
            &self,
            current_money: &PerPlayer<Money>,
        ) -> impl Iterator<Item = Money> {
            self.eligibilities(current_money).filter(
                |(contributors, money)| {
                    // Side pots with one player in them are "trivial" and not
                    // desired to be returned
                    contributors.len() > 1 && *money != 0
                }
            )
            .map(|(_, money)| money)
        }

        pub fn amount_for(&self, index: HandIndex) -> Money {
            // TODO? Avoid calculating the other players' amounts here?
            self.amounts()[usize::from(index)]
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
    mod call_amount_works {
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

                let mut index = 0;
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

                    index += 1;
                }

                let actual: Money = pot.call_amount();

                assert_eq!(actual, expected);
            }
        }

        #[test]
        fn on_these_examples() {
            a!([bet(5), bet(10)], 10);
            a!([all_in(300), all_in(500)], 500);
            a!([all_in(300), all_in(500), all_in(800)], 800);
            a!([all_in(800), all_in(500), all_in(300)], 800);
            a!([all_in(500), all_in(300), all_in(800)], 800);
            a!([all_in(300), all_in(500), bet(800)], 800);
            a!([all_in(300), all_in(500), bet(800), bet(800)], 800);
            a!([all_in(300), all_in(500), bet(900), bet(900)], 900);
        }
    }

    #[cfg(test)]
    mod individual_pots_works {
        use super::*;
        #[derive(Debug)]
        struct Spec {
            action: PotAction,
            is_all_in: bool,
        }

        fn bet(bet: Money) -> Spec {
            Spec {
                action: PotAction::Bet(bet),
                is_all_in: false,
            }
        }

        fn all_in(bet: Money) -> Spec {
            Spec {
                action: PotAction::Bet(bet),
                is_all_in: true,
            }
        }

        fn fold() -> Spec {
            Spec {
                action: PotAction::Fold,
                is_all_in: false,
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
                        spec.action,
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
            a!([bet(5), bet(10), fold()], [15]);
            a!([all_in(300), fold(), all_in(500), fold(), bet(800)], [900, 400]);
        }
    }

    #[cfg(test)]
    mod is_round_complete_works {
        use super::{*, RoundOutcome::*};

        #[derive(Debug)]
        struct Spec {
            action: PotAction,
            is_all_in: bool,
        }

        fn bet(bet: Money) -> Spec {
            Spec {
                action: PotAction::Bet(bet),
                is_all_in: false,
            }
        }

        fn all_in(bet: Money) -> Spec {
            Spec {
                action: PotAction::Bet(bet),
                is_all_in: true,
            }
        }

        fn fold() -> Spec {
            Spec {
                action: PotAction::Fold,
                is_all_in: false,
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
                        spec.action,
                    );

                    moneys[i] = if spec.is_all_in {
                        0
                    } else {
                        1
                    };
                }

                let actual = pot.round_outcome(&moneys);

                assert_eq!(actual, expected, "{specs:?}");
            }
        }

        #[test]
        fn on_these_examples() {
            a!([bet(5), bet(10)], Undetermined);
            a!([all_in(300), all_in(500)], AdvanceToNext);
            a!([all_in(300), all_in(500), all_in(800)], AdvanceToNext);
            a!([all_in(800), all_in(500), all_in(300)], AdvanceToNext);
            a!([all_in(500), all_in(300), all_in(800)], AdvanceToNext);
            a!([all_in(300), all_in(500), bet(800)], AdvanceToNext);
            a!([all_in(300), all_in(500), bet(300)], Undetermined);
            a!([all_in(300), all_in(500), bet(800), bet(800)], AdvanceToNext);
            a!([all_in(300), all_in(500), bet(900), bet(900)], AdvanceToNext);
            a!([bet(5), bet(10), fold()], Undetermined);
            a!([all_in(300), fold(), all_in(500), fold(), bet(800)], AdvanceToNext);
            a!([all_in(300), fold(), fold()], AwardNow(0));
            a!([fold(), all_in(300), fold()], AwardNow(1));
            a!([fold(), fold(), all_in(300)], AwardNow(2));
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

        pub fn deal_community_cards(&mut self) -> Option<CommunityCards> {
            self.burn();
            let [Some(card1), Some(card2), Some(card3)] =
                [self.draw(), self.draw(), self.draw()]
                else {
                    return None
                };
            Some(CommunityCards::Flop([card1, card2, card3]))
        }

        pub fn deal_to_community_cards(
            &mut self,
            community_cards: &mut CommunityCards
        ) {
            match *community_cards {
                CommunityCards::Flop(flop) => {
                    self.burn();
                    if let Some(turn) = self.draw() {
                        *community_cards = CommunityCards::Turn(flop, turn);
                    } else {
                        debug_assert!(false, "Ran out of cards for turn!");
                    }
                },
                CommunityCards::Turn(flop, turn) => {
                    self.burn();
                    if let Some(river) = self.draw() {
                        *community_cards = CommunityCards::River(flop, turn, river);
                    } else {
                        debug_assert!(false, "Ran out of cards for river!");
                    }
                }
                CommunityCards::River(..) => {
                    // Nothing left to deal out.
                }
            }
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

    impl CommunityCards {
        pub fn contains(&self, card: Card) -> bool {
            match *self {
                Self::Flop([c1, c2, c3]) =>
                    c1 == card
                    || c2 == card
                    || c3 == card,
                Self::Turn([c1, c2, c3], c4) =>
                    c1 == card
                    || c2 == card
                    || c3 == card
                    || c4 == card,
                Self::River([c1, c2, c3], c4, c5) =>
                    c1 == card
                    || c2 == card
                    || c3 == card
                    || c4 == card
                    || c5 == card,
            }
        }
    }

    pub type FullBoard = [Card; 5];

    impl From<FullBoard> for CommunityCards {
        fn from(full_board: FullBoard) -> Self {
            Self::River(
                [
                    full_board[0],
                    full_board[1],
                    full_board[2],
                ],
                full_board[3],
                full_board[4],
            )
        }
    }
}

