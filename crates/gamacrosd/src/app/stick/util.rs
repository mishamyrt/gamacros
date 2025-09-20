use gamacros_gamepad::Axis as CtrlAxis;
use gamacros_workspace::StickSide;

#[inline]
pub(crate) fn axis_index(axis: CtrlAxis) -> usize {
    match axis {
        CtrlAxis::LeftX => 0,
        CtrlAxis::LeftY => 1,
        CtrlAxis::RightX => 2,
        CtrlAxis::RightY => 3,
        CtrlAxis::LeftTrigger => 4,
        CtrlAxis::RightTrigger => 5,
    }
}

#[inline]
pub(crate) fn side_index(side: &StickSide) -> usize {
    match side {
        StickSide::Left => 0,
        StickSide::Right => 1,
    }
}

#[inline]
pub(crate) fn axes_for_side(axes: [f32; 6], side: &StickSide) -> (f32, f32) {
    match side {
        StickSide::Left => (
            axes[axis_index(CtrlAxis::LeftX)],
            axes[axis_index(CtrlAxis::LeftY)],
        ),
        StickSide::Right => (
            axes[axis_index(CtrlAxis::RightX)],
            axes[axis_index(CtrlAxis::RightY)],
        ),
    }
}

#[inline]
pub(crate) fn invert_xy(
    x: f32,
    y: f32,
    invert_x: bool,
    invert_y: bool,
) -> (f32, f32) {
    let nx = if invert_x { -x } else { x };
    let ny = if invert_y { -y } else { y };
    (nx, ny)
}

#[inline]
pub(crate) fn magnitude2d(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

#[inline]
pub(crate) fn normalize_after_deadzone(mag: f32, deadzone: f32) -> f32 {
    if mag <= deadzone {
        0.0
    } else {
        ((mag - deadzone) / (1.0 - deadzone)).clamp(0.0, 1.0)
    }
}
