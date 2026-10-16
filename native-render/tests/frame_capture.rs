use pokelinux_render::frame::*;
use pokelinux_render::{Error, ShmDescription, Snapshot, SnapshotStore};
use std::ffi::c_void;
use std::mem::{align_of, offset_of, size_of};
use std::ptr;
use std::slice;

struct Fixture {
    store: SnapshotStore,
    snapshots: Vec<Snapshot>,
    resources: Vec<FrameResourceBorrowV1>,
    draws: Vec<FrameDrawV1>,
    rects: Vec<FrameRectV1>,
    target: FrameTargetV1,
}
impl Fixture {
    fn new() -> Self {
        let store = SnapshotStore::new(4096).unwrap();
        let snapshots: Vec<_> = [(3, 2, 20, 0x34325241), (2, 3, 12, 0x34325258)]
            .into_iter()
            .enumerate()
            .map(|(i, (width, height, stride, drm_format))| {
                let description = ShmDescription {
                    resource_id: 10 + i as u64,
                    generation: 21 + i as u64,
                    commit_sequence: 31 + i as u64,
                    drm_format,
                    width,
                    height,
                    stride,
                    bytes_per_pixel: 4,
                    reserved: 0,
                };
                let pixels: Vec<_> = (0..stride * height)
                    .map(|j| (j * 3 + i as u32) as u8)
                    .collect();
                store.capture(description, &pixels).unwrap()
            })
            .collect();
        let resources = snapshots
            .iter()
            .map(|s| FrameResourceBorrowV1 {
                description: s.description(),
                snapshot: ptr::from_ref(s),
            })
            .collect();
        let cm = FrameColorManagementV1 {
            flags: 7,
            source_tf: 2,
            target_tf: 1,
            tonemap_mode: 1,
            src_tf_min: 0.01,
            src_tf_max: 100.0,
            dst_tf_min: 0.02,
            dst_tf_max: 200.0,
            src_ref_luminance: 80.0,
            dst_ref_luminance: 90.0,
            max_luminance: 1000.0,
            dst_max_luminance: 500.0,
            convert_matrix: [1., 2., 3., 4., 5., 6., 7., 8., 9.],
            dst_primaries_xyz: [9., 8., 7., 6., 5., 4., 3., 2., 1.],
            sdr_saturation: 1.2,
            sdr_brightness: 0.8,
        };
        let material = FrameMaterialV1 {
            alpha: 0.75,
            rounding_px: 6.25,
            rounding_power: 2.2,
            discard_opacity: 0.25,
            flags: 127,
            wrap_x: 1,
            wrap_y: 1,
            reserved: 0,
            tint_rgb: [0.2, 0.3, 0.4],
            cm,
        };
        let draws = (0..4)
            .map(|i| FrameDrawV1 {
                kind: [PR_DRAW_CLEAR, PR_DRAW_RECT, PR_DRAW_SHM, PR_DRAW_SHM][i],
                resource_index: if i < 2 { u32::MAX } else { (i - 2) as u32 },
                scissor_first: (i + 2) as u32,
                scissor_count: 1,
                box_: FrameBoxV1 {
                    x: 1.25,
                    y: 2.5,
                    width: 30.,
                    height: 40.,
                    rotation: 0.125,
                },
                rounding_box: FrameBoxV1 {
                    x: 0.5,
                    y: 1.5,
                    width: 32.,
                    height: 42.,
                    rotation: 0.25,
                },
                projection: [0.5, 1., 1.5, 2., 2.5, 3., 3.5, 4., 4.5],
                uv: [0.1, 0.2, 0.8, 0.9],
                converted_color: [0.2, 0.3, 0.4, 0.5],
                color_srgb: [0.6, 0.7, 0.8, 0.9],
                material,
                surface_id: 100 + i as u64,
                commit_sequence: if i < 2 { 99 } else { 31 + (i - 2) as u64 },
            })
            .collect();
        Self {
            store,
            snapshots,
            resources,
            draws,
            rects: (0..6)
                .map(|i| FrameRectV1 {
                    x1: i,
                    y1: i,
                    x2: i + 10,
                    y2: i + 20,
                })
                .collect(),
            target: FrameTargetV1 {
                output_id: 5,
                output_generation: 6,
                width: 1080,
                height: 2424,
                transformed_width: 2424,
                transformed_height: 1080,
                output_transform: 1,
                flags: 1,
                scale: 1.5,
                target_projection: [1., 2., 3., 4., 5., 6., 7., 8., 9.],
                scale_matrix: [9., 8., 7., 6., 5., 4., 3., 2., 1.],
            },
        }
    }
    fn borrow(&self) -> FrameBorrowV1 {
        FrameBorrowV1 {
            abi_version: PR_FRAME_ABI_V1,
            struct_size: size_of::<FrameBorrowV1>() as u32,
            flags: 3,
            reserved: 0,
            scene_sequence: 1_u64 << 63,
            security_generation: (1_u64 << 63) + 7,
            target: self.target,
            damage_first: 0,
            damage_count: 1,
            final_damage_first: 1,
            final_damage_count: 1,
            draw_count: self.draws.len() as u32,
            resource_count: self.resources.len() as u32,
            rect_count: self.rects.len() as u32,
            reserved2: 0,
            draws: self.draws.as_ptr(),
            resources: self.resources.as_ptr(),
            rects: self.rects.as_ptr(),
        }
    }
    fn capture(&self) -> Frame {
        unsafe { Frame::capture_borrow(&self.borrow()) }.expect("valid frame must capture")
    }
}
fn refused(input: &FrameBorrowV1) {
    assert!(unsafe { Frame::capture_borrow(input) }.is_err());
}

#[test]
fn sealed_repr_c_layouts() {
    assert_eq!(
        (size_of::<FrameRectV1>(), align_of::<FrameRectV1>()),
        (16, 4)
    );
    assert_eq!((size_of::<FrameBoxV1>(), align_of::<FrameBoxV1>()), (40, 8));
    assert_eq!(
        (size_of::<FrameTargetV1>(), align_of::<FrameTargetV1>()),
        (120, 8)
    );
    assert_eq!(
        (
            size_of::<FrameColorManagementV1>(),
            align_of::<FrameColorManagementV1>()
        ),
        (128, 4)
    );
    assert_eq!(
        (size_of::<FrameMaterialV1>(), align_of::<FrameMaterialV1>()),
        (172, 4)
    );
    assert_eq!(
        (size_of::<FrameDrawV1>(), align_of::<FrameDrawV1>()),
        (368, 8)
    );
    assert_eq!(
        (
            size_of::<FrameResourceBorrowV1>(),
            align_of::<FrameResourceBorrowV1>()
        ),
        (56, 8)
    );
    assert_eq!(
        (size_of::<FrameBorrowV1>(), align_of::<FrameBorrowV1>()),
        (208, 8)
    );
    assert_eq!(
        (
            offset_of!(FrameDrawV1, box_),
            offset_of!(FrameDrawV1, material),
            offset_of!(FrameDrawV1, surface_id)
        ),
        (16, 180, 352)
    );
    assert_eq!(
        (
            offset_of!(FrameBorrowV1, target),
            offset_of!(FrameBorrowV1, draws),
            offset_of!(FrameBorrowV1, resources),
            offset_of!(FrameBorrowV1, rects)
        ),
        (32, 184, 192, 200)
    );
}
#[test]
fn copies_all_values_and_arc_clones_pixels_without_recopy() {
    let mut f = Fixture::new();
    let frame = f.capture();
    let expected_draws = f.draws.clone();
    let expected_rects = f.rects.clone();
    let expected_target = f.target;
    let pixels: Vec<_> = f.snapshots.iter().map(|s| s.pixels().as_ptr()).collect();
    let used = f.store.used();
    f.draws.fill(FrameDrawV1::default());
    f.rects.fill(FrameRectV1::default());
    f.target.scale = 99.;
    frame
        .with_borrow(|v| unsafe {
            assert_eq!(v.flags, 3);
            assert_eq!(v.scene_sequence, 1_u64 << 63);
            assert_eq!(v.security_generation, (1_u64 << 63) + 7);
            assert_eq!(v.target, expected_target);
            assert_eq!(
                slice::from_raw_parts(v.draws, v.draw_count as usize),
                expected_draws
            );
            assert_eq!(
                slice::from_raw_parts(v.rects, v.rect_count as usize),
                expected_rects
            );
            for (i, r) in slice::from_raw_parts(v.resources, v.resource_count as usize)
                .iter()
                .enumerate()
            {
                assert_eq!(r.description, f.snapshots[i].description());
                assert_eq!((&*r.snapshot).pixels().as_ptr(), pixels[i]);
            }
        })
        .unwrap();
    assert_eq!(f.store.used(), used);
}
#[test]
fn frame_owns_resources_after_original_handles_and_store_drop() {
    let f = Fixture::new();
    let frame = f.capture();
    let bytes: Vec<Vec<u8>> = f.snapshots.iter().map(|s| s.pixels().to_vec()).collect();
    drop(f);
    frame
        .with_borrow(|v| unsafe {
            for (i, r) in slice::from_raw_parts(v.resources, 2).iter().enumerate() {
                assert_eq!((&*r.snapshot).pixels(), bytes[i]);
            }
        })
        .unwrap();
}
#[test]
fn preserves_known_frame_flags_and_admits_empty_value_frame() {
    let f = Fixture::new();
    for flags in 0..=3 {
        let mut b = f.borrow();
        b.flags = flags;
        let frame = unsafe { Frame::capture_borrow(&b) }.unwrap();
        assert_eq!(frame.flags(), flags);
    }
    let mut b = f.borrow();
    b.draw_count = 0;
    b.resource_count = 0;
    b.rect_count = 0;
    b.damage_count = 0;
    b.final_damage_count = 0;
    b.final_damage_first = 0;
    b.draws = ptr::null();
    b.resources = ptr::null();
    b.rects = ptr::null();
    assert!(unsafe { Frame::capture_borrow(&b) }.is_ok());
}
#[test]
fn refuses_header_unknown_bits_and_c_api_clears_output() {
    let f = Fixture::new();
    for field in 0..7 {
        let mut b = f.borrow();
        match field {
            0 => b.abi_version = 2,
            1 => b.struct_size -= 1,
            2 => b.flags = 4,
            3 => b.reserved = 1,
            4 => b.reserved2 = 1,
            5 => b.scene_sequence = 0,
            _ => b.security_generation = 0,
        };
        refused(&b);
    }
    let mut out = ptr::dangling_mut::<Frame>();
    let mut b = f.borrow();
    b.flags = 4;
    assert_eq!(
        unsafe { pr_frame_capture_v1(&b, &mut out) },
        Error::Invalid as i32
    );
    assert!(out.is_null());
    assert_eq!(
        unsafe { pr_frame_capture_v1(ptr::null(), &mut out) },
        Error::Invalid as i32
    );
    assert!(out.is_null());
    assert_eq!(
        unsafe { pr_frame_capture_v1(&b, ptr::null_mut()) },
        Error::Invalid as i32
    );
    let mut b = f.borrow();
    b.draws = ptr::null();
    refused(&b);
    let mut b = f.borrow();
    b.draws = ptr::without_provenance::<FrameDrawV1>(1);
    refused(&b);
}
#[test]
fn refuses_invalid_target_identity_dimensions_transform_and_scale() {
    let f = Fixture::new();
    for field in 0..10 {
        let mut b = f.borrow();
        match field {
            0 => b.target.output_id = 0,
            1 => b.target.output_generation = 0,
            2 => b.target.width = 0,
            3 => b.target.height = 16385,
            4 => b.target.transformed_width = 0,
            5 => b.target.transformed_height = 16385,
            6 => b.target.output_transform = 8,
            7 => b.target.flags = 2,
            8 => b.target.scale = 0.,
            _ => b.target.scale = f64::INFINITY,
        };
        refused(&b);
    }
}
#[test]
fn refuses_nonfinite_values_in_all_value_domains() {
    for field in 0..12 {
        let mut f = Fixture::new();
        match field {
            0 => f.target.target_projection[3] = f32::NAN,
            1 => f.target.scale_matrix[3] = f32::INFINITY,
            2 => f.draws[0].box_.x = f64::NAN,
            3 => f.draws[0].rounding_box.y = f64::INFINITY,
            4 => f.draws[0].projection[3] = f32::NAN,
            5 => f.draws[0].uv[2] = f32::INFINITY,
            6 => f.draws[0].converted_color[1] = f32::NAN,
            7 => f.draws[0].color_srgb[0] = f32::INFINITY,
            8 => f.draws[0].material.alpha = f32::NAN,
            9 => f.draws[0].material.tint_rgb[1] = f32::INFINITY,
            10 => f.draws[0].material.cm.convert_matrix[3] = f32::NAN,
            _ => f.draws[0].material.cm.dst_primaries_xyz[3] = f32::INFINITY,
        };
        refused(&f.borrow());
    }
    let mut f = Fixture::new();
    f.draws[0].rounding_box.width = f64::MAX;
    refused(&f.borrow());
}
#[test]
fn refuses_any_descriptor_mismatch_without_consuming_input() {
    for field in 0..10 {
        let mut f = Fixture::new();
        let d = &mut f.resources[1].description;
        match field {
            0 => d.resource_id += 1,
            1 => d.generation += 1,
            2 => d.commit_sequence += 1,
            3 => d.drm_format += 1,
            4 => d.width += 1,
            5 => d.height += 1,
            6 => d.stride += 1,
            7 => d.bytes_per_pixel = 3,
            8 => d.reserved = 1,
            _ => f.resources[1].snapshot = ptr::null(),
        };
        let used = f.store.used();
        refused(&f.borrow());
        assert_eq!(f.store.used(), used);
        assert_eq!(f.snapshots[1].pixels().len(), 36);
    }
}
#[test]
fn refuses_wrong_draw_binding_index_commit_and_kind() {
    for field in 0..7 {
        let mut f = Fixture::new();
        match field {
            0 => f.draws[2].resource_index = 2,
            1 => f.draws[2].resource_index = 1,
            2 => f.draws[2].commit_sequence += 1,
            3 => f.draws[2].surface_id = 0,
            4 => f.draws[0].resource_index = 0,
            5 => f.draws[0].kind = 3,
            _ => f.draws[1].box_.width = 0.,
        };
        refused(&f.borrow());
    }
}
#[test]
fn global_rectangles_are_bounded_and_ranges_cannot_overlap() {
    for field in 0..7 {
        let mut f = Fixture::new();
        match field {
            0 => f.rects[0].x1 = -1,
            1 => f.rects[0].x2 = 1081,
            2 => f.rects[0].y2 = 2425,
            3 => f.rects[0].y2 = 0,
            4 => f.draws[1].scissor_first = 2,
            5 => f.draws[1].scissor_first = u32::MAX,
            _ => f.draws[1].scissor_count = u32::MAX,
        };
        refused(&f.borrow());
    }
    let mut f = Fixture::new();
    f.rects = vec![
        FrameRectV1 {
            x1: 0,
            y1: 0,
            x2: 1,
            y2: 1
        };
        4096
    ];
    f.draws.clear();
    let mut b = f.borrow();
    b.damage_count = 4096;
    b.final_damage_count = 0;
    b.final_damage_first = 4096;
    assert!(unsafe { Frame::capture_borrow(&b) }.is_ok());
    b.final_damage_first = 0;
    b.final_damage_count = 1;
    refused(&b);
}
#[test]
fn rejects_caps_before_dereference_and_accepts_max_draw_resource_counts() {
    let f = Fixture::new();
    for field in 0..3 {
        let mut b = f.borrow();
        match field {
            0 => {
                b.draw_count = 257;
                b.draws = ptr::null();
            }
            1 => {
                b.resource_count = 257;
                b.resources = ptr::null();
            }
            _ => {
                b.rect_count = 4097;
                b.rects = ptr::null();
            }
        }
        assert!(matches!(
            unsafe { Frame::capture_borrow(&b) },
            Err(Error::Capacity)
        ));
    }
    let mut f = Fixture::new();
    f.draws = vec![FrameDrawV1::default(); 256];
    f.resources = vec![f.resources[0]; 256];
    let used = f.store.used();
    let frame = f.capture();
    assert_eq!(f.store.used(), used);
    assert_eq!(
        frame
            .with_borrow(|v| (v.draw_count, v.resource_count))
            .unwrap(),
        (256, 256)
    );
}
#[test]
fn refuses_unknown_material_flags_and_invalid_color_management() {
    for field in 0..13 {
        let mut f = Fixture::new();
        let m = &mut f.draws[2].material;
        match field {
            0 => m.flags = 128,
            1 => m.wrap_x = 2,
            2 => m.wrap_y = 2,
            3 => m.reserved = 1,
            4 => m.rounding_px = -1.,
            5 => m.rounding_power = 0.,
            6 => m.cm.flags = 8,
            7 => m.cm.flags = 2,
            8 => m.cm.src_tf_max = m.cm.src_tf_min,
            9 => m.cm.dst_ref_luminance = 0.,
            10 => m.cm.tonemap_mode = 0,
            11 => m.cm.sdr_saturation = -1.,
            _ => m.cm.source_tf = u32::MAX,
        };
        refused(&f.borrow());
    }
}
struct VisitToken {
    calls: usize,
    valid: bool,
    retained: Option<Snapshot>,
}
unsafe extern "C" fn visitor(view: *const FrameBorrowV1, token: *mut c_void) -> i32 {
    let token = unsafe { &mut *token.cast::<VisitToken>() };
    let view = unsafe { &*view };
    token.calls += 1;
    let resource = unsafe { &*view.resources };
    let snapshot = unsafe { &*resource.snapshot };
    token.valid = view.scene_sequence == 1_u64 << 63
        && resource.description == snapshot.description()
        && snapshot.pixels().len() == 40;
    token.retained = Some(snapshot.clone());
    73
}
#[test]
fn synchronous_c_visitor_propagates_status_and_only_explicit_clone_survives() {
    let f = Fixture::new();
    let mut frame = ptr::null_mut();
    assert_eq!(unsafe { pr_frame_capture_v1(&f.borrow(), &mut frame) }, 0);
    let mut token = VisitToken {
        calls: 0,
        valid: false,
        retained: None,
    };
    assert_eq!(
        unsafe { pr_frame_with_view_v1(frame, Some(visitor), ptr::from_mut(&mut token).cast()) },
        73
    );
    assert_eq!(token.calls, 1);
    assert!(token.valid);
    assert_eq!(
        unsafe { pr_frame_with_view_v1(frame, None, ptr::null_mut()) },
        Error::Invalid as i32
    );
    unsafe { pr_frame_release(frame) };
    drop(f);
    assert_eq!(token.retained.unwrap().pixels().len(), 40);
    unsafe { pr_frame_release(ptr::null_mut()) };
}
#[test]
fn real_joined_thread_views_owned_frame_after_inputs_drop() {
    let f = Fixture::new();
    let frame = f.capture();
    let used = f.store.used();
    let store = f.store;
    drop(f.snapshots);
    drop(f.resources);
    drop(f.draws);
    drop(f.rects);
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        let count = frame
            .with_borrow(|v| unsafe {
                slice::from_raw_parts(v.resources, v.resource_count as usize)
                    .iter()
                    .map(|r| (&*r.snapshot).pixels().len())
                    .sum::<usize>()
            })
            .unwrap();
        sender.send(count).unwrap();
    });
    assert_eq!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap(),
        used as usize
    );
    thread.join().unwrap();
    assert_eq!(store.used(), 0);
}
