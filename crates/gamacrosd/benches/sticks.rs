use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use gamacros_gamepad::{Axis as CtrlAxis, ControllerId, ControllerInfo};
use gamacros_workspace::{
    AppRules, Profile, StickMode, StickRules, ArrowsParams, StickSide,
};
use gamacrosd::app::{Action, Gamacros};

fn build_profile_arrows() -> Profile {
    let mut rules = gamacros_workspace::RuleMap::default();
    let mut app = AppRules::default();
    let mut sticks = StickRules::default();
    sticks.insert(
        StickSide::Left,
        StickMode::Arrows(ArrowsParams {
            deadzone: 0.2,
            repeat_delay_ms: 200,
            repeat_interval_ms: 40,
            invert_x: false,
            invert_y: false,
        }),
    );
    app.sticks = sticks;
    rules.insert("bench.app".into(), app);
    Profile {
        controllers: Default::default(),
        blacklist: Default::default(),
        rules,
        shell: None,
    }
}

#[allow(clippy::approx_constant)]
pub fn bench_sticks_arrows(c: &mut Criterion) {
    let mut g = Gamacros::new();
    let profile = build_profile_arrows();
    g.set_workspace(profile);
    g.set_active_app("bench.app");
    let id: ControllerId = 1;
    g.add_controller(ControllerInfo {
        id,
        name: "bench".to_string(),
        supports_rumble: false,
        vendor_id: 0,
        product_id: 0,
    });

    // Simulate diagonal movement around unit circle
    c.bench_function("sticks_arrows_tick", |b| {
        b.iter(|| {
            for t in 0..16u32 {
                let angle = (t as f32) * 0.3926991; // ~22.5 deg steps
                let x = angle.cos();
                let y = angle.sin();
                g.on_axis_motion(id, CtrlAxis::LeftX, x);
                g.on_axis_motion(id, CtrlAxis::LeftY, y);
                let mut n = 0usize;
                g.on_tick_with(|a| {
                    {
                        match a {
                            Action::KeyTap(_)
                            | Action::MouseMove { .. }
                            | Action::Scroll { .. }
                            | Action::KeyPress(_)
                            | Action::KeyRelease(_)
                            | Action::Rumble { .. }
                            | Action::Shell(_)
                            | Action::Macros(_) => {
                                n += 1;
                            }
                        };
                        black_box(())
                    };
                });
                black_box(n);
            }
        })
    });
}

criterion_group!(benches, bench_sticks_arrows);
criterion_main!(benches);
