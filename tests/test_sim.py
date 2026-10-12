import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location('pixel_sim', Path(__file__).parents[1] / 'tools' / 'sim.py')
sim = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(sim)


class SimulationContract(unittest.TestCase):
    def test_offline_guest_has_no_host_devices_or_network(self):
        args = sim.qemu_command(Path('/work'), Path('/results'), provision=False)
        self.assertEqual(args[args.index('-nic') + 1], 'none')
        self.assertIn('systemd.mask=systemd-networkd-wait-online.service', args[args.index('-append') + 1])
        self.assertIn('tcg,thread=multi', args)
        self.assertNotIn('-enable-kvm', args)
        self.assertTrue(any('readonly=on' in arg and 'mount_tag=project' not in arg for arg in args))
        self.assertTrue(any('id=results' in arg and 'security_model=mapped-xattr' in arg for arg in args))
        # root=/dev/vda is inside the guest; no host device is passed as a drive/share.
        self.assertFalse(any('file=/dev/' in arg or 'path=/dev/' in arg or 'hostfwd' in arg for arg in args))

    def test_provision_network_has_no_inbound_forwarding(self):
        args = sim.qemu_command(Path('/work'), Path('/results'), provision=True)
        self.assertIn('user,model=virtio-net-pci', args)
        self.assertFalse(any('hostfwd' in arg for arg in args))

    def test_incomplete_or_failed_guest_is_not_green(self):
        for checks in ({}, {'boot': True}, {name: name != 'touch' for name in sim.REQUIRED_CHECKS}):
            with self.subTest(checks=checks), self.assertRaises(ValueError):
                sim.validate_result({'checks': checks, 'hardware_verified': False})

    def test_simulation_must_not_claim_pixel_hardware_validation(self):
        result = {'checks': dict.fromkeys(sim.REQUIRED_CHECKS, True), 'hardware_verified': True}
        with self.assertRaises(ValueError):
            sim.validate_result(result)
        result['hardware_verified'] = False
        sim.validate_result(result)

    def test_error_after_checks_is_still_a_failure(self):
        with self.assertRaises(ValueError):
            sim.validate_result({'checks': dict.fromkeys(sim.REQUIRED_CHECKS, True),
                                 'hardware_verified': False, 'error': 'cleanup failed'})

    def test_factory_boundary_requires_trace_not_only_egl_error(self):
        output = 'CLIENT_EXTENSIONS=EGL_EXT_client_extensions\nDISPLAY=0x1 ERROR=0x3000\nINITIALIZE=0 VERSION=0.0 ERROR=0x3003\n'
        with self.assertRaises(ValueError):
            sim.validate_factory_boundary(77, output, '')
        sim.validate_factory_boundary(77, output,
            'openat(AT_FDCWD,"/vendor/etc/powervr.ini",O_RDONLY) = 3\n'
            'openat(AT_FDCWD,"/dev/dri/renderD128",O_RDWR) = -1 errno=2 (No such file or directory)\n')
        with self.assertRaises(ValueError):
            sim.validate_factory_boundary(139, output, 'renderD128')
