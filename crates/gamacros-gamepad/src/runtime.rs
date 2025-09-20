use std::sync::Arc;
use std::thread;

use crossbeam_channel::Receiver;
use ahash::AHashMap;
use sdl2::controller::{Button as SdlButton, GameController, Axis as SdlAxis};
use sdl2::event::Event;
use sdl2::haptic::Haptic;
use sdl2::joystick::Joystick;

use crate::command::Command;
use crate::events::ControllerEvent;
use crate::manager::Inner;
use crate::types::{Button, ControllerId, ControllerInfo, Axis};

/// Starts the SDL2-backed runtime thread that drives device discovery and events.
pub(crate) fn start_runtime_thread(
    inner: Arc<Inner>,
    cmd_rx: Receiver<Command>,
    ready_tx: Option<std::sync::mpsc::Sender<()>>,
) {
    thread::spawn(move || {
        // SDL must live entirely within this thread
        let sdl_ctx = match sdl2::init() {
            Ok(ctx) => ctx,
            Err(_) => {
                return;
            }
        };
        let controller_subsystem = match sdl_ctx.game_controller() {
            Ok(c) => c,
            Err(_) => return,
        };
        let joystick_subsystem = match sdl_ctx.joystick() {
            Ok(j) => j,
            Err(_) => return,
        };
        let haptic_subsystem = match sdl_ctx.haptic() {
            Ok(h) => h,
            Err(_) => return,
        };
        let mut event_pump = match sdl_ctx.event_pump() {
            Ok(p) => p,
            Err(_) => return,
        };

        let mut controllers: AHashMap<ControllerId, GameController> =
            AHashMap::new();
        let mut joysticks: AHashMap<ControllerId, Joystick> = AHashMap::new();
        let mut haptics: AHashMap<ControllerId, Haptic> = AHashMap::new();
        let mut trigger_state: AHashMap<ControllerId, (bool, bool)> =
            AHashMap::new();

        // Initial enumeration
        if let Ok(num_joysticks) = joystick_subsystem.num_joysticks() {
            for i in 0..num_joysticks {
                if controller_subsystem.is_game_controller(i) {
                    if let Ok(controller) = controller_subsystem.open(i) {
                        let id: ControllerId = match joystick_subsystem.open(i) {
                            Ok(js) => js.instance_id() as ControllerId,
                            Err(_) => i as ControllerId,
                        };
                        let info = ControllerInfo {
                            id,
                            name: controller.name().to_string(),
                            vendor_id: controller.vendor_id().unwrap_or(0),
                            product_id: controller.product_id().unwrap_or(0),
                            supports_rumble: controller.has_rumble(),
                        };
                        controllers.insert(id, controller);
                        if let Ok(mut map) = inner.controllers_info.write() {
                            map.insert(id, info.clone());
                        }
                        broadcast(&inner, ControllerEvent::Connected(info));
                    }
                } else if let Ok(joystick) = joystick_subsystem.open(i) {
                    let id: ControllerId = joystick.instance_id() as ControllerId;
                    if joystick.has_rumble() {
                        if let Ok(h) = haptic_subsystem
                            .open_from_joystick_id(joystick.instance_id())
                        {
                            haptics.insert(id, h);
                        }
                    }
                    let info = ControllerInfo {
                        id,
                        name: joystick.name().to_string(),
                        vendor_id: 0,
                        product_id: 0,
                        supports_rumble: joystick.has_rumble(),
                    };
                    joysticks.insert(id, joystick);
                    if let Ok(mut map) = inner.controllers_info.write() {
                        map.insert(id, info.clone());
                    }
                    broadcast(&inner, ControllerEvent::Connected(info));
                }
            }
        }

        if let Some(tx) = ready_tx {
            let _ = tx.send(());
        }

        loop {
            // Wait for an SDL event or timeout to reduce idle CPU usage
            if let Some(event) = event_pump.wait_event_timeout(10) {
                match event {
                    Event::ControllerDeviceAdded { which, .. } => {
                        if let Ok(controller) = controller_subsystem.open(which) {
                            let id: ControllerId =
                                match joystick_subsystem.open(which) {
                                    Ok(js) => js.instance_id() as ControllerId,
                                    Err(_) => which as ControllerId,
                                };
                            let info = ControllerInfo {
                                id,
                                name: controller.name().to_string(),
                                vendor_id: controller.vendor_id().unwrap_or(0),
                                product_id: controller.product_id().unwrap_or(0),
                                supports_rumble: controller.has_rumble(),
                            };
                            controllers.insert(id, controller);
                            if let Ok(mut map) = inner.controllers_info.write() {
                                map.insert(id, info.clone());
                            }
                            broadcast(&inner, ControllerEvent::Connected(info));
                        }
                    }
                    Event::ControllerDeviceRemoved { which, .. } => {
                        let id: ControllerId = which as ControllerId;
                        controllers.remove(&id);
                        joysticks.remove(&id);
                        haptics.remove(&id);
                        trigger_state.remove(&id);
                        if let Ok(mut map) = inner.controllers_info.write() {
                            map.remove(&id);
                        }
                        broadcast(&inner, ControllerEvent::Disconnected(id));
                    }
                    Event::ControllerButtonDown { which, button, .. } => {
                        if let Some(btn) = map_sdl_button(button) {
                            broadcast(
                                &inner,
                                ControllerEvent::ButtonPressed {
                                    id: which as ControllerId,
                                    button: btn,
                                },
                            );
                        }
                    }
                    Event::ControllerButtonUp { which, button, .. } => {
                        if let Some(btn) = map_sdl_button(button) {
                            broadcast(
                                &inner,
                                ControllerEvent::ButtonReleased {
                                    id: which as ControllerId,
                                    button: btn,
                                },
                            );
                        }
                    }
                    Event::ControllerAxisMotion {
                        which, axis, value, ..
                    } => {
                        const THRESHOLD: i16 = 20000;
                        let id = which as ControllerId;
                        let entry =
                            trigger_state.entry(id).or_insert((false, false));

                        // Emit analog event for all axes
                        if let Some(mapped) = map_sdl_axis(axis) {
                            let norm = (value as f32) / (i16::MAX as f32);
                            broadcast(
                                &inner,
                                ControllerEvent::AxisMotion {
                                    id,
                                    axis: mapped,
                                    value: norm,
                                },
                            );
                        }

                        // Preserve trigger-as-button semantics for compatibility
                        match axis {
                            SdlAxis::TriggerLeft => {
                                let pressed = value > THRESHOLD;
                                if pressed && !entry.0 {
                                    broadcast(
                                        &inner,
                                        ControllerEvent::ButtonPressed {
                                            id,
                                            button: Button::LeftTrigger,
                                        },
                                    );
                                    entry.0 = true;
                                } else if !pressed && entry.0 {
                                    broadcast(
                                        &inner,
                                        ControllerEvent::ButtonReleased {
                                            id,
                                            button: Button::LeftTrigger,
                                        },
                                    );
                                    entry.0 = false;
                                }
                            }
                            SdlAxis::TriggerRight => {
                                let pressed = value > THRESHOLD;
                                if pressed && !entry.1 {
                                    broadcast(
                                        &inner,
                                        ControllerEvent::ButtonPressed {
                                            id,
                                            button: Button::RightTrigger,
                                        },
                                    );
                                    entry.1 = true;
                                } else if !pressed && entry.1 {
                                    broadcast(
                                        &inner,
                                        ControllerEvent::ButtonReleased {
                                            id,
                                            button: Button::RightTrigger,
                                        },
                                    );
                                    entry.1 = false;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
                // Drain any additional queued events quickly
                for ev in event_pump.poll_iter() {
                    match ev {
                        Event::ControllerDeviceAdded { which, .. } => {
                            if let Ok(controller) = controller_subsystem.open(which)
                            {
                                let id: ControllerId =
                                    match joystick_subsystem.open(which) {
                                        Ok(js) => js.instance_id() as ControllerId,
                                        Err(_) => which as ControllerId,
                                    };
                                let info = ControllerInfo {
                                    id,
                                    name: controller.name().to_string(),
                                    vendor_id: controller.vendor_id().unwrap_or(0),
                                    product_id: controller.product_id().unwrap_or(0),
                                    supports_rumble: controller.has_rumble(),
                                };
                                controllers.insert(id, controller);
                                if let Ok(mut map) = inner.controllers_info.write() {
                                    map.insert(id, info.clone());
                                }
                                broadcast(&inner, ControllerEvent::Connected(info));
                            }
                        }
                        Event::ControllerDeviceRemoved { which, .. } => {
                            let id: ControllerId = which as ControllerId;
                            controllers.remove(&id);
                            joysticks.remove(&id);
                            haptics.remove(&id);
                            trigger_state.remove(&id);
                            if let Ok(mut map) = inner.controllers_info.write() {
                                map.remove(&id);
                            }
                            broadcast(&inner, ControllerEvent::Disconnected(id));
                        }
                        Event::ControllerButtonDown { which, button, .. } => {
                            if let Some(btn) = map_sdl_button(button) {
                                broadcast(
                                    &inner,
                                    ControllerEvent::ButtonPressed {
                                        id: which as ControllerId,
                                        button: btn,
                                    },
                                );
                            }
                        }
                        Event::ControllerButtonUp { which, button, .. } => {
                            if let Some(btn) = map_sdl_button(button) {
                                broadcast(
                                    &inner,
                                    ControllerEvent::ButtonReleased {
                                        id: which as ControllerId,
                                        button: btn,
                                    },
                                );
                            }
                        }
                        Event::ControllerAxisMotion {
                            which, axis, value, ..
                        } => {
                            const THRESHOLD: i16 = 20000;
                            let id = which as ControllerId;
                            let entry =
                                trigger_state.entry(id).or_insert((false, false));
                            if let Some(mapped) = map_sdl_axis(axis) {
                                let norm = (value as f32) / (i16::MAX as f32);
                                broadcast(
                                    &inner,
                                    ControllerEvent::AxisMotion {
                                        id,
                                        axis: mapped,
                                        value: norm,
                                    },
                                );
                            }
                            match axis {
                                SdlAxis::TriggerLeft => {
                                    let pressed = value > THRESHOLD;
                                    if pressed && !entry.0 {
                                        broadcast(
                                            &inner,
                                            ControllerEvent::ButtonPressed {
                                                id,
                                                button: Button::LeftTrigger,
                                            },
                                        );
                                        entry.0 = true;
                                    } else if !pressed && entry.0 {
                                        broadcast(
                                            &inner,
                                            ControllerEvent::ButtonReleased {
                                                id,
                                                button: Button::LeftTrigger,
                                            },
                                        );
                                        entry.0 = false;
                                    }
                                }
                                SdlAxis::TriggerRight => {
                                    let pressed = value > THRESHOLD;
                                    if pressed && !entry.1 {
                                        broadcast(
                                            &inner,
                                            ControllerEvent::ButtonPressed {
                                                id,
                                                button: Button::RightTrigger,
                                            },
                                        );
                                        entry.1 = true;
                                    } else if !pressed && entry.1 {
                                        broadcast(
                                            &inner,
                                            ControllerEvent::ButtonReleased {
                                                id,
                                                button: Button::RightTrigger,
                                            },
                                        );
                                        entry.1 = false;
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Handle commands
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    Command::Rumble { id, low, high, ms } => {
                        if let Some(ctrl) = controllers.get_mut(&id) {
                            if let Err(e) = ctrl.set_rumble(low, high, ms) {
                                println!("{}", ctrl.has_rumble());
                                eprintln!("Failed to set rumble: {e}");
                            }
                        } else if let Some(h) = haptics.get_mut(&id) {
                            let strength = (low.max(high) as f32) / 65535.0;
                            h.rumble_play(strength, ms);
                        }
                    }
                    Command::StopRumble { id } => {
                        if let Some(ctrl) = controllers.get_mut(&id) {
                            if let Err(e) = ctrl.set_rumble(0, 0, 0) {
                                eprintln!("Failed to stop rumble: {e}");
                            }
                        } else if let Some(h) = haptics.get_mut(&id) {
                            h.rumble_stop();
                        }
                    }
                }
            }
        }
    });
}

fn map_sdl_button(button: SdlButton) -> Option<Button> {
    Some(match button {
        SdlButton::A => Button::A,
        SdlButton::B => Button::B,
        SdlButton::X => Button::X,
        SdlButton::Y => Button::Y,
        SdlButton::Back => Button::Back,
        SdlButton::Guide => Button::Guide,
        SdlButton::Start => Button::Start,
        SdlButton::LeftStick => Button::LeftStick,
        SdlButton::RightStick => Button::RightStick,
        SdlButton::LeftShoulder => Button::LeftShoulder,
        SdlButton::RightShoulder => Button::RightShoulder,
        SdlButton::DPadUp => Button::DPadUp,
        SdlButton::DPadDown => Button::DPadDown,
        SdlButton::DPadLeft => Button::DPadLeft,
        SdlButton::DPadRight => Button::DPadRight,
        _ => return None,
    })
}

fn map_sdl_axis(axis: SdlAxis) -> Option<Axis> {
    Some(match axis {
        SdlAxis::LeftX => Axis::LeftX,
        SdlAxis::LeftY => Axis::LeftY,
        SdlAxis::RightX => Axis::RightX,
        SdlAxis::RightY => Axis::RightY,
        SdlAxis::TriggerLeft => Axis::LeftTrigger,
        SdlAxis::TriggerRight => Axis::RightTrigger,
    })
}

fn broadcast(inner: &Inner, event: ControllerEvent) {
    if let Ok(mut subs) = inner.subscribers.lock() {
        subs.retain(|tx| tx.send(event.clone()).is_ok());
    }
}
