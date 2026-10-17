# Explicit virtual battery backend

Select the existing kernel's matching `test_power` module only inside a disposable
VM, and install `battery.conf` at `/etc/pokelinux/battery.conf`, root0:0/0644.
The normal unit reads `/sys/class/power_supply/test_battery/uevent` through an
owned reader process. There is no software percentage override or UI fixture
value in the production service/client.

A physical device requires its own selected power_supply name and separately
qualified kernel behavior; the VM profile supplies no Pixel battery evidence.
Never load the test module or inspect power attributes on the Harbour host or a
physical phone. Stage the service, unit and selected config through the finite
`install-native-services` manifest; staging does not activate host services.
