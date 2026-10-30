use super::super::*;
use crate::model::{Capabilities, InputState, LoopbackState};
use i_slint_backend_testing::{ElementHandle, TestingBackend, TestingBackendOptions};
use std::{cell::RefCell, rc::Rc, time::Duration};

fn advance(milliseconds: u64) {
    i_slint_backend_testing::mock_elapsed_time(Duration::ZERO);
    let mut remaining = milliseconds;
    while remaining > 0 {
        let step = remaining.min(8);
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(step));
        remaining -= step;
    }
}

fn control(ui: &AppWindow, label: &str) -> ElementHandle {
    let matches: Vec<_> = ElementHandle::find_by_accessible_label(ui, label)
        .filter(|element| element.accessible_enabled().is_some())
        .collect();
    assert_eq!(matches.len(), 1, "expected one visible control: {label}");
    matches.into_iter().next().unwrap()
}

fn numeric(ui: &AppWindow, label: &str) -> f32 {
    control(ui, label).accessible_value().unwrap().parse().unwrap()
}

fn frame(ui: &AppWindow, name: &str) -> Vec<u8> {
    let pixels = ui.window().take_snapshot().unwrap();
    let path = std::path::Path::new("evidence/motion");
    std::fs::create_dir_all(path).unwrap();
    let file = std::fs::File::create(path.join(format!("{name}.png"))).unwrap();
    let mut encoder = png::Encoder::new(file, pixels.width(), pixels.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(pixels.as_bytes()).unwrap();
    pixels.as_bytes().to_vec()
}

#[test]
#[ignore = "offscreen UI motion regression; run tools/test-motion.ps1 for element metadata"]
fn headless_motion_regression() {
    // A memory-only software renderer: no native window, SendInput, driver,
    // desktop capture, mouse movement or keyboard focus is involved.
    slint::platform::set_platform(Box::new(TestingBackend::new(TestingBackendOptions {
        mock_time: true,
        threading: false,
        renderer_name: Some("software".into()),
    }))).unwrap();
    let ui = AppWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(720.0, 790.0));
    let mut state = Snapshot {
        connected: true,
        generation: 1,
        status: "SIMULATED · offscreen motion check · no hardware".into(),
        output_db: -20.0,
        monitor_blend: 50.0,
        sample_rate: 96000,
        buffer_size: 1024,
        supported_sample_rates: vec![44100, 48000, 88200, 96000],
        supported_buffers: vec![16, 32, 64, 128, 256, 512, 1024, 2048, 4096],
        inputs: [
            InputState { gain_db: 10.0, ..Default::default() },
            InputState { gain_db: 12.0, ..Default::default() },
        ],
        loopback: std::array::from_fn(|_| LoopbackState { known: true, ..Default::default() }),
        capabilities: Capabilities {
            gain: true, input_mute: true, phantom: true, link: true,
            smartgain: true, output_volume: true, output_mute: true,
            monitor_blend: true, monitor_pan: true, loopback: true,
            sample_rate: true, buffer_size: true, meters: true,
        },
        ..Default::default()
    };
    apply(&ui, state.clone());
    let writes = Rc::new(RefCell::new(Vec::<(usize, f32)>::new()));
    let callback = {
        let writes = writes.clone();
        Rc::new(move |key: usize, value: f32| writes.borrow_mut().push((key, value)))
    };
    { let c = callback.clone(); ui.on_gain_edited(move |i, v| c(i as usize, v)); }
    { let c = callback.clone(); ui.on_output_edited(move |v| c(2, v)); }
    { let c = callback.clone(); ui.on_blend_edited(move |v| c(3, v)); }
    { let c = callback.clone(); ui.on_monitor_pan_edited(move |v| c(4, v)); }
    { let c = callback.clone(); ui.on_loopback_gain_edited(move |i, v| c(5 + i as usize * 2, v)); }
    { let c = callback.clone(); ui.on_loopback_pan_edited(move |i, v| c(6 + i as usize * 2, v)); }
    { let c = callback.clone(); ui.on_phantom_edited(move |i, v| c(100 + i as usize, v as u8 as f32)); }
    { let c = callback.clone(); ui.on_sample_rate_edited(move |v| c(110, v as f32)); }
    { let c = callback.clone(); ui.on_buffer_size_edited(move |v| c(111, v as f32)); }
    ui.show().unwrap(); // Only registers the in-memory testing window.
    advance(300);

    let labels = ["Input 1 gain", "Input 2 gain", "Output level", "Monitor mix", "Monitor pan",
        "Mic 1 level", "Mic 1 pan", "Mic 2 level", "Mic 2 pan", "Main L level", "Main L pan",
        "Main R level", "Main R pan", "Loopback L level", "Loopback L pan", "Loopback R level", "Loopback R pan"];
    let authoritative = [17.0, 19.0, -35.0, 65.0, 25.0, -25.0, 30.0, -24.0, 31.0,
        -23.0, 32.0, -22.0, 33.0, -21.0, 34.0, -20.0, 35.0];
    for (key, label) in labels.iter().enumerate() {
        let prior = numeric(&ui, label);
        let requested = prior + if key < 3 { 0.5 } else { 1.0 };
        control(&ui, label).set_accessible_value(requested.to_string());
        assert_eq!(numeric(&ui, label), requested, "animation delayed actual value: {label}");
        assert_eq!(writes.borrow().last(), Some(&(key, requested)));
        apply(&ui, state.clone()); // Simulated failed-write rollback.
        assert_eq!(numeric(&ui, label), prior, "rollback failed: {label}");
        match key {
            0 | 1 => state.inputs[key].gain_db = authoritative[key],
            2 => state.output_db = authoritative[key],
            3 => state.monitor_blend = authoritative[key],
            4 => state.monitor_pan = authoritative[key],
            _ if key % 2 == 1 => state.loopback[(key - 5) / 2].gain_db = authoritative[key],
            _ => state.loopback[(key - 5) / 2].pan = authoritative[key],
        }
        apply(&ui, state.clone()); // Unrelated external authoritative value.
        assert_eq!(numeric(&ui, label), authoritative[key], "readback failed: {label}");
    }
    assert_eq!(writes.borrow().len(), 17, "visual interpolation sent extra commands");
    advance(300);
    let stable = frame(&ui, "console");
    advance(500);
    assert_eq!(stable, frame(&ui, "idle"), "settled motion must stop repainting changes");
    assert!(!ui.window().has_active_animations(), "decorative animation kept running while idle");

    let before_resize = control(&ui, "Output level").size().width;
    ui.window().set_size(slint::LogicalSize::new(720.0, 520.0));
    advance(40);
    frame(&ui, "reflow-40ms");
    let middle_resize = control(&ui, "Output level").size().width;
    advance(300);
    frame(&ui, "reflow-settled");
    let after_resize = control(&ui, "Output level").size().width;
    assert!(after_resize < middle_resize && middle_resize < before_resize,
        "reflow did not interpolate: {before_resize} -> {middle_resize} -> {after_resize}");
    ui.window().set_size(slint::LogicalSize::new(720.0, 790.0));
    advance(300);

    let input = control(&ui, "Input 1 gain");
    let settings = control(&ui, "Settings");
    let position = settings.absolute_position();
    let size = settings.size();
    let center = slint::LogicalPosition::new(position.x + size.width / 2.0, position.y + size.height / 2.0);
    use slint::platform::{WindowEvent, PointerEventButton};
    ui.window().dispatch_event(WindowEvent::PointerMoved { position: center });
    advance(140);
    let hovering = frame(&ui, "button-hover");
    ui.window().dispatch_event(WindowEvent::PointerPressed { position: center, button: PointerEventButton::Left });
    advance(32);
    assert_ne!(hovering, frame(&ui, "button-pressed"), "button press gave no visual feedback");
    assert_eq!(settings.size(), size, "press animation moved the hitbox");
    ui.window().dispatch_event(WindowEvent::PointerReleased { position: center, button: PointerEventButton::Left });
    advance(40);
    let opening = frame(&ui, "settings-enter-40ms");
    advance(200);
    let opened = frame(&ui, "settings-open");
    assert_ne!(opening, opened, "panel did not interpolate its entrance");
    assert_eq!(input.accessible_enabled(), Some(false), "covered input remained interactive");
    input.invoke_accessible_increment_action();
    assert_eq!(writes.borrow().len(), 17, "covered input accepted a command");
    let rate = control(&ui, "Sample rate");
    control(&ui, "Settings").invoke_accessible_default_action();
    advance(16);
    assert!(rate.is_valid(), "outgoing controls vanished before the exit fade");
    assert_eq!(rate.accessible_enabled(), Some(false), "outgoing controls stayed interactive");
    let closing = frame(&ui, "settings-exit-16ms");
    advance(200);
    assert_ne!(closing, frame(&ui, "settings-closed"));
    assert_eq!(control(&ui, "Input 1 gain").accessible_enabled(), Some(true));

    // Latest selection wins, including close/reopen while a fade is pending.
    ui.window().set_size(slint::LogicalSize::new(720.0, 450.0));
    advance(300);
    for pane in [3, 2, 3, 3, 2] {
        ui.invoke_toggle_detail(pane);
        advance(8);
    }
    advance(300);
    assert_eq!(control(&ui, "Loopback source").accessible_enabled(), Some(true));
    ui.invoke_select_source(5);
    advance(0);
    assert_eq!(numeric(&ui, "Loopback R level"), -20.0);
    assert_eq!(numeric(&ui, "Loopback R pan"), 35.0);
    frame(&ui, "rapid-switch-final-loopback");
    ui.invoke_toggle_detail(2);
    advance(300);
    assert_eq!(writes.borrow().len(), 17, "navigation applied saved hardware state");

    ui.set_reduced_motion(true);
    advance(0);
    control(&ui, "Settings").invoke_accessible_default_action();
    advance(0);
    advance(0);
    assert_eq!(control(&ui, "Sample rate").computed_opacity(), 1.0);
    control(&ui, "Settings").invoke_accessible_default_action();
    advance(0);
    advance(0);
    assert_eq!(control(&ui, "Input 1 gain").accessible_enabled(), Some(true));
    ui.set_reduced_motion(false);
    advance(300);

    let sizes = [(1080, 790), (720, 790), (720, 520), (400, 790), (400, 400)];
    for (width, height) in sizes {
        ui.window().set_size(slint::LogicalSize::new(width as f32, height as f32));
        advance(350);
        for label in ["Output level", "Input 1 gain", "Input 2 gain"] {
            let element = control(&ui, label);
            let p = element.absolute_position();
            let s = element.size();
            assert!(s.width > 0.0 && s.height > 0.0 && p.x >= 0.0 && p.y >= 0.0
                && p.x + s.width <= width as f32 + 1.0 && p.y + s.height <= height as f32 + 1.0,
                "{label} outside {width}x{height}: {p:?}, {s:?}");
        }
        frame(&ui, &format!("layout-{width}x{height}"));
    }
    for (pane, label) in [(1, "Monitor pan"), (2, "Loopback R level"), (3, "Sample rate"), (4, "Start")] {
        ui.invoke_toggle_detail(pane);
        advance(300);
        let element = control(&ui, label);
        assert_eq!(element.accessible_enabled(), Some(true), "compact inspector disabled {label}");
        let p = element.absolute_position();
        let s = element.size();
        assert!(p.x >= 0.0 && p.y >= 0.0 && p.x + s.width <= 401.0 && p.y + s.height <= 401.0,
            "compact {label} outside viewport: {p:?}, {s:?}");
        frame(&ui, &format!("compact-inspector-{pane}"));
        ui.invoke_toggle_detail(pane);
        advance(250);
    }
    assert_eq!(writes.borrow().len(), 17, "resize/reflow issued a hardware command");
    let report = serde_json::json!({"backend":"headless software; no desktop window or physical input",
        "continuous_controls":17,"readback_checks":34,"actual_commands":17,
        "extra_animation_commands":0,"rapid_navigation_passed":true,
        "covered_and_exiting_controls_disabled":true,"reduced_motion_passed":true,
        "idle_pixels_stable":true,"idle_decorative_animations_stopped":true,
        "reflow_interpolates":true,"press_feedback_and_stable_hitbox":true,
        "compact_inspectors_passed":4,"logical_sizes":sizes});
    std::fs::write("evidence/motion/report.json", serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("{report}");
}
