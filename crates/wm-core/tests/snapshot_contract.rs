use wm_core::snapshot::{Observation, Rect, WindowId, WindowObservation, WindowSnapshot};

fn window(id: u64, title: Observation<String>) -> WindowObservation {
    WindowObservation {
        id: WindowId(id),
        title,
        class_name: Observation::Available("TestWindow".to_owned()),
        process_id: Observation::Available(42),
        process_path: Observation::Available("C:\\test-app.exe".to_owned()),
        owner: Observation::Available(None),
        visible: Observation::Available(true),
        minimized: Observation::Available(false),
        cloaked: Observation::Available(false),
        rect: Observation::Available(Rect {
            left: -20,
            top: 10,
            right: 380,
            bottom: 310,
        }),
        style: Observation::Available(0x00cf_0000),
        ex_style: Observation::Available(0),
    }
}

#[test]
fn snapshot_sorts_windows_by_stable_id_and_serializes_deterministically() {
    let first = WindowSnapshot::new(
        123,
        vec![
            window(9, Observation::Available("Second".to_owned())),
            window(2, Observation::Available("First".to_owned())),
        ],
    );
    let second = WindowSnapshot::new(
        123,
        vec![
            window(2, Observation::Available("First".to_owned())),
            window(9, Observation::Available("Second".to_owned())),
        ],
    );

    assert_eq!(first.windows[0].id, WindowId(2));
    assert_eq!(first, second);
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&second).unwrap()
    );
}

#[test]
fn snapshot_preserves_unavailable_fields_without_misreporting_them_as_false() {
    let observation = Observation::Unavailable("window disappeared during query".to_owned());
    let window = window(7, observation.clone());
    let snapshot = WindowSnapshot::new(456, vec![window]);
    let encoded = serde_json::to_string(&snapshot).unwrap();
    let decoded: WindowSnapshot = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.windows[0].title, observation);
    assert_ne!(decoded.windows[0].visible, Observation::Available(false));
}

#[test]
fn snapshot_keeps_a_window_when_process_metadata_is_inaccessible() {
    let mut candidate = window(8, Observation::Available("Protected app".to_owned()));
    candidate.process_path = Observation::Unavailable("access denied".to_owned());
    let snapshot = WindowSnapshot::new(457, vec![candidate]);

    assert_eq!(snapshot.windows.len(), 1);
    assert_eq!(snapshot.windows[0].process_id, Observation::Available(42));
    assert_eq!(
        snapshot.windows[0].process_path,
        Observation::Unavailable("access denied".to_owned())
    );
}

#[test]
fn snapshot_retains_hidden_minimized_cloaked_and_owned_window_facts() {
    let mut candidate = window(11, Observation::Available(String::new()));
    candidate.visible = Observation::Available(false);
    candidate.minimized = Observation::Available(true);
    candidate.cloaked = Observation::Available(true);
    candidate.owner = Observation::Available(Some(WindowId(3)));
    let snapshot = WindowSnapshot::new(789, vec![candidate]);

    let observed = &snapshot.windows[0];
    assert_eq!(observed.title, Observation::Available(String::new()));
    assert_eq!(observed.visible, Observation::Available(false));
    assert_eq!(observed.minimized, Observation::Available(true));
    assert_eq!(observed.cloaked, Observation::Available(true));
    assert_eq!(observed.owner, Observation::Available(Some(WindowId(3))));
}

#[test]
fn snapshot_keeps_geometry_and_styles_in_the_observation() {
    let snapshot = WindowSnapshot::new(1, vec![window(4, Observation::Available("A".to_owned()))]);
    let observed = &snapshot.windows[0];

    assert_eq!(
        observed.rect,
        Observation::Available(Rect {
            left: -20,
            top: 10,
            right: 380,
            bottom: 310
        })
    );
    assert_eq!(observed.style, Observation::Available(0x00cf_0000));
    assert_eq!(observed.ex_style, Observation::Available(0));
}

#[test]
fn empty_desktop_is_a_valid_serializable_snapshot() {
    let snapshot = WindowSnapshot::new(0, Vec::new());
    assert!(snapshot.windows.is_empty());
    let decoded: WindowSnapshot =
        serde_json::from_str(&serde_json::to_string(&snapshot).unwrap()).unwrap();
    assert_eq!(decoded, snapshot);
}
