mod compiled;
mod repeat;
mod tick;
pub(crate) mod util;

pub(crate) use compiled::CompiledStickRules;
pub(crate) use repeat::StickProcessor;

#[derive(Clone, Copy)]
pub(super) enum StepperMode {
    Volume,
    Brightness,
}

impl StepperMode {
    pub(super) fn key_for(&self, positive: bool) -> gamacros_control::Key {
        match self {
            StepperMode::Volume => {
                if positive {
                    gamacros_control::Key::VolumeUp
                } else {
                    gamacros_control::Key::VolumeDown
                }
            }
            #[cfg(target_os = "macos")]
            StepperMode::Brightness => {
                if positive {
                    gamacros_control::Key::BrightnessUp
                } else {
                    gamacros_control::Key::BrightnessDown
                }
            }
            #[cfg(not(target_os = "macos"))]
            StepperMode::Brightness => {
                unimplemented!()
            }
        }
    }
    pub(super) fn kind_for(
        &self,
        axis: gamacros_workspace::Axis,
        positive: bool,
    ) -> repeat::RepeatKind {
        match self {
            StepperMode::Volume => repeat::RepeatKind::Volume { axis, positive },
            StepperMode::Brightness => {
                repeat::RepeatKind::Brightness { axis, positive }
            }
        }
    }
}
