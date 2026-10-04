use shr_lightdesk::{
    model::{Attribute, Authority},
    operator::{
        Assignment, DeskRole, Event, INPUT_CAPACITY, LedState, Operator, OperatorInput, PUMP_BUDGET,
    },
    render::fixture_rect,
    simulator::Simulator,
    surface::{Page, ResultState, Surface, actions::Action},
};
fn rig() -> (Operator, Simulator) {
    let sim = Simulator::new();
    (Operator::new(Surface::new(sim.snapshot())), sim)
}
fn assign(o: &mut Operator, generation: u64, role: DeskRole) {
    o.event(Event::Assignment(Assignment {
        generation,
        endpoint: "test-controller-a".into(),
        role,
    }))
    .unwrap();
}
fn midi(generation: u64, bytes: [u8; 3]) -> OperatorInput {
    OperatorInput::Midi {
        generation,
        endpoint: "test-controller-a".into(),
        bytes,
    }
}
fn key(o: &mut Operator, action: Action) {
    o.enqueue(OperatorInput::Keyboard(action)).unwrap();
}
#[test]
fn e02_role_observation_and_generation_fence_both_workers() {
    let (mut o, sim) = rig();
    // Exact GP01 provider example bytes retained; this is a pure consumer model,
    // not a wire decoder or proof of native endpoint ownership.
    assert!(include_str!("fixtures/gp01/e02.json").contains("\"audio-desk\""));
    assign(&mut o, 7, DeskRole::Audio);
    let before = o.assignment().cloned();
    assert!(
        o.event(Event::Assignment(Assignment {
            generation: 7,
            endpoint: "test-controller-a".into(),
            role: DeskRole::Lighting
        }))
        .is_err()
    );
    assert_eq!(o.assignment(), before.as_ref());
    assert!(o.event(Event::DeviceLost { generation: 6 }).is_err());
    assert!(o.enqueue(midi(7, [0x90, 50, 100])).is_err());
    assert!(o.take_led(0).is_none());
    assign(&mut o, 8, DeskRole::Lighting);
    o.enqueue(midi(8, [0x80, 50, 0])).unwrap();
    o.enqueue(midi(8, [0x90, 50, 100])).unwrap();
    o.event(Event::DeviceLost { generation: 8 }).unwrap();
    assign(&mut o, 9, DeskRole::Lighting);
    assert_eq!(o.pump(&sim).processed, 0);
    assert!(o.enqueue(midi(8, [0x90, 50, 100])).is_err());
    assert_eq!(o.take_led(0).unwrap().generation, 9);
    o.enqueue(midi(9, [0x90, 50, 100])).unwrap();
    o.pump(&sim);
    assert!(o.surface.selected.is_empty());
    o.enqueue(midi(9, [0x80, 50, 0])).unwrap();
    o.enqueue(midi(9, [0x90, 50, 100])).unwrap();
    o.pump(&sim);
    assert_eq!(
        o.surface.selected.iter().copied().collect::<Vec<_>>(),
        vec![11]
    );
}
#[test]
fn saturation_is_visible_bounded_and_requires_release_and_pickup() {
    let (mut o, sim) = rig();
    assign(&mut o, 1, DeskRole::Lighting);
    o.surface.select(&[11]).unwrap();
    o.surface
        .action(Action::Edit(Attribute::Intensity))
        .unwrap();
    o.enqueue(midi(1, [0xb0, 16, 0])).unwrap();
    o.pump(&sim);
    assert!(o.surface.controller.pickup[0].acquired);
    for _ in 0..INPUT_CAPACITY {
        key(&mut o, Action::Navigate(1));
    }
    assert!(o.enqueue(midi(1, [0x99, 40, 100])).is_err());
    assert_eq!(o.queued(), 0);
    assert!(o.surface.notice.contains("overflow"));
    assert!(!o.surface.controller.pickup[0].acquired);
    assert!(o.surface.modal.is_none());
    o.enqueue(midi(1, [0x99, 40, 100])).unwrap();
    o.pump(&sim);
    assert!(!o.surface.additive);
    o.enqueue(midi(1, [0x89, 40, 0])).unwrap();
    o.enqueue(midi(1, [0x99, 40, 100])).unwrap();
    o.pump(&sim);
    assert!(o.surface.additive);
    for _ in 0..INPUT_CAPACITY {
        key(&mut o, Action::Navigate(1));
    }
    let p = o.pump(&sim);
    assert_eq!(p.processed, PUMP_BUDGET);
    assert_eq!(o.queued(), INPUT_CAPACITY - PUMP_BUDGET);
    assert!(p.requests.is_empty());
}
#[test]
fn authority_requests_stay_pending_and_led_mailbox_coalesces_independently() {
    let (mut o, mut sim) = rig();
    assign(&mut o, 1, DeskRole::Lighting);
    let initial = o.take_led(0).unwrap();
    key(&mut o, Action::Blackout(true));
    let p = o.pump(&sim);
    assert_eq!(p.requests.len(), 1);
    assert!(!o.surface.confirmed.blackout);
    assert!(matches!(o.surface.result, ResultState::Pending(_)));
    assert!(o.take_led(49).is_none());
    let pending = o.take_led(50).unwrap();
    assert_eq!(pending.pads, [LedState::Pending; 8]);
    assert_eq!(pending.revision, initial.revision);
    let reply = sim.request(&p.requests[0]);
    o.event(Event::Reply(reply)).unwrap();
    key(&mut o, Action::Page(Page::Health));
    o.pump(&sim);
    let latest = o.take_led(100).unwrap();
    assert_eq!(latest.pads, [LedState::Fault; 8]);
    assert!(latest.revision > pending.revision);
    assert!(matches!(latest.result, ResultState::Confirmed(_)));
    assert!(o.take_led(200).is_none());
    o.pump(&sim);
    assert!(o.take_led(250).is_none()); // unchanged desired state is not retransmitted
    key(&mut o, Action::Page(Page::Stage));
    o.pump(&sim); // LED output cannot stall input
    assert_eq!(o.surface.page, Page::Stage);
    o.event(Event::DeviceLost { generation: 1 }).unwrap();
    assert!(o.take_led(300).is_none());
}
#[test]
fn focus_and_reconnect_fence_key_edges_and_preserve_engine_state() {
    let (mut o, sim) = rig();
    assign(&mut o, 1, DeskRole::Lighting);
    key(&mut o, Action::Blackout(true));
    o.event(Event::Focus(false)).unwrap();
    assert!(
        o.enqueue(OperatorInput::Keyboard(Action::Blackout(true)))
            .is_err()
    );
    assert_eq!(o.pump(&sim).processed, 0);
    o.enqueue(midi(1, [0x80, 50, 0])).unwrap();
    o.enqueue(midi(1, [0x90, 50, 100])).unwrap();
    o.pump(&sim); // role-routed MIDI works independently of desktop keyboard focus
    assert!(o.surface.selected.contains(&11));
    o.event(Event::Focus(true)).unwrap();
    o.enqueue(OperatorInput::KeyDown {
        key: 1,
        action: Action::Blackout(true),
    })
    .unwrap();
    assert!(o.pump(&sim).requests.is_empty());
    o.enqueue(OperatorInput::KeyUp { key: 1 }).unwrap();
    o.enqueue(OperatorInput::KeyDown {
        key: 1,
        action: Action::Blackout(true),
    })
    .unwrap();
    let p = o.pump(&sim);
    assert_eq!(p.requests.len(), 1);
    o.event(Event::RendererLost).unwrap();
    assert!(o.surface.pending.is_some());
    key(&mut o, Action::ClearHold);
    o.event(Event::Reconnect(sim.snapshot())).unwrap();
    assert_eq!(o.queued(), 0);
    assert!(o.surface.pending.is_none());
    assert!(o.enqueue(midi(1, [0xb0, 16, 0])).is_err());
    assert!(
        o.event(Event::Assignment(Assignment {
            generation: 1,
            endpoint: "test-controller-a".into(),
            role: DeskRole::Lighting
        }))
        .is_err()
    );
    assign(&mut o, 2, DeskRole::Lighting);
    assert!(!o.surface.confirmed.blackout);
}
#[test]
fn renderer_loss_resize_and_letterboxed_hits_keep_selection_continuity() {
    let (mut o, sim) = rig();
    let f = &o.surface.confirmed.fixtures[2];
    let id = f.id;
    let (x, y, _, _) = fixture_rect(f);
    let old = o.layout();
    o.enqueue(OperatorInput::Hit {
        layout: old,
        x: x + 10,
        y: y + 10,
    })
    .unwrap();
    o.event(Event::Resize {
        width: 960,
        height: 600,
    })
    .unwrap();
    assert_eq!(o.pump(&sim).refused, 1);
    assert!(o.surface.selected.is_empty());
    o.enqueue(OperatorInput::Hit {
        layout: o.layout(),
        x: (x + 10) / 2,
        y: (y + 10) / 2 + 30,
    })
    .unwrap();
    o.pump(&sim);
    assert!(o.surface.selected.contains(&id));
    let selected = o.surface.selected.clone();
    let layout = o.layout();
    o.event(Event::RendererLost).unwrap();
    assert!(o.scene().is_none());
    assert!(o.event(Event::RendererRestored { layout }).is_err());
    o.event(Event::RendererRestored { layout: o.layout() })
        .unwrap();
    assert!(o.scene().unwrap().in_bounds());
    assert_eq!(o.surface.selected, selected);
    o.event(Event::Resize {
        width: 0,
        height: 0,
    })
    .unwrap();
    assert!(o.scene().is_none());
    o.enqueue(OperatorInput::Hit {
        layout: o.layout(),
        x: 0,
        y: 0,
    })
    .unwrap();
    o.pump(&sim);
    assert_eq!(o.surface.selected, selected);
    o.event(Event::Resize {
        width: u32::MAX,
        height: u32::MAX,
    })
    .unwrap();
    o.enqueue(OperatorInput::Hit {
        layout: o.layout(),
        x: 0,
        y: 0,
    })
    .unwrap();
    o.pump(&sim);
    assert_eq!(o.surface.selected, selected);
}

#[test]
fn keyboard_focus_transitions_preserve_queued_controller_blackout_and_edges() {
    for (before, after) in [(true, false), (false, true)] {
        let (mut o, sim) = rig();
        assign(&mut o, 1, DeskRole::Lighting);
        // Initial device assignment requires release. No release is supplied at focus change.
        o.enqueue(midi(1, [0x89, 38, 0])).unwrap();
        o.enqueue(midi(1, [0x89, 40, 0])).unwrap();
        o.pump(&sim);
        o.event(Event::Focus(before)).unwrap();
        o.surface.menu = true;
        o.enqueue(midi(1, [0x99, 40, 100])).unwrap();
        o.event(Event::Focus(after)).unwrap();
        let p = o.pump(&sim);
        assert_eq!(p.processed, 1);
        assert_eq!(p.requests.len(), 1);
        assert!(matches!(
            p.requests[0].command,
            shr_lightdesk::model::Command::Blackout(true)
        ));
        o.enqueue(midi(1, [0x99, 40, 100])).unwrap();
        assert!(o.pump(&sim).requests.is_empty()); // held edge survives
    }
}

#[test]
fn pad_feedback_matches_base_menu_modal_semantics() {
    use shr_lightdesk::surface::actions::PadAction;
    let (mut o, sim) = rig();
    assign(&mut o, 1, DeskRole::Lighting);
    for (n, page) in Page::ALL.into_iter().enumerate() {
        o.surface.action(Action::Multi(true)).unwrap();
        o.pump(&sim);
        o.take_led(n as u64 * 100);
        o.surface.action(Action::Multi(false)).unwrap();
        o.surface.action(Action::Page(page)).unwrap();
        o.pump(&sim);
        let pads = o.take_led(n as u64 * 100 + 50).unwrap().pads;
        let expected = match page {
            Page::Stage => Some(0),
            Page::Programmer => Some(1),
            Page::Playbacks => Some(2),
            Page::Library => Some(3),
            _ => None,
        };
        for (i, led) in pads.iter().enumerate() {
            assert_eq!(
                *led,
                if Some(i) == expected {
                    LedState::Selected
                } else {
                    LedState::Available
                }
            );
        }
    }
    o.surface.menu = true;
    assert_eq!(o.surface.pad_action(5), PadAction::Page(Page::Health));
    assert_eq!(o.surface.pad_action(4), PadAction::Blackout);
    o.surface.page = Page::Health;
    o.pump(&sim);
    assert_eq!(o.take_led(800).unwrap().pads[5], LedState::Selected);
    o.surface.select(&[11]).unwrap();
    o.surface
        .action(Action::Edit(Attribute::Intensity))
        .unwrap();
    o.pump(&sim);
    assert_eq!(
        o.take_led(850).unwrap().pads,
        [
            LedState::Available,
            LedState::Available,
            LedState::Off,
            LedState::Off,
            LedState::Off,
            LedState::Available,
            LedState::Available,
            LedState::Available
        ]
    );
    assert_eq!(o.surface.pad_action(6), PadAction::Confirm);
    assert_eq!(o.surface.pad_action(7), PadAction::Cancel);
}

#[test]
fn exhausted_role_generation_fails_closed_and_never_wraps() {
    let (mut o, sim) = rig();
    assign(&mut o, u64::MAX - 1, DeskRole::Lighting);
    o.enqueue(midi(u64::MAX - 1, [0x89, 40, 0])).unwrap();
    assert!(
        o.event(Event::Assignment(Assignment {
            generation: u64::MAX,
            endpoint: "test-controller-a".into(),
            role: DeskRole::Lighting,
        }))
        .is_err()
    );
    assert!(o.assignment().is_none());
    assert!(o.take_led(0).is_none());
    assert_eq!(o.pump(&sim).processed, 0);
    assert!(o.enqueue(midi(u64::MAX - 1, [0x99, 40, 100])).is_err());
    assert!(
        o.event(Event::Assignment(Assignment {
            generation: 1,
            endpoint: "test-controller-a".into(),
            role: DeskRole::Lighting,
        }))
        .is_err()
    );
    assert!(o.assignment().is_none());
}

#[test]
fn invalidated_modal_controller_press_cannot_retarget_blackout() {
    for (before, after) in [(true, false), (false, true)] {
        let (mut o, sim) = rig();
        assign(&mut o, 1, DeskRole::Lighting);
        o.enqueue(midi(1, [0x89, 40, 0])).unwrap();
        o.pump(&sim);
        o.event(Event::Focus(before)).unwrap();
        o.surface.select(&[11]).unwrap();
        o.surface.menu = true;
        o.surface
            .action(Action::Edit(Attribute::Intensity))
            .unwrap();
        o.enqueue(midi(1, [0x99, 40, 100])).unwrap();
        o.event(Event::Focus(after)).unwrap();
        assert!(o.surface.modal.is_none());
        let p = o.pump(&sim);
        assert_eq!(p.refused, 1);
        assert!(p.requests.is_empty());
        assert!(o.surface.notice.contains("stale controller modal"));
        // A duplicate down cannot acquire a new meaning; release still clears its edge.
        o.surface.menu = true;
        o.enqueue(midi(1, [0x99, 40, 100])).unwrap();
        assert!(o.pump(&sim).requests.is_empty());
        o.enqueue(midi(1, [0x89, 40, 0])).unwrap();
        o.enqueue(midi(1, [0x99, 40, 100])).unwrap();
        assert_eq!(o.pump(&sim).requests.len(), 1);
    }
}
