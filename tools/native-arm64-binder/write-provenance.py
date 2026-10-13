#!/usr/bin/env python3
"""Record executed ARM guest Binder evidence; never infer a pass from source alone."""
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
root = Path(__file__).resolve().parents[2]
base = root/'artifacts/native-arm64-binder'

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def relative(path):
    return str(path.relative_to(root))

def record(label, expected, count=None):
    directory = base/'runtime'/label
    result = json.loads((directory/'result.json').read_text())
    assert result['passed'] is expected
    assert result['architecture'] == 'aarch64' and result['qemu_mode'] == 'system'
    assert result['hardware_verified'] is False and result['phone_accessed'] is False
    if count is not None:
        assert len(result['checks']) == count
        assert all(value['passed'] for value in result['checks'].values())
    files = [path for path in directory.iterdir() if path.is_file() and path.name != 'serial.log']
    return {'directory': relative(directory), 'result': result,
            'files_sha256': {path.name: sha(path) for path in sorted(files)}}

sources = list((root/'tools/native-arm64-binder').rglob('*'))
sources = [path for path in sources if path.is_file() and '__pycache__' not in path.parts]
sources += [root/path for path in [
    'native-radio/Cargo.toml','native-radio/Cargo.lock','native-radio/build.rs',
    'native-radio/aidl/RadioExternalResponseInfo.aidl','native-radio/src/lib.rs',
    'native-radio/src/transport.rs','native-radio/src/receiver.rs','native-radio/src/owner.rs',
    'native-radio/src/worker.rs','native-radio/src/supervisor.rs','native-radio/src/request.rs',
    'native-radio/src/deadline.rs','native-radio/src/response.rs','native-radio/src/indication.rs',
    'native-ims/Cargo.toml','native-ims/src/requests.rs']]
rsbinder = next((root/'sim/rust-cargo/registry/src').glob('*/rsbinder-0.12.0'))
sources += [rsbinder/'src'/name for name in ['process_state.rs','native.rs','proxy.rs','thread_state.rs','parcel.rs','status.rs']]
sources += [rsbinder/'aidl/16/android/os/IServiceManager.aidl']
generated = list((base/'target/aarch64-unknown-linux-gnu/debug/build').glob('pokelinux-radio-*/out/radio.rs'))
generated += list((base/'target/aarch64-unknown-linux-gnu/debug/build').glob('rsbinder-*/out/service_manager_16.rs'))
rust = record('final-rust-green', True, 9)
ndk = record('final-ndk-green', True, 3)
assert all(value.get('peer_reaped') for value in rust['result']['checks'].values())
assert all(value['ndk-peer_reaped'] and value['manager_reaped'] and not value['ndk_stub_runtime']
           for value in ndk['result']['checks'].values())
config = (base/'runtime/final-rust-green/kernel.config').read_text()
evidence = {
    'scope': 'Real ARM64 guest-kernel Binder through native Rust radio owner/workers/supervisor; independent synthetic native Rust peer and separate genuine Android-runtime NDK interop',
    'recorded_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'repository_head_at_recording': subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),
    'production_sources_changed_by_this_gate': False,
    'phone_accessed': False, 'hardware_verified': False, 'production_backend_enabled': False,
    'evidence_classes': {
        'arm64_system_qemu_kernel_binder_synthetic_native_peer': 'Nine executed scenarios, genuine guest driver, manual independent Rust peer wire parcels',
        'arm64_system_qemu_original_android_runtime_interop': 'Three executed scenarios with synthetic NDK peer over hash-verified original ARM64 Android Binder/Bionic, synthetic Rust service manager and one readiness property',
        'arm64_user_mode_qemu': 'Not used for these runtime verdicts',
        'x86_android_reference': 'Earlier independent evidence; not reused as ARM proof',
        'original_vendor_radio_service_execution': False, 'physical_device_execution': False},
    'retained_test_kernel': {
        'release': rust['result']['kernel'], 'version': rust['result']['kernel_version'],
        'image_path': 'sim/boot/Image', 'image_sha256': sha(root/'sim/boot/Image'),
        'initramfs_sha256': sha(root/'sim/boot/initramfs-linux.img'),
        'config_sha256': sha(base/'runtime/final-rust-green/kernel.config'),
        'binder_config': rust['result']['binder_config'],
        'compiler_config': [line for line in config.splitlines() if line.startswith(('CONFIG_CC_VERSION_TEXT=', 'CONFIG_LD_VERSION='))],
        'matched_module_directory': '/usr/lib/modules/7.1.6-1-aarch64-ARCH',
        'inventory_sha256': sha(base/'retained-capabilities.txt'),
        'new_kernel_or_module_build': False,
        'public_prebuilt_origin': 'Retained Arch Linux ARM base/boot image from pinned public rootfs; Binder built in, no retained kernel headers needed',
        'arch_base_url': 'https://de3.mirror.archlinuxarm.org/os/multi/ArchLinuxARM-2026.08-aarch64-rootfs.tar.gz',
        'arch_base_archive_sha256': sha(root/'sim/cache/ArchLinuxARM-aarch64-latest.tar.gz'),
        'pixel_matched_kernel': False,
        'google_target_kernel_modules_firmware': 'Retained target assets untouched; not replaced or qualified by this test kernel'},
    'execution_environment': {
        'builder_image': 'sha256:0b4b956502a9f55d992eaced6975737b523be19bd2034078983b36bbcfd7aee6',
        'qemu_container_image': 'sha256:561177a4305051567b1d28d67ba9a95e0ce0ad32a8815b49da42fe71aa5a3dd6',
        'rust_compiler': 'rustc1.98.1 (48a229cea 2026-09-01), LLVM22.1.8',
        'rust_cross_linker': 'aarch64-linux-gnu-gcc Debian14.2.0-19',
        'ndk_compiler': (base/'ndk-compiler.txt').read_text().strip(),
        'native_uid': 1000, 'network': 'none at container and QEMU NIC',
        'root_disk': 'dedicated clone, snapshot writes; shared base disk untouched',
        'project_mount': 'read-only', 'host_device_transport': 'no adb/USB/host Binder device/module exposure',
        'process_ownership': 'actual production worker JoinHandle/death watch and Supervisor Child retained until completed join/actual reap; every independent fixture Popen explicitly waited'},
    'source_sha256': {relative(path): sha(path) for path in sorted(set(sources))},
    'generated_interface_sha256': {relative(path): sha(path) for path in sorted(generated)},
    'red_green': {
        'driver_provisioning_red': record('driver-red',False),
        'unchanged_driver_binary_green': record('driver-green',True),
        'held_fixture_capability_red': record('held-red',False),
        'held_fixture_red_meaning': 'Actual Binder UnknownTransaction199 before synthetic hold controls; fixture capability regression, not a new production policy defect',
        'genuine_ndk_startup_red': record('ndk-red-2',False),
        'genuine_ndk_startup_red_meaning': 'Original libbinder_ndk loaded, then actual12-second timeout on absent servicemanager.ready property; resolved via pinned original serializer and genuine Bionic API, without library mocking',
        'ndk_manager_numbering_red': record('ndk-green',False),
        'ndk_manager_numbering_red_meaning': 'Actual NDK service registration rejected before Android16 addService5/checkService3 correction; early stdout prefix was lost by original ready helper, retained stderr/kernel logs show failure',
        'setup_failures_not_runtime_policy_red': [
            'runtime/harness-setup-failed: initial container user/mount paths corrected before actual driver red',
            'runtime/ndk-red: Android loader rejected glibc-style --library-path argument; corrected to actual LD_LIBRARY_PATH interface'],
        'frozen_artifacts': ['driver-red','held-red','rust-green','ndk-red','ndk-manager-red'],
        'frozen_artifacts_sha256': {relative(path): sha(path) for name in ['driver-red','held-red','rust-green','ndk-red','ndk-manager-red']
            for path in sorted((base/name).rglob('*')) if path.is_file() and '__pycache__' not in path.parts}},
    'synthetic_native_rust_peer_green': rust,
    'genuine_android_runtime_ndk_peer_green': ndk,
    'ndk_link_inputs': json.loads((root/'tools/native-arm64-binder/ndk-link-inputs.json').read_text()),
    'genuine_android_runtime_inputs': json.loads((root/'tools/native-arm64-binder/android-runtime-inputs.json').read_text()),
    'public_aosp_fixture_sources': json.loads((root/'tools/native-arm64-binder/public-inputs.json').read_text()),
    'reproduction': ['bash tools/native-arm64-binder/run.sh build',
        'bash tools/native-arm64-binder/run.sh clone (once; refuses overwriting existing clone)',
        'bash tools/native-arm64-binder/run.sh run',
        'bash tools/native-arm64-binder/run.sh ndk-build',
        'bash tools/native-arm64-binder/run.sh ndk-run'],
    'limits': ['System-QEMU ARM CPU/virtual hardware only; not physical Pixel evidence',
        'Arch Binder/kernel policy differs from matched Google target; Pixel permissions/SELinux remain open',
        'NDK peer and minimal service manager/readiness property are synthetic; no original vendor radio service executed',
        'Explicit genuine library path lacks Android linker namespace config; recorded loader warning, not full Android deployment',
        'NDK held-registration/permanent-supervisor scenarios are not qualified; those nine gates use independent native Rust peer',
        'One-way submission/raw echo remain OutcomeUnknown, no modem or semantic receipt claim',
        'Local reaping is not vendor cleanup; no implicit uncertain registration retry/successor allowed',
        'Installed service manager/app boundaries, vendor discovery/reset/cleanup, native IMS/call state, permanent request driver pressure, modem/carrier behavior and hardware remain open']}
assert evidence['red_green']['driver_provisioning_red']['result']['client_sha256'] == evidence['red_green']['unchanged_driver_binary_green']['result']['client_sha256']
target = root/'docs/evidence/native-arm64-binder-provenance.json'
target.write_text(json.dumps(evidence,indent=2)+'\n')
print('Recorded actual ARM Binder gates: nine synthetic native Rust, three genuine Android-runtime NDK; hardware/vendor service unqualified')
