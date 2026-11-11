//! Export the camera at the instant Export is pressed, with remembered hidden-edge, tolerance
//! and zero-point choices.
use crate::dialogs::form_row;
use crate::{SolveApp, dialogs::Kind, selection::SelInput};
use serde_json::json;
use solidcraft_engine::schema::silhouette::DxfZero;

pub fn start(app: &SolveApp, id: &str) -> Option<(Kind, Vec<SelInput>)> {
    (id == "file.export_view_dxf").then(|| {
        let tolerance = app.preferences.view_dxf_tolerance;
        let tolerance = if tolerance.is_finite() && (0.0001..=1.0).contains(&tolerance) { tolerance } else { 0.01 };
        (Kind::ViewDxf { dashed: app.preferences.view_dxf_dashed, tolerance, zero: app.preferences.view_dxf_zero }, Vec::new())
    })
}

pub fn rows(app: &SolveApp, ui: &mut egui::Ui, dashed: &mut bool, tolerance: &mut f64, zero: &mut DxfZero) {
    crate::ui_kit::text(ui, app.tr_label("Hidden edges"), crate::ui_kit::Text::Note("hidden"));
    for (value, key, label) in [(false, "omit", "Do not export hidden edges"), (true, "dashed", "Export hidden edges as dashed lines")] {
        let r = crate::ui_kit::list_picker(ui, key, app.tr_label(label), *dashed == value);
        crate::scenario::publish_handle(&format!("dialog:dxf:{key}"), r.rect.center());
        if r.clicked() {
            *dashed = value;
        }
    }
    crate::ui_kit::text(ui, app.tr("dxf.zero"), crate::ui_kit::Text::Note("zero"));
    for (value, key, label) in [(DxfZero::Target, "target", "dxf.zero_target"), (DxfZero::Origin, "origin", "dxf.zero_origin")] {
        let r = crate::ui_kit::list_picker(ui, key, app.tr(label), *zero == value);
        crate::scenario::publish_handle(&format!("dialog:dxf:{key}"), r.rect.center());
        if r.clicked() {
            *zero = value;
        }
    }
    form_row(ui, "dxf.tolerance", |ui| {
        let r = crate::ui_kit::bounded_length_field(ui, tolerance, &app.session.doc.units, 8, 0.0001..=1.0, 0.001);
        crate::scenario::publish_handle("dialog:dxf:tolerance", r.rect.center());
    });
    crate::ui_kit::text(
        ui,
        app.tr_label(if app.cam.fov > 0.0 { "Current perspective view" } else { "Current orthographic view (1:1 mm)" }),
        crate::ui_kit::Text::Note("projection"),
    );
    crate::ui_kit::text(ui, app.tr_label("dxf.tolerance_help"), crate::ui_kit::Text::Note("tolerance"));
}

pub fn run(app: &mut SolveApp, dashed: bool, tolerance: f64, zero: DxfZero) -> Result<(), String> {
    if crate::viewport::section_plane(app).is_some() {
        return Err(app.tr_label("Turn off section analysis before exporting the view.").into());
    }
    if !solidcraft_engine::cmd::view_dxf_exportable(&app.session) {
        return Err(app.tr_label("No visible bodies or sketches to export.").into());
    }
    if !tolerance.is_finite() || !(0.0001..=1.0).contains(&tolerance) {
        return Err(app.tr_label("dxf.invalid_tolerance").into());
    }
    app.preferences.view_dxf_dashed = dashed;
    app.preferences.view_dxf_tolerance = tolerance;
    app.preferences.view_dxf_zero = zero;
    let camera = app.cam;
    app.pick_save(&format!("{}-view.dxf", app.session.doc.name), &["dxf"], move |app, path| {
        if let Some(path) = path {
            let params = json!({"camera":camera,"hidden":if dashed {"dashed"} else {"omit"},"tolerance":tolerance,"zero":zero,"path":path});
            let mut request = crate::file_load::Request::new(&path, crate::file_load::Kind::ExportDxf);
            request.options = params.as_object().cloned().unwrap_or_default();
            let _ = crate::file_load::begin_request(app, request);
        }
    });
    Ok(())
}

pub(crate) fn progress_text(work: Option<(&str, usize, usize)>) -> String {
    let (stage, done, total) = work.unwrap_or(("dxf_meshes", 0, 1));
    let label = match stage {
        "dxf_index" => "dxf.progress.index",
        "dxf_projection" => "dxf.progress.projection",
        "dxf_occlusion" => "dxf.progress.occlusion",
        "dxf_write" => "dxf.progress.write",
        _ => "dxf.progress.meshes",
    };
    format!("{}: {done}/{total}", solidcraft_ui_text::tr_label(label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use solidcraft_viewport::StandardView;

    #[test]
    fn view_dxf_file_menu_exports_with_the_current_camera_and_remembers_hidden_choice() {
        let mut h = crate::scenario::Harness::new();
        h.app.run("solid.box", json!({"length":40,"width":30,"height":10})).unwrap();
        h.app.run("solid.cylinder", json!({"radius":3,"height":4,"base":[10,10,0],"operation":"cut"})).unwrap();
        h.app.cam.fov = 0.0;
        h.app.cam.set_view(StandardView::Top);
        h.frames(3);
        h.step(&json!({"click":{"handle":"appbar:folder"}})).unwrap();
        h.step(&json!({"widget":"Export Current View as DXF…"})).unwrap();
        assert!(matches!(h.app.dialog.as_ref().map(|d| &d.kind), Some(Kind::ViewDxf { dashed: false, .. })));
        h.step(&json!({"click":{"handle":"dialog:dxf:tolerance"},"double":true})).unwrap();
        h.step(&json!({"key":"A","cmd":true})).unwrap();
        h.step(&json!({"text":"0.1"})).unwrap();
        h.step(&json!({"key":"Tab"})).unwrap();
        h.step(&json!({"click":{"handle":"dialog:dxf:dashed"}})).unwrap();
        let dir = h.dir.clone();
        h.app.services.pick_save = Some(Box::new(move |name, extensions| {
            assert!(name.ends_with("-view.dxf"));
            assert_eq!(extensions, &["dxf"]);
            Box::pin(std::future::ready(Some(dir.join("view.dxf").to_string_lossy().into_owned())))
        }));
        h.step(&json!({"click":{"handle":"dialog:Export DXF…"}})).unwrap();
        wait_for_export(&mut h);
        let text = std::fs::read_to_string(h.dir.join("view.dxf")).unwrap();
        assert!(text.contains("LWPOLYLINE") && text.contains("HIDDEN"));
        assert!(h.app.dialog.is_none() && h.app.preferences.view_dxf_dashed);
        let saved = h.app.prefs();
        let mut restored = SolveApp::new(Default::default(), Default::default());
        restored.load_prefs(&saved);
        assert!(restored.preferences.view_dxf_dashed);
        assert_eq!(restored.preferences.view_dxf_tolerance, 0.1);
        h.app.start("file.export_view_dxf");
        h.frames(3);
        assert!(matches!(h.app.dialog.as_ref().map(|d| &d.kind), Some(Kind::ViewDxf { dashed: true, tolerance: 0.1, .. })));
        h.step(&json!({"click":{"handle":"dialog:dxf:omit"}})).unwrap();
        h.step(&json!({"click":{"handle":"dialog:Export DXF…"}})).unwrap();
        wait_for_export(&mut h);
        let text = std::fs::read_to_string(h.dir.join("view.dxf")).unwrap();
        assert!(!text.contains("LWPOLYLINE") && !text.contains("HIDDEN"));
        assert!(!h.app.preferences.view_dxf_dashed);
    }

    #[test]
    fn view_dxf_zero_point_can_be_the_model_origin_and_is_remembered() {
        let mut h = crate::scenario::Harness::new();
        h.app.run("solid.box", json!({"length":40,"width":30,"height":10})).unwrap();
        h.app.cam.fov = 0.0;
        h.app.cam.set_view(StandardView::Top);
        h.app.cam.target = solidcore_record::Vec3::new(20., 15., 5.);
        h.frames(3);
        h.app.start("file.export_view_dxf");
        h.frames(3);
        let dir = h.dir.clone();
        h.app.services.pick_save = Some(Box::new(move |name, extensions| {
            assert!(name.ends_with("-view.dxf"));
            assert_eq!(extensions, &["dxf"]);
            Box::pin(std::future::ready(Some(dir.join("view.dxf").to_string_lossy().into_owned())))
        }));
        // The default centres the drawing on the camera target, not on the model's origin.
        h.step(&json!({"click":{"handle":"dialog:Export DXF…"}})).unwrap();
        wait_for_export(&mut h);
        let (lo, hi) = dxf_bounds(&h.dir.join("view.dxf"));
        assert!(near(lo, -20., -15.) && near(hi, 20., 15.), "{lo:?} {hi:?}");
        // "Model origin" puts the model's origin at the drawing's 0,0.
        h.app.start("file.export_view_dxf");
        h.frames(3);
        h.step(&json!({"click":{"handle":"dialog:dxf:origin"}})).unwrap();
        h.step(&json!({"click":{"handle":"dialog:Export DXF…"}})).unwrap();
        wait_for_export(&mut h);
        let (lo, hi) = dxf_bounds(&h.dir.join("view.dxf"));
        assert!(near(lo, 0., 0.) && near(hi, 40., 30.), "{lo:?} {hi:?}");
        // The choice is remembered, in the app and in the stored preferences.
        assert_eq!(h.app.preferences.view_dxf_zero, DxfZero::Origin);
        let saved = h.app.prefs();
        let mut restored = SolveApp::new(Default::default(), Default::default());
        restored.load_prefs(&saved);
        assert_eq!(restored.preferences.view_dxf_zero, DxfZero::Origin);
        h.app.start("file.export_view_dxf");
        h.frames(3);
        assert!(matches!(h.app.dialog.as_ref().map(|d| &d.kind), Some(Kind::ViewDxf { zero: DxfZero::Origin, .. })));
    }

    /// The drawing's bounding box, in the DXF's own units.
    fn dxf_bounds(path: &std::path::Path) -> ((f64, f64), (f64, f64)) {
        let text = std::fs::read_to_string(path).unwrap();
        let mut lo = (f64::MAX, f64::MAX);
        let mut hi = (f64::MIN, f64::MIN);
        let mut any = false;
        for g in solidcraft_engine::io::read_dxf(&text).unwrap() {
            let pts: Vec<(f64, f64)> = match g {
                solidcraft_engine::io::Geom2::Point(p) => vec![(p.x, p.y)],
                solidcraft_engine::io::Geom2::Line(a, b) => vec![(a.x, a.y), (b.x, b.y)],
                solidcraft_engine::io::Geom2::Circle(c, r) => vec![(c.x - r, c.y), (c.x + r, c.y)],
                solidcraft_engine::io::Geom2::Arc { a, b, .. } => vec![(a.x, a.y), (b.x, b.y)],
                solidcraft_engine::io::Geom2::Ellipse { c, major, .. } => vec![(c.x, c.y), (major.x, major.y)],
                solidcraft_engine::io::Geom2::Spline { pts, .. } => pts.iter().map(|p| (p.x, p.y)).collect(),
                solidcraft_engine::io::Geom2::Conic { a, b, .. } => vec![(a.x, a.y), (b.x, b.y)],
                solidcraft_engine::io::Geom2::Text { at, .. } => vec![(at.x, at.y)],
            };
            for (x, y) in pts {
                any = true;
                lo = (lo.0.min(x), lo.1.min(y));
                hi = (hi.0.max(x), hi.1.max(y));
            }
        }
        assert!(any, "the drawing is empty");
        (lo, hi)
    }

    fn near(p: (f64, f64), x: f64, y: f64) -> bool {
        (p.0 - x).abs() < 1e-3 && (p.1 - y).abs() < 1e-3
    }

    #[test]
    fn inch_dxf_tolerance_is_edited_in_inches_and_kept_in_millimetres() {
        let mut h = crate::scenario::Harness::new();
        h.app.run("document.units", json!({"units":"in"})).unwrap();
        h.app.start("file.export_view_dxf");
        h.frames(3);
        h.step(&json!({"click":{"handle":"dialog:dxf:tolerance"},"double":true})).unwrap();
        h.step(&json!({"key":"A","cmd":true})).unwrap();
        h.step(&json!({"text":"0.001"})).unwrap();
        h.step(&json!({"key":"Tab"})).unwrap();
        let Some(Kind::ViewDxf { tolerance, .. }) = h.app.dialog.as_ref().map(|d| &d.kind) else { panic!("no DXF dialog") };
        assert!((*tolerance - 0.0254).abs() < 1e-10, "{tolerance}");
    }

    fn wait_for_export(h: &mut crate::scenario::Harness) {
        let began = std::time::Instant::now();
        loop {
            h.frames(1);
            if h.app.file_load.idle() && !h.app.file_picker_busy() {
                break;
            }
            assert!(began.elapsed().as_secs() < 10, "export did not finish");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
