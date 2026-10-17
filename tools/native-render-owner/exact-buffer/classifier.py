"""Pure frozen oracles. Source preparation never supplies fabricated run records."""
import errno
import json
import re

MISSING = {
    4: ('SExactBufferRequest', None),
    5: ('reconfigureExact', 'CSwapchain'),
    6: ('exactAllocation', 'CGBMBuffer'),
    7: ('createExact', 'CDRMFB'),
    8: ('exactImport', 'CDRMFB'),
    9: ('closeExact', 'CDRMFB'),
}
GROUPS = (
    'actual_implicit_allocation_raw_export',
    'actual_cap0_explicit_refusal_no_retry',
    'positive_buffer_request_mismatch_refusals',
    'actual_implicit_import_getfb2_same_buffer',
    'owned_never_attached_close_and_buffer_release',
)

def structural(control_code, control_stdout, control_stderr,
               target_code, target_stdout, target_stderr, target_path):
    assert control_code == 0 and control_stdout == b'' and control_stderr == b''
    assert target_code == 1 and target_stdout == b''
    rows = target_stderr.decode('utf-8').replace('\u2018', "'").replace('\u2019', "'").splitlines()
    assert len(rows) == len(MISSING)
    found = []
    for text in rows:
        match = re.fullmatch(re.escape(target_path) + r':([0-9]+):([0-9]+): error: (.+)', text)
        assert match
        line, column, message = int(match[1]), int(match[2]), match[3]
        assert line in MISSING and column > 0
        name, owner = MISSING[line]
        if owner is None:
            expected = "'SExactBufferRequest' in namespace 'Aquamarine' does not name a type"
        else:
            expected = f"'{name}' is not a member of 'Aquamarine::{owner}'"
        # Only GCC suggestions naming genuine selected existing SDK declarations.
        suggestion = None
        if '; did you mean ' in message:
            message, suffix = message.split('; did you mean ', 1)
            hint = re.fullmatch(r"'([A-Za-z_][A-Za-z0-9_]*)'\?", suffix)
            assert hint
            suggestion = hint[1]
            allowed = {'reconfigureExact': {'reconfigure'}, 'createExact': {'create'},
                       'closeExact': {'closeHandles'}, 'SExactBufferRequest': {'SAllocatorBufferParams'}}
            assert suggestion in allowed.get(name, set())
        assert message == expected
        found.append({'line': line, 'column': column, 'missing': name, 'suggestion': suggestion})
    assert {r['line'] for r in found} == set(MISSING)
    return {'aggregate_passed': False, 'qualified_missing_interface_RED': True,
            'existing_control_exit': 0, 'target_exit': 1, 'missing': found,
            'allocation_import_behavior_qualified': False}

def kernel(body):
    records = {}
    for line in body.decode('utf-8').splitlines():
        if not line.startswith('EXACT_BUFFER_'):
            continue
        label, raw = line.split(' ', 1)
        value = json.loads(raw)
        assert isinstance(value, dict)
        records.setdefault(label, []).append(value)
    assert 'EXACT_BUFFER_FAILURE' not in records
    def one(label):
        rows = records['EXACT_BUFFER_' + label]
        assert len(rows) == 1
        return rows[0]
    result = one('RESULT')
    assert result['operation_cases_passed'] is True and result['groups'] == 5
    assert result['worker_tid'] > 0 and result['control_tid'] > 0 and result['worker_tid'] != result['control_tid']
    assert result['stop_result'] == 0 and result['whole_native_retirement'] is False
    assert result['joined'] is False and result['process_disposal_required'] is True
    cases = records['EXACT_BUFFER_CASE']
    assert len(cases) == 5 and [r['name'] for r in cases] == list(GROUPS)
    assert all(r['passed'] is True and r['worker_tid'] == result['worker_tid'] for r in cases)
    assert all(a['at_ns'] <= b['at_ns'] for a, b in zip(cases, cases[1:]))
    device, context = one('DEVICE'), one('CONTEXT')
    assert device['devices_equal'] == 1 and device['allocator_get'] == [0, 0] and device['kms_get'] == [0, 0]
    assert device['allocator_rdev'] > 0 and device['kms_rdev'] == context['rdev'] > 0
    assert context['cap_return'] == 0 and context['cap_errno'] == 0 and context['cap_value'] == 0
    assert all(context[k] > 0 for k in ('session', 'enumeration_sequence', 'lease', 'device_generation', 'connector'))
    allocation, imported, get, close = one('ALLOCATION'), one('IMPORT'), one('GETFB2'), one('CLOSE')
    assert allocation['create_count'] == allocation['getter_count'] == 1 and 1 <= allocation['planes'] <= 4
    attempted = one('ALLOCATION_ATTEMPT')
    assert attempted['result'] == 1 and attempted['api'] == 1 and attempted['create_count'] == attempted['getter_count'] == 1
    assert attempted['returned_buffer'] is True and attempted['export_success'] is True and attempted['create_errno'] == 0
    assert attempted['raw_modifier'] == allocation['raw_modifier'] and attempted['buffer_id'] == allocation['buffer_id']
    refusal = one('REFUSAL')
    assert refusal['result'] == 5 and refusal['cap_return'] == refusal['cap_value'] == refusal['api'] == refusal['create_count'] == refusal['getter_count'] == 0
    mismatches = records['EXACT_BUFFER_MISMATCH']
    assert len(mismatches) == 3 and [r['which'] for r in mismatches] == [0, 1, 2]
    assert mismatches[0]['result'] == mismatches[1]['result'] == 4 and mismatches[2]['result'] in (2, 4)
    assert all(r['prime_count'] == r['addfb_count'] == 0 for r in mismatches)
    assert allocation['buffer_id'] == imported['buffer_id'] == close['buffer_id'] > 0
    assert close['request_id'] == context['session']
    assert imported['framebuffer_id'] == get['framebuffer_id'] == close['framebuffer_id'] > 0
    assert imported['result'] == 1 and imported['flags'] == 0 and imported['with_modifiers'] is False
    assert get['width'] == get['height'] == 64 and get['format'] == 0x34325258
    assert imported['prime_count'] == allocation['planes'] and imported['addfb_count'] == 1
    assert imported['addfb_return'] == imported['addfb_errno'] == 0
    assert close['count'] == 1 and close['return'] == close['errno'] == 0
    exports = records['EXACT_BUFFER_EXPORT_PLANE']
    planes = records['EXACT_BUFFER_GETFB2_PLANE']
    assert len(exports) == allocation['planes'] and len(planes) == 4
    assert [r['plane'] for r in exports] == list(range(len(exports)))
    assert [r['plane'] for r in planes] == list(range(4))
    for i, p in enumerate(planes):
        if i < len(exports):
            assert p['pitch'] == exports[i]['pitch'] > 0 and p['offset'] == exports[i]['offset']
        else:
            assert p['pitch'] == 0
    handles = {p['handle'] for p in planes if p['handle']}
    assert len(handles) == get['unique_fresh_handles']
    closed = records.get('EXACT_BUFFER_GETFB2_HANDLE', [])
    assert len(closed) == len(handles) and {p['handle'] for p in closed} == handles
    assert all(p['return'] == 0 for p in closed)
    absent = one('REGISTRATION_ABSENT')
    assert absent['framebuffer_id'] == close['framebuffer_id'] and absent['errno'] == errno.ENOENT
    fdclosed = records['EXACT_BUFFER_EXPORT_CLOSED']
    assert len(fdclosed) == len(exports)
    for a, b in zip(exports, fdclosed):
        assert a['plane'] == b['plane'] and a['fd'] == b['fd'] and b['return'] == -1 and b['errno'] == errno.EBADF
    return {'operation_cases_passed': True, 'groups': list(GROUPS),
            'whole_native_retirement': False, 'worker_join_qualified': False,
            'external_direct_wait_EOF_and_process_disposal_still_required': True,
            'frame_presentation_or_nativeReady_qualified': False}
