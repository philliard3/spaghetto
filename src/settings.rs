use std::fmt::Debug;

#[derive(Copy, Clone, Debug, Default)]
pub struct FrontToBack;

#[derive(Copy, Clone, Debug, Default)]
pub struct BackToFront;

pub trait DropBehavior: seal_drop_behavior::Sealed + Debug + Copy + Default {}
pub(crate) mod seal_drop_behavior {
    pub trait Sealed {
        const IS_INVERTED: bool;
    }
}

impl DropBehavior for FrontToBack {}
impl DropBehavior for BackToFront {}

impl seal_drop_behavior::Sealed for FrontToBack {
    const IS_INVERTED: bool = false;
}
impl seal_drop_behavior::Sealed for BackToFront {
    const IS_INVERTED: bool = true;
}

pub enum RebalanceStrategy {
    StartAtFront,
    Middle,
    FavorCrowdedSide,
    OnlyChangeCrowdedSide,
}
pub(crate) mod seal_rebalance_behavior {
    pub trait Sealed {
        const BEHAVIOR: super::RebalanceStrategy;
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct StartAtFront;

#[derive(Copy, Clone, Debug, Default)]
pub struct Middle;

#[derive(Copy, Clone, Debug, Default)]
pub struct FavorCrowdedSide;

#[derive(Copy, Clone, Debug, Default)]
pub struct OnlyChangeCrowdedSide;

pub trait RebalanceBehavior: seal_rebalance_behavior::Sealed + Debug + Copy + Default {}

impl seal_rebalance_behavior::Sealed for StartAtFront {
    const BEHAVIOR: RebalanceStrategy = RebalanceStrategy::StartAtFront;
}
impl RebalanceBehavior for StartAtFront {}

impl seal_rebalance_behavior::Sealed for Middle {
    const BEHAVIOR: RebalanceStrategy = RebalanceStrategy::Middle;
}
impl RebalanceBehavior for Middle {}

impl seal_rebalance_behavior::Sealed for FavorCrowdedSide {
    const BEHAVIOR: RebalanceStrategy = RebalanceStrategy::FavorCrowdedSide;
}
impl RebalanceBehavior for FavorCrowdedSide {}

impl seal_rebalance_behavior::Sealed for OnlyChangeCrowdedSide {
    const BEHAVIOR: RebalanceStrategy = RebalanceStrategy::OnlyChangeCrowdedSide;
}
impl RebalanceBehavior for OnlyChangeCrowdedSide {}
