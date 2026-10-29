"""Read-only checks for a future persistent hwsim consumer.

No process, network or filesystem mutation. These checks validate supplied
observations, not their provenance. They deliberately expose no overall PASS:
the runtime owner must still bind actual peers, raw readbacks, waits and EOF.
"""
import importlib.util
from pathlib import Path

_SOURCE = Path(__file__).resolve().parents[1] / 'native-wireless-association/address-packets/address_consumer.py'
_SPEC = importlib.util.spec_from_file_location('wireless_address_observations', _SOURCE)
address = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(address)
need = address.need
integer = address.integer


def existing_identity(fact, selected):
    """Normalized recorder observation, not a new production wire schema."""
    address.target(selected)
    need(type(fact) is dict, 'existing-link observation object')
    need(fact.get('target') == selected, 'same exact existing-link target')
    need('observed_lifetime' in fact and fact['observed_lifetime'] is None,
         'existing link must not invent a witnessed create lifetime')
    need(type(fact.get('configured')) is bool, 'explicit profile configuration')
    return fact['configured']


def persistent_identity(samples, *, selected, worker, requester_closed_at, until):
    """Check monotonic samples after requester closure; radio checked separately."""
    integer(requester_closed_at, 1, address.U64 - 1, 'requester closure time')
    integer(until, requester_closed_at + 1, address.U64, 'original observation cutoff')
    need(type(worker) is str and len(worker) == 32
         and all(c in '0123456789abcdef' for c in worker), 'bound worker identity')
    need(type(samples) is list and 2 <= len(samples) <= 32, 'bounded repeated observations')
    previous = requester_closed_at
    for sample in samples:
        need(type(sample) is dict and sample.get('worker') == worker,
             'same persistent worker, no replacement')
        need(sample.get('retained_worker_pidfd_live') is True,
             'retained worker must still be observed live')
        need(existing_identity(sample.get('profile'), selected), 'profile remains configured')
        previous = integer(sample.get('observed_at_ns'), previous + 1, until - 1,
                           'fresh monotonic observation after actual requester closure')
    return previous


def independent_connected_readback(*, selected, address_body, address_receipt,
                                   armed, until, **l2):
    """Reuse actual address and station/AP parsers; IP alone never suffices."""
    need(type(address_body) is bytes and len(address_body) <= 262144, 'bounded raw address readback')
    address.receipt(address_receipt,
                    expected_argv=['/usr/bin/ip', '-j', 'address', 'show', 'dev', selected['name']],
                    until=until, armed=armed, code=0,
                    byte_count=len(address_body), limit=262144)
    profile = address.kernel_profile(address_body, selected=selected, configured=True)
    need(l2['selected_station']['ifindex'] == selected['ifindex']
         and l2['selected_station']['name'] == selected['name']
         and l2['namespace_binding']['station_namespace'] == selected['namespace'],
         'radio and profile select the same interface and namespace')
    radio = address.l2_observation(armed=armed, until=until, **l2)
    return {'profile': profile, 'radio': radio}


def explicit_disconnect_order(*, requested_at, original_until, profile_removed_at,
                              disconnect_submitted_at, radio_disconnected_at):
    """Recorded operation order only; never infer radio cleanup from process exit."""
    integer(requested_at, 1, address.U64 - address.ACTION_NS, 'explicit Disconnect request')
    integer(original_until, requested_at + 1, requested_at + address.ACTION_NS,
            'unchanged action cutoff')
    integer(profile_removed_at, requested_at, original_until - 1, 'profile removal readback')
    integer(disconnect_submitted_at, profile_removed_at, original_until - 1,
            'radio Disconnect follows profile removal')
    integer(radio_disconnected_at, disconnect_submitted_at, original_until - 1,
            'radio readback within original cutoff')
