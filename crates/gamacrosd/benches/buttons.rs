use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use gamacros_control::KeyCombo;
use gamacros_gamepad::{Button, ControllerId, ControllerInfo};
use gamacros_workspace::{AppRules, ButtonAction, ButtonRule, Profile, StickRules};
use gamacrosd::app::{Action, Gamacros};
use std::sync::Arc;

fn build_profile_simple(button: Button, combo: KeyCombo) -> Profile {
    let mut rules = gamacros_workspace::RuleMap::default();
    let mut app = AppRules::default();
    let mut buttons = gamacros_workspace::ButtonRules::default();
    let mut chord = gamacros_bit_mask::Bitmask::empty();
    chord.insert(button);
    buttons.insert(
        chord,
        ButtonRule {
            action: ButtonAction::Keystroke(Arc::new(combo)),
            vibrate: None,
        },
    );
    app.buttons = buttons;
    app.sticks = StickRules::default();
    rules.insert("bench.app".into(), app);
    Profile {
        controllers: Default::default(),
        blacklist: Default::default(),
        rules,
        shell: None,
    }
}

pub fn bench_button_path(c: &mut Criterion) {
    let mut g = Gamacros::new();
    let profile = build_profile_simple(
        Button::A,
        KeyCombo::from_key(gamacros_control::Key::F1),
    );
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
    let button = Button::A;

    c.bench_function("buttons_press_release_single", |b| {
        b.iter(|| {
            let mut sink_count = 0usize;
            g.on_button_with(
                id,
                button,
                gamacrosd::app::ButtonPhase::Pressed,
                |a| {
                    match a {
                        Action::KeyPress(_)
                        | Action::Rumble { .. }
                        | Action::Shell(_)
                        | Action::Macros(_)
                        | Action::MouseMove { .. }
                        | Action::Scroll { .. }
                        | Action::KeyTap(_)
                        | Action::KeyRelease(_) => {
                            sink_count += 1;
                        }
                    };
                    black_box(());
                },
            );
            g.on_button_with(
                id,
                button,
                gamacrosd::app::ButtonPhase::Released,
                |a| {
                    match a {
                        Action::KeyPress(_)
                        | Action::Rumble { .. }
                        | Action::Shell(_)
                        | Action::Macros(_)
                        | Action::MouseMove { .. }
                        | Action::Scroll { .. }
                        | Action::KeyTap(_)
                        | Action::KeyRelease(_) => {
                            sink_count += 1;
                        }
                    };
                    black_box(());
                },
            );
            black_box(sink_count)
        })
    });
}

criterion_group!(benches, bench_button_path);
criterion_main!(benches);
