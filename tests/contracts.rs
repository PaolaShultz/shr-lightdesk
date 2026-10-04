use shr_lightdesk::{
    controller::*,
    model::*,
    render::{self, Font, Primitive},
    simulator::Simulator,
    surface::*,
};
fn rig() -> (Surface, Simulator) {
    let sim = Simulator::new();
    (Surface::new(sim.snapshot()), sim)
}
fn send(d: &mut Surface, s: &mut Simulator, c: Command) {
    let r = d.queue(c).unwrap();
    d.acknowledge(s.request(&r));
    assert!(
        matches!(d.result, ResultState::Confirmed(_)),
        "{:?}",
        d.result
    );
}
fn set(d: &mut Surface, s: &mut Simulator, id: u16, a: Attribute, v: i32) {
    send(
        d,
        s,
        Command::Set {
            targets: vec![(id, a)],
            value: v,
        },
    );
}
fn record(d: &mut Surface, s: &mut Simulator, id: u16) {
    send(
        d,
        s,
        Command::Record {
            id,
            palette: false,
            replace: false,
        },
    );
}
fn output(d: &Surface) -> Vec<i32> {
    d.confirmed
        .resolved
        .values()
        .map(|r| r.final_value)
        .collect()
}

#[test]
fn selection_and_groups_do_not_write() {
    let (mut d, s) = rig();
    let before = d.confirmed.clone();
    d.select(&[11, 12]).unwrap();
    d.additive = true;
    d.select(&[12, 13]).unwrap();
    assert_eq!(d.selected.iter().copied().collect::<Vec<_>>(), vec![11, 13]);
    d.layer = Layer::Groups;
    d.additive = false;
    d.input(Input::Key(0), &s).unwrap();
    assert_eq!(d.selected.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
    d.deselect();
    assert_eq!(d.confirmed, before);
}
#[test]
fn unsupported_multi_edit_is_atomic() {
    let (mut d, mut s) = rig();
    let before = s.snapshot();
    let r = d
        .queue(Command::Set {
            targets: vec![(11, Attribute::Pan), (21, Attribute::Pan)],
            value: 20,
        })
        .unwrap();
    d.acknowledge(s.request(&r));
    assert!(matches!(d.result, ResultState::Rejected(_)));
    assert_eq!(s.snapshot(), before);
}
#[test]
fn pending_never_looks_confirmed_and_busy_is_visible() {
    let (mut d, _) = rig();
    d.queue(Command::Master(500)).unwrap();
    assert_eq!(d.confirmed.master, 1000);
    assert!(d.queue(Command::Master(600)).is_err());
    assert!(matches!(d.result, ResultState::Pending(_)));
}
#[test]
fn clear_to_hold_and_record_are_distinct() {
    let (mut d, mut s) = rig();
    set(&mut d, &mut s, 11, Attribute::Intensity, 420);
    record(&mut d, &mut s, 1);
    let look = output(&d);
    send(&mut d, &mut s, Command::ClearToHold);
    assert_eq!(output(&d), look);
    assert!(d.confirmed.programmer.is_empty());
    assert_eq!(
        d.confirmed.resolved[&(11, Attribute::Intensity)].source,
        Source::Hold
    );
    let r = d
        .queue(Command::Record {
            id: 2,
            palette: false,
            replace: false,
        })
        .unwrap();
    assert!(s.request(&r).result.is_err());
}
#[test]
fn intensity_htp_color_activation_and_manual_zero() {
    let (mut d, mut s) = rig();
    set(&mut d, &mut s, 11, Attribute::Intensity, 700);
    set(&mut d, &mut s, 11, Attribute::Red, 100);
    record(&mut d, &mut s, 1);
    send(&mut d, &mut s, Command::Go { cue: 1, slot: 0 });
    set(&mut d, &mut s, 11, Attribute::Intensity, 300);
    set(&mut d, &mut s, 11, Attribute::Red, 900);
    record(&mut d, &mut s, 2);
    send(&mut d, &mut s, Command::Go { cue: 2, slot: 1 });
    send(
        &mut d,
        &mut s,
        Command::Release {
            targets: vec![(11, Attribute::Intensity), (11, Attribute::Red)],
            destination: Destination::Playback,
        },
    );
    assert_eq!(d.confirmed.resolved[&(11, Attribute::Intensity)].value, 700);
    assert_eq!(d.confirmed.resolved[&(11, Attribute::Red)].value, 900);
    set(&mut d, &mut s, 11, Attribute::Intensity, 0);
    assert_eq!(d.confirmed.resolved[&(11, Attribute::Intensity)].value, 0);
    send(&mut d, &mut s, Command::Off { slot: 1 });
    assert_eq!(d.confirmed.resolved[&(11, Attribute::Red)].value, 100);
}
#[test]
fn masters_and_blackout_only_affect_intensity() {
    let (mut d, mut s) = rig();
    set(&mut d, &mut s, 21, Attribute::Intensity, 800);
    set(&mut d, &mut s, 21, Attribute::Pan, 1200);
    send(&mut d, &mut s, Command::Master(500));
    assert_eq!(
        d.confirmed.resolved[&(21, Attribute::Intensity)].final_value,
        400
    );
    send(&mut d, &mut s, Command::Blackout(true));
    assert_eq!(
        d.confirmed.resolved[&(21, Attribute::Intensity)].final_value,
        0
    );
    assert_eq!(
        d.confirmed.resolved[&(21, Attribute::Pan)].final_value,
        1200
    );
    send(&mut d, &mut s, Command::Mode(Mode::Auto));
    assert!(d.confirmed.blackout);
    send(&mut d, &mut s, Command::Blackout(false));
    assert_eq!(
        d.confirmed.resolved[&(21, Attribute::Intensity)].final_value,
        400
    );
}
#[test]
fn modes_preserve_look_and_automation_cannot_steal_hold() {
    let (mut d, mut s) = rig();
    let t = (11, Attribute::Intensity);
    set(&mut d, &mut s, 11, t.1, 440);
    send(&mut d, &mut s, Command::ClearToHold);
    let before = output(&d);
    for mode in [Mode::Auto, Mode::Assist, Mode::Manual, Mode::Auto] {
        send(&mut d, &mut s, Command::Mode(mode));
        assert_eq!(output(&d), before);
    }
    send(
        &mut d,
        &mut s,
        Command::Grant {
            targets: vec![t],
            low: 100,
            high: 600,
        },
    );
    send(
        &mut d,
        &mut s,
        Command::Propose {
            target: t,
            value: 900,
        },
    );
    assert_eq!(d.confirmed.automation[&t], 600);
    assert_eq!(output(&d), before);
    d.select(&[11]).unwrap();
    d.release_preview(&s, t.1, Destination::Automation).unwrap();
    assert_eq!(output(&d), before);
    let c = d.confirm_release().unwrap();
    send(&mut d, &mut s, c);
    assert_eq!(d.confirmed.resolved[&t].value, 600);
}
#[test]
fn assist_accept_is_explicit_and_takes_programmer() {
    let (mut d, mut s) = rig();
    send(&mut d, &mut s, Command::Mode(Mode::Assist));
    let t = (11, Attribute::Intensity);
    let before = output(&d);
    send(
        &mut d,
        &mut s,
        Command::Propose {
            target: t,
            value: 550,
        },
    );
    assert_eq!(output(&d), before);
    send(&mut d, &mut s, Command::Accept { targets: vec![t] });
    assert_eq!(d.confirmed.resolved[&t].source, Source::Programmer);
    assert_eq!(d.confirmed.resolved[&t].value, 550);
}
#[test]
fn release_preview_cannot_follow_selection_or_new_revision() {
    let (mut d, mut s) = rig();
    d.select(&[11]).unwrap();
    d.release_preview(&s, Attribute::Intensity, Destination::Playback)
        .unwrap();
    d.select(&[12]).unwrap();
    assert!(d.confirm_release().is_err());
    d.release_preview(&s, Attribute::Intensity, Destination::Playback)
        .unwrap();
    send(&mut d, &mut s, Command::Master(500));
    assert!(d.confirm_release().is_err());
}
#[test]
fn release_to_playback_excludes_auto_until_explicit_return() {
    let (mut d, mut s) = rig();
    let t = (11, Attribute::Intensity);
    send(&mut d, &mut s, Command::Mode(Mode::Auto));
    send(
        &mut d,
        &mut s,
        Command::Grant {
            targets: vec![t],
            low: 0,
            high: 1000,
        },
    );
    send(
        &mut d,
        &mut s,
        Command::Propose {
            target: t,
            value: 700,
        },
    );
    send(
        &mut d,
        &mut s,
        Command::Release {
            targets: vec![t],
            destination: Destination::Playback,
        },
    );
    assert_eq!(d.confirmed.resolved[&t].value, 0);
    send(
        &mut d,
        &mut s,
        Command::Propose {
            target: t,
            value: 800,
        },
    );
    assert_eq!(d.confirmed.resolved[&t].value, 0);
}
#[test]
fn lost_ack_reconnect_refreshes_applied_state_without_replay() {
    let (mut d, mut s) = rig();
    let r = d.queue(Command::Master(500)).unwrap();
    let delayed = s.request(&r);
    d.disconnect();
    assert!(d.queue(Command::Master(900)).is_err());
    d.acknowledge(delayed.clone());
    assert_eq!(d.confirmed.master, 1000);
    d.reconnect(s.snapshot());
    assert_eq!(d.confirmed.master, 500);
    assert!(d.pending.is_none());
    d.acknowledge(delayed);
    assert_eq!(d.confirmed.master, 500);
}
#[test]
fn stale_revision_wrong_epoch_and_duplicate_id_rejected() {
    let (mut d, mut s) = rig();
    let r = d.queue(Command::Master(500)).unwrap();
    let reply = s.request(&r);
    assert_eq!(s.request(&r), reply);
    let mut bad = r.clone();
    bad.command = Command::Master(300);
    assert!(s.request(&bad).result.is_err());
    bad.id += 1;
    assert!(s.request(&bad).result.is_err());
    bad.epoch += 1;
    assert!(s.request(&bad).result.is_err());
    assert_eq!(s.snapshot().master, 500);
}
#[test]
fn expired_ids_stay_expired_after_bounded_cache_rollover() {
    let (_, mut s) = rig();
    let first = Request {
        id: 1,
        epoch: 1,
        revision: 0,
        command: Command::Master(500),
    };
    assert!(s.request(&first).result.is_ok());
    for id in 2..=70 {
        let r = Request {
            id,
            epoch: 1,
            revision: s.snapshot().revision,
            command: Command::Master(600),
        };
        assert!(s.request(&r).result.is_ok());
    }
    assert!(s.request(&first).result.is_err());
    assert_eq!(s.snapshot().master, 600);
}
#[test]
fn record_update_does_not_mutate_active_playback() {
    let (mut d, mut s) = rig();
    set(&mut d, &mut s, 11, Attribute::Intensity, 500);
    record(&mut d, &mut s, 1);
    send(&mut d, &mut s, Command::Go { cue: 1, slot: 0 });
    set(&mut d, &mut s, 11, Attribute::Intensity, 800);
    send(
        &mut d,
        &mut s,
        Command::Record {
            id: 1,
            palette: false,
            replace: true,
        },
    );
    assert_eq!(
        d.confirmed.playbacks[0].as_ref().unwrap().values[&(11, Attribute::Intensity)],
        500
    );
    assert_eq!(
        d.confirmed.cues[&1].values[&(11, Attribute::Intensity)],
        800
    );
}
#[test]
fn palette_recall_only_changes_selected_matching_fixtures() {
    let (mut d, mut s) = rig();
    set(&mut d, &mut s, 11, Attribute::Red, 200);
    set(&mut d, &mut s, 12, Attribute::Red, 300);
    send(
        &mut d,
        &mut s,
        Command::Record {
            id: 1,
            palette: true,
            replace: false,
        },
    );
    send(&mut d, &mut s, Command::ClearToHold);
    send(
        &mut d,
        &mut s,
        Command::RecallPalette {
            id: 1,
            fixtures: vec![11],
        },
    );
    assert_eq!(d.confirmed.programmer.len(), 1);
    assert!(d.confirmed.holds.contains_key(&(12, Attribute::Red)));
}
#[test]
fn midi_ownership_channels_edges_and_octaves() {
    let mut c = Controller::default();
    assert_eq!(c.decode("audio-sim", &[0x90, 48, 100]), None);
    assert_eq!(
        c.decode("light-sim", &[0x90, 60, 100]),
        Some(Input::Key(12))
    );
    assert_eq!(c.decode("light-sim", &[0x90, 60, 100]), None);
    assert_eq!(c.decode("light-sim", &[0x90, 60, 0]), None);
    assert_eq!(c.decode("light-sim", &[0x99, 36, 100]), Some(Input::Pad(0)));
    c.context_changed();
    assert_eq!(c.decode("light-sim", &[0x99, 36, 100]), None);
    assert_eq!(c.decode("light-sim", &[0xa9, 36, 50]), None);
    c.disconnected();
    assert_eq!(c.decode("light-sim", &[0x99, 36, 100]), None);
    c.decode("light-sim", &[0x89, 36, 0]);
    assert_eq!(c.decode("light-sim", &[0x99, 36, 100]), Some(Input::Pad(0)));
    assert_eq!(c.decode("light-sim", &[0xb0, 16, 128]), None);
}
#[test]
fn ambiguous_profiles_and_devices_do_not_get_first_match() {
    let p = Profile {
        pad_channel: 0,
        ..Profile::default()
    };
    assert!(p.validate().is_err());
    let v = vec!["same".into(), "same".into()];
    assert!(unique_endpoint(&v, "same").is_err());
    assert!(unique_endpoint(&v, "missing").is_err());
}
#[test]
fn pickup_has_no_stale_crossing_and_relative_steps_are_bounded() {
    let mut p = Pickup::default();
    assert!(!p.apply(10, 64));
    assert!(p.apply(90, 64));
    let mut c = Controller::default();
    c.pickup[0] = p;
    c.context_changed();
    assert_eq!(c.value(0, 90, 500, (0, 1000)), None);
    assert_eq!(relative(EncoderMode::TwosComplement, 127), Some(-1));
    assert_eq!(relative(EncoderMode::BinaryOffset, 65), Some(1));
    assert_eq!(relative(EncoderMode::SignedBit, 65), Some(-1));
    assert_eq!(relative(EncoderMode::TwosComplement, 63), Some(8));
    c.profile.modes[0] = EncoderMode::TwosComplement;
    c.fine = true;
    assert_eq!(c.value(0, 1, 500, (0, 1000)), Some(501));
}
#[test]
fn every_page_is_full_hd_font_complete_and_bounded() {
    let (mut d, mut s) = rig();
    d.select(&[11, 12, 21]).unwrap();
    set(&mut d, &mut s, 11, Attribute::Intensity, 400);
    let font = Font::default();
    for page in Page::ALL {
        d.page = page;
        let scene = render::scene(&d);
        assert!(scene.in_bounds(), "{page:?}");
        for p in &scene.primitives {
            if let Primitive::Text { value, .. } = p {
                assert!(value.chars().all(|c| font.supports(c)), "{value}");
            }
        }
        let svg = render::svg(&scene);
        assert!(svg.contains("1920 1080"));
        assert!(svg.contains("OFFLINE SIM"));
        assert!(!svg.contains("<script"));
    }
}
#[test]
fn rendering_changes_for_pending_rejected_disconnected_and_blackout() {
    let (mut d, mut s) = rig();
    let a = render::svg(&render::scene(&d));
    let r = d.queue(Command::Master(500)).unwrap();
    let b = render::svg(&render::scene(&d));
    assert!(b.contains("PENDING"));
    assert_ne!(a, b);
    d.acknowledge(Reply {
        id: r.id,
        epoch: r.epoch,
        result: Err("fixture unavailable".into()),
    });
    assert!(render::svg(&render::scene(&d)).contains("REJECTED"));
    send(&mut d, &mut s, Command::Blackout(true));
    assert!(render::svg(&render::scene(&d)).contains("BLACKOUT LATCHED"));
    d.disconnect();
    assert!(render::svg(&render::scene(&d)).contains("DISCONNECTED"));
}

#[test]
fn tightening_a_live_auto_grant_holds_current_look_until_release() {
    let (mut d, mut s) = rig();
    let t = (11, Attribute::Intensity);
    send(&mut d, &mut s, Command::Mode(Mode::Auto));
    send(
        &mut d,
        &mut s,
        Command::Grant {
            targets: vec![t],
            low: 0,
            high: 1000,
        },
    );
    send(
        &mut d,
        &mut s,
        Command::Propose {
            target: t,
            value: 900,
        },
    );
    send(
        &mut d,
        &mut s,
        Command::Release {
            targets: vec![t],
            destination: Destination::Automation,
        },
    );
    let before = output(&d);
    send(
        &mut d,
        &mut s,
        Command::Grant {
            targets: vec![t],
            low: 0,
            high: 500,
        },
    );
    assert_eq!(output(&d), before);
    assert_eq!(d.confirmed.automation[&t], 500);
    assert_eq!(d.confirmed.resolved[&t].source, Source::Hold);
}

#[test]
fn controller_actions_reach_the_same_authority_and_respect_capabilities() {
    let (mut d, mut s) = rig();
    d.select(&[11]).unwrap();
    d.controller.profile.modes[0] = EncoderMode::TwosComplement;
    let event = d.controller.decode("light-sim", &[0xb0, 16, 1]).unwrap();
    let command = d.input(event, &s).unwrap().unwrap();
    send(&mut d, &mut s, command);
    assert_eq!(d.confirmed.programmer[&(11, Attribute::Intensity)], 10);
    assert!(d.input(Input::Rotary(4, 64), &s).is_err());
    d.input(Input::Pad(6), &s).unwrap(); // Actions
    assert!(d.input(Input::Rotary(0, 1), &s).is_err());
    let command = d.input(Input::Pad(1), &s).unwrap().unwrap(); // Clear Hold
    send(&mut d, &mut s, command);
    assert!(d.confirmed.programmer.is_empty());
    assert_eq!(d.confirmed.holds[&(11, Attribute::Intensity)], 10);
}

#[test]
fn release_preview_detects_authority_changes_before_the_surface_refreshes() {
    let (mut d, mut s) = rig();
    d.select(&[11]).unwrap();
    assert!(
        s.request(&Request {
            id: 70,
            epoch: 1,
            revision: 0,
            command: Command::Master(500)
        })
        .result
        .is_ok()
    );
    assert!(
        d.release_preview(&s, Attribute::Intensity, Destination::Playback)
            .is_err()
    );
    assert!(d.preview.is_none());
    assert_eq!(d.confirmed.master, 1000);
}

#[test]
fn raster_and_svg_share_the_same_full_hd_scene() {
    let (d, _) = rig();
    let scene = render::scene(&d);
    let bytes = render::ppm(&scene);
    let header = b"P6\n1920 1080\n255\n";
    assert!(bytes.starts_with(header));
    assert_eq!(bytes.len(), header.len() + 1920 * 1080 * 3);
    assert_eq!(&bytes[header.len()..header.len() + 3], &[0x19, 0x23, 0x30]);
}
