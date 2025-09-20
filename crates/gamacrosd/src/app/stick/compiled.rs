use gamacros_workspace::{StickMode, StickRules, StickSide};

#[derive(Debug, Clone, Default)]
pub struct CompiledStickRules {
    pub(super) sides: [Option<StickMode>; 2],
}

impl CompiledStickRules {
    pub fn from_rules(rules: &StickRules) -> Self {
        let mut sides: [Option<StickMode>; 2] = [None, None];
        if let Some(mode) = rules.get(&StickSide::Left) {
            sides[0] = Some(mode.clone());
        }
        if let Some(mode) = rules.get(&StickSide::Right) {
            sides[1] = Some(mode.clone());
        }
        Self { sides }
    }

    #[inline]
    pub fn left(&self) -> Option<&StickMode> {
        self.sides[0].as_ref()
    }

    #[inline]
    pub fn right(&self) -> Option<&StickMode> {
        self.sides[1].as_ref()
    }
}
