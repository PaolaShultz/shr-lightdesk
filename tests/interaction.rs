use shr_lightdesk::{
    controller::{EncoderMode, Input},
    model::{Attribute, Authority, Command},
    simulator::Simulator,
    surface::{
        Layer, Page, Surface,
        actions::{Action, Draft, typed_value},
    },
};
fn rig() -> (Surface, Simulator) {
    let sim = Simulator::new();
    (Surface::new(sim.snapshot()), sim)
}
fn apply(d: &mut Surface, sim: &mut Simulator, command: Command) {
    let request = d.queue(command).unwrap();
    d.acknowledge(sim.request(&request));
}
#[test]
fn typed_editor_is_detached_validated_and_preserves_intentional_zero() {
    let (mut d, mut sim) = rig();
    d.action(Action::Select(vec![11, 12])).unwrap();
    let before = d.confirmed.clone();
    d.action(Action::Edit(Attribute::Intensity)).unwrap();
    for text in ["NaN", "1e2", "10.55", "101", "-1", "9999999999999999"] {
        d.action(Action::Text(text.into())).unwrap();
        assert!(d.action(Action::Confirm).is_err(), "{text}");
        assert!(d.modal.is_some());
        assert_eq!(d.confirmed, before);
    }
    d.action(Action::Text("0".into())).unwrap();
    let command = d.action(Action::Confirm).unwrap().unwrap();
    assert_eq!(
        command,
        Command::Set {
            targets: vec![(11, Attribute::Intensity), (12, Attribute::Intensity)],
            value: 0
        }
    );
    apply(&mut d, &mut sim, command);
    assert_eq!(d.confirmed.programmer[&(11, Attribute::Intensity)], 0);
    assert!(d.action(Action::Edit(Attribute::Pan)).is_err());
    assert_eq!(typed_value("-12.3"), Ok(-123));
}
#[test]
fn record_destination_does_not_go_or_replace_implicitly() {
    let (mut d, mut sim) = rig();
    apply(
        &mut d,
        &mut sim,
        Command::Set {
            targets: vec![(11, Attribute::Intensity)],
            value: 450,
        },
    );
    let before = d.confirmed.clone();
    d.action(Action::Record {
        palette: false,
        replace: false,
        id: 1,
    })
    .unwrap();
    d.action(Action::Text("0".into())).unwrap();
    assert!(d.action(Action::Confirm).is_err());
    d.action(Action::Text("32".into())).unwrap();
    assert_eq!(d.confirmed, before);
    let command = d.action(Action::Confirm).unwrap().unwrap();
    apply(&mut d, &mut sim, command);
    assert!(d.confirmed.cues.contains_key(&32));
    assert!(d.confirmed.playbacks.iter().all(Option::is_none));
    assert_eq!(d.confirmed.programmer, before.programmer);
    d.action(Action::Record {
        palette: false,
        replace: false,
        id: 32,
    })
    .unwrap();
    assert!(d.action(Action::Confirm).is_err());
    d.action(Action::Cancel).unwrap();
    d.action(Action::Record {
        palette: true,
        replace: false,
        id: 2,
    })
    .unwrap();
    let command = d.action(Action::Confirm).unwrap().unwrap();
    apply(&mut d, &mut sim, command);
    assert!(d.confirmed.palettes.contains_key(&2));
}
#[test]
fn selection_page_revision_and_input_loss_invalidate_drafts() {
    let (mut d, mut sim) = rig();
    d.select(&[11]).unwrap();
    for action in [
        Action::Page(Page::Library),
        Action::Select(vec![12]),
        Action::ContextLost,
        Action::Back,
    ] {
        d.action(Action::Edit(Attribute::Intensity)).unwrap();
        d.action(Action::Text("42".into())).unwrap();
        let before = d.confirmed.clone();
        d.action(action).unwrap();
        assert!(d.modal.is_none());
        assert_eq!(d.confirmed, before);
        assert!(d.action(Action::Confirm).is_err());
    }
    d.action(Action::Edit(Attribute::Intensity)).unwrap();
    d.action(Action::Text("42".into())).unwrap();
    d.confirmed.revision += 1;
    assert!(d.action(Action::Confirm).is_err());
    assert!(d.modal.is_none());
    d.reconnect(sim.snapshot());
    d.action(Action::Edit(Attribute::Intensity)).unwrap();
    apply(&mut d, &mut sim, Command::Master(500));
    assert!(d.modal.is_none());
    d.action(Action::Edit(Attribute::Intensity)).unwrap();
    d.disconnect();
    assert!(d.modal.is_none());
    assert!(d.action(Action::Confirm).is_err());
}
#[test]
fn blackout_off_is_revision_bound_cancelable_and_never_a_pad_toggle() {
    let (mut d, mut sim) = rig();
    apply(&mut d, &mut sim, Command::Blackout(true));
    d.menu = true;
    assert!(d.input(Input::Pad(4), &sim).unwrap().is_none());
    assert!(matches!(
        d.modal.as_ref().unwrap().draft,
        Draft::BlackoutOff
    ));
    assert!(d.confirmed.blackout);
    d.input(Input::Pad(7), &sim).unwrap();
    assert!(d.confirmed.blackout);
    d.action(Action::Blackout(false)).unwrap();
    d.action(Action::Page(Page::Health)).unwrap();
    assert!(d.action(Action::Confirm).is_err());
    d.action(Action::Blackout(false)).unwrap();
    let command = d.input(Input::Pad(6), &sim).unwrap().unwrap();
    assert_eq!(command, Command::Blackout(false));
    apply(&mut d, &mut sim, command);
    assert!(!d.confirmed.blackout);
}
#[test]
fn keyboard_and_controller_share_drafts_navigation_and_confirmation() {
    let (mut keyboard, sim) = rig();
    let (mut controller, _) = rig();
    keyboard.action(Action::Select(vec![11])).unwrap();
    controller.input(Input::Key(2), &sim).unwrap();
    keyboard.action(Action::Edit(Attribute::Intensity)).unwrap();
    controller.input(Input::Pad(6), &sim).unwrap();
    controller.input(Input::Pad(3), &sim).unwrap();
    keyboard.action(Action::Text("45".into())).unwrap();
    controller.input(Input::Key(4), &sim).unwrap();
    controller.input(Input::Key(5), &sim).unwrap();
    assert_eq!(
        keyboard.action(Action::Confirm).unwrap(),
        controller.input(Input::Pad(6), &sim).unwrap()
    );
    for page in Page::ALL {
        keyboard.action(Action::Page(page)).unwrap();
        controller
            .input(Input::Action(Action::Page(page)), &sim)
            .unwrap();
        assert_eq!(keyboard.page, controller.page);
        assert_eq!(keyboard.selected, controller.selected);
    }
    controller
        .input(Input::Action(Action::Layer(Layer::Groups)), &sim)
        .unwrap();
    assert_eq!(controller.layer, Layer::Groups);
    controller
        .input(Input::Action(Action::Navigate(1)), &sim)
        .unwrap();
    controller
        .input(Input::Key(controller.selection_focus), &sim)
        .unwrap();
    assert!(!controller.selected.is_empty());
}
#[test]
fn loss_requires_release_and_rearms_pickup_without_replaying_drafts() {
    let (mut d, sim) = rig();
    d.select(&[11]).unwrap();
    d.controller.decode("light-sim", &[0x99, 42, 100]);
    d.action(Action::ContextLost).unwrap();
    assert_eq!(d.controller.decode("light-sim", &[0x99, 42, 100]), None);
    d.controller.decode("light-sim", &[0x89, 42, 0]);
    assert_eq!(
        d.controller.decode("light-sim", &[0x99, 42, 100]),
        Some(Input::Pad(6))
    );
    d.action(Action::Edit(Attribute::Intensity)).unwrap();
    d.controller.profile.modes[0] = EncoderMode::TwosComplement;
    d.input(Input::Rotary(0, 1), &sim).unwrap();
    assert!(
        matches!(&d.modal.as_ref().unwrap().draft, Draft::Attribute { text, .. } if text == "1.0")
    );
    assert!(d.confirmed.programmer.is_empty());
}

#[test]
fn rotary_and_typed_keyboard_emit_identical_scalar_commands() {
    let (mut keyboard, sim) = rig();
    let (mut controller, _) = rig();
    keyboard.select(&[21]).unwrap();
    controller.select(&[21]).unwrap();
    keyboard.action(Action::Edit(Attribute::Pan)).unwrap();
    keyboard.action(Action::Text("-1.0".into())).unwrap();
    controller.controller.profile.modes[4] = EncoderMode::TwosComplement;
    assert_eq!(
        keyboard.action(Action::Confirm).unwrap(),
        controller.input(Input::Rotary(4, 127), &sim).unwrap()
    );
    keyboard.action(Action::Edit(Attribute::Intensity)).unwrap();
    keyboard.action(Action::Text("42".into())).unwrap();
    keyboard.selected.insert(11); // Even unexpected direct local scope mutation cannot retarget a draft.
    assert!(keyboard.action(Action::Confirm).is_err());
    assert!(keyboard.modal.is_none());
}

#[test]
fn invalid_drafts_never_enter_rotary_arithmetic() {
    let (mut d, sim) = rig();
    d.select(&[21]).unwrap();
    for text in ["2147483647", "-2147483648", "0", "33", "garbage"] {
        d.action(Action::Record {
            palette: false,
            replace: false,
            id: 1,
        })
        .unwrap();
        d.action(Action::Text(text.into())).unwrap();
        assert!(d.input(Input::Rotary(0, 0), &sim).is_err());
    }
    for a in Attribute::ALL {
        d.action(Action::Edit(a)).unwrap();
        for text in ["214748364.7", "-214748364.7"] {
            d.action(Action::Text(text.into())).unwrap();
            assert!(d.input(Input::Rotary(0, 127), &sim).is_err());
        }
        let (lo, hi) = a.range();
        for (value, raw) in [(lo, 0), (hi, 127)] {
            d.action(Action::Text(format!(
                "{}{}.{:01}",
                if value < 0 { "-" } else { "" },
                value.abs() / 10,
                value.abs() % 10
            )))
            .unwrap();
            assert!(d.input(Input::Rotary(0, raw), &sim).is_ok());
        }
    }
    let mut c = shr_lightdesk::controller::Controller::default();
    assert_eq!(
        c.value(0, 127, i32::MAX, (i32::MIN, i32::MAX)),
        Some(i32::MAX)
    );
    assert_eq!(c.value(0, 0, 0, (0, 0)), None);
}
#[test]
fn typed_and_navigation_edits_rearm_but_rotary_drafts_remain_continuous() {
    let (mut d, sim) = rig();
    d.select(&[11]).unwrap();
    d.action(Action::Edit(Attribute::Intensity)).unwrap();
    d.input(Input::Rotary(0, 0), &sim).unwrap();
    d.input(Input::Rotary(0, 1), &sim).unwrap();
    assert!(d.controller.pickup[0].acquired);
    d.action(Action::Text("90".into())).unwrap();
    d.input(Input::Rotary(0, 1), &sim).unwrap();
    assert!(!d.controller.pickup[0].acquired);
    assert!(
        matches!(&d.modal.as_ref().unwrap().draft, Draft::Attribute { text, .. } if text == "90")
    );
    d.input(Input::Rotary(0, 114), &sim).unwrap();
    assert!(d.controller.pickup[0].acquired);
    d.action(Action::Navigate(1)).unwrap();
    assert!(!d.controller.pickup[0].acquired);
    d.input(Input::Rotary(0, 114), &sim).unwrap();
    d.action(Action::Backspace).unwrap();
    assert!(!d.controller.pickup[0].acquired);
}
#[test]
fn context_cancellation_restores_base_pad_and_rotary_meanings() {
    let (mut d, sim) = rig();
    d.select(&[11]).unwrap();
    for action in [
        Action::Select(vec![11]),
        Action::Layer(Layer::Fixtures),
        Action::Attribute(1),
    ] {
        d.action(Action::Edit(Attribute::Intensity)).unwrap();
        d.action(action).unwrap();
        assert!(d.modal.is_none());
        assert!(!d.menu);
        d.input(Input::Pad(1), &sim).unwrap();
        assert_eq!(d.page, Page::Programmer);
        assert!(d.input(Input::Rotary(0, 0), &sim).is_ok());
    }
}

#[test]
fn fresh_zoom_draft_has_a_valid_rotary_start_without_typed_input() {
    let (mut d, sim) = rig();
    d.select(&[21]).unwrap();
    d.action(Action::Edit(Attribute::Zoom)).unwrap();
    d.input(Input::Rotary(0, 0), &sim).unwrap();
    assert!(
        matches!(&d.modal.as_ref().unwrap().draft, Draft::Attribute { text, .. } if text == "5.0")
    );
    d.input(Input::Rotary(0, 1), &sim).unwrap();
    assert!(d.controller.pickup[0].acquired);
    assert!(d.confirmed.programmer.is_empty());
}
