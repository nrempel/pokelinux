#!/usr/bin/env python3
"""Frozen semantic expectation for real assigned SHM plus the linked padded-cursor consumer.
Authored source only until an actual native run supplies these records. CPU Cairo paint
is not compositor readback, GPU upload, a software-cursor output or presentation.
"""
import json
import sys


def classify(r):
    try:
        assert r['schema'] == 1 and r['setup_failure'] is None and r['setup_qualified'] is True
        assert r['Source_or_Observer_selected'] is False and r['phone_accessed'] is False
        assert r['actual_owned_epoch_closed'] and r['ECHILD'] and r['all_original_waits_and_EOF_qualified']
        assert r['cleanup_errors'] == [] and all(x['pidfd_dead'] and x['proc_absent'] for x in r['descendant_closure'])
        assert r['Pixel10_profile']['qualification']['qualified'] and not r['Pixel10_profile']['hardware_verified']
        pid, comp = int(r['client_pid']), int(r['compositor_PID'])
        window_id, cursor_id = int(r['window_surface_id']), int(r['cursor_surface_id'])
        assert pid > 0 and comp > 0 and window_id > 0 and cursor_id > 0 and window_id != cursor_id
        assert r['focus_reply'] == 'ok' and r['active_window']['class'] == 'shm-boundary-client'
        assert int(r['active_window']['pid']) == pid and r['active_window_covers_selected_output_center'] is True
        for key, expected, value in [
            ('virtual_pointer_created', 'profile_virtual_pointer_created', r['virtual_pointer_id']),
            ('pointer_motion_flushed', 'profile_pointer_motion_flushed', r['virtual_pointer_id']),
            ('pointer_motion_synced', 'profile_pointer_motion_synced', r['virtual_pointer_id']),
            ('pointer_enter', 'profile_pointer_enter', 1),
            ('pointer_enter_serial', 'profile_pointer_enter_serial', r['pointer_serial']),
            ('assigned_buffer_committed', 'profile_assigned_buffer_committed', r['assigned_buffer_id']),
            ('assigned_buffer_released', 'buffer_release', r['assigned_buffer_id']),
            ('cursor_buffer_committed', 'profile_cursor_buffer_committed', r['cursor_buffer_id']),
            ('cursor_buffer_released', 'buffer_release', r['cursor_buffer_id']),
            ('assigned_flushed', 'profile_assigned_flushed', window_id),
            ('assigned_synced', 'profile_assigned_synced', window_id),
            ('assigned_frame_done', 'profile_assigned_frame_done', window_id),
            ('cursor_request_queued', 'profile_cursor_request_queued', r['pointer_serial']),
            ('cursor_flushed', 'profile_cursor_flushed', cursor_id),
            ('cursor_synced', 'profile_cursor_synced', cursor_id)]:
            e = r[key]
            assert e['event'] == expected and int(e['pid']) == pid and int(e['uid']) == 1000
            assert int(e['value']) == int(value) and int(e['at_ns']) > 0
        assert int(r['virtual_pointer_id']) > 0 and int(r['pointer_serial']) > 0
        assert any(e['event'] == 'profile_seat_capabilities' and e['pid'] == pid and e['uid'] == 1000 and int(e['value']) & 1 for e in r['seat_capability_events'])
        assert r['pointer_motion_request']['parent_send_after_ns'] <= r['pointer_motion_flushed']['at_ns'] <= r['pointer_motion_synced']['at_ns']
        assert r['virtual_pointer_created']['at_ns'] <= r['pointer_enter']['at_ns'] <= r['cursor_request']['parent_send_before_ns']
        assert r['pointer_enter_serial']['at_ns'] <= r['pointer_enter']['at_ns']
        assert r['assigned_request']['parent_send_after_ns'] <= r['assigned_flushed']['at_ns'] <= r['assigned_synced']['at_ns']
        assert r['assigned_request']['parent_send_before_ns'] <= r['assigned_frame_done']['at_ns'] < r['actual_client_closure']['checked_at_ns']
        assert r['cursor_request']['parent_send_after_ns'] <= r['cursor_flushed']['at_ns'] <= r['cursor_synced']['at_ns']
        assert r['cursor_request_queued']['value'] == r['pointer_serial']
        assert 0 < int(r['assigned_buffer_id']) < int(r['cursor_buffer_id']) <= 8
        assert r['assigned_buffer_committed']['at_ns'] <= r['assigned_buffer_released']['at_ns'] < int(r['window_before']['at_ns'])
        assert r['cursor_buffer_committed']['at_ns'] <= r['cursor_buffer_released']['at_ns'] < int(r['cursor_before']['at_ns'])
        fields = ('resource_id', 'generation', 'commit_sequence', 'width', 'height', 'stride', 'drm_format', 'pixel_bytes')
        retained_identity = {}
        for kind, surface_id in [('window', window_id), ('cursor', cursor_id)]:
            first, current, retained = r[kind + '_before'], r[kind + '_after'], r[kind + '_retained_after_client_exit']
            retained_identity[kind] = [int(first[x]) for x in fields]
            for a in [first, current, retained]:
                assert a['ok'] is True and a['kind'] == kind and int(a['peer_pid']) == pid
                assert int(a['uid_verified']) == 1000 and int(a['surface_id']) == surface_id
                assert a['original_pattern_every_pixel'] is True and a['padding_original'] is True
                assert all(int(a[x]) > 0 for x in fields)
                assert [int(a[x]) for x in fields] == retained_identity[kind]
            for a in [first, current]:
                assert a['live_checked'] is True and a['texture_ok'] is True and a['client_buffer_released'] is True
                if kind == 'cursor':
                    assert a['selected_pointer_role'] and a['cursor_cache_equals_padded_snapshot']
                    assert a['production_consumer_invoked'] and a['cairo_setup_ok']
                    assert int(a['consumer_pixels_checked']) == 1024 and int(a['consumer_output_stride']) == 128
                    assert 0 <= int(a['consumer_mismatches']) <= 1024 and int(a['consumer_input_stride']) > 0
                    assert a['consumer_original_pattern_every_pixel'] is (int(a['consumer_mismatches']) == 0)
                    if a['consumer_mismatches']:
                        assert 0 <= int(a['first_mismatch_x']) < 32 and 0 <= int(a['first_mismatch_y']) < 32
                        assert int(a['first_mismatch_actual']) != int(a['first_mismatch_expected'])
                else:
                    assert not a['production_consumer_invoked'] and int(a['stride']) == int(a['width'])*4
                    assert int(a['drm_format']) == 875713112 and int(a['pixel_bytes']) == int(a['stride'])*int(a['height'])
            if kind == 'cursor':
                assert tuple(int(first[x]) for x in ('width', 'height', 'stride', 'drm_format', 'pixel_bytes')) == (32, 32, 144, 875713089, 4608)
            assert retained['live_checked'] is False and int(retained['held_handles']) == 2
            assert int(retained['at_ns']) >= r['actual_client_closure']['checked_at_ns']
        assert int(r['window_before']['at_ns']) >= r['assigned_synced']['at_ns']
        assert int(r['cursor_before']['at_ns']) >= r['cursor_synced']['at_ns']
        assert int(r['cursor_after']['at_ns']) >= int(r['cursor_before']['at_ns'])
        assert r['window_before']['held_handles'] == 1 and r['cursor_before']['held_handles'] == 2
        assert all(r[x]['held_handles'] == 2 for x in ['window_after', 'cursor_after'])
        assert len(r['witness_command_receipts']) == 7
        for w, key in zip(r['witness_command_receipts'], ['window_before', 'cursor_before', 'window_after', 'cursor_after', 'window_retained_after_client_exit', 'cursor_retained_after_client_exit', 'witness_close']):
            assert w['response'] == r[key]
            assert (int(w['actual_peer_PID']), int(w['actual_peer_UID']), int(w['actual_peer_GID'])) == (comp, 1000, 1000)
            assert w['send_before_ns'] <= w['reply_after_ns'] < w['original_cutoff_ns'] == r['original_witness_until_ns']
            assert w['response']['ok'] is True
        for key, expected_pid in [('actual_client_closure', pid), ('actual_compositor_closure', comp)]:
            a = r[key]
            assert int(a['PID']) == expected_pid and a['pidfd_dead'] and a['proc_absent']
            assert a['actual_wait_exit'] in ([0] if key == 'actual_client_closure' else [0, -15, -9])
            assert a['checked_at_ns'] < a['original_cutoff_ns']
        for key, op in [('pointer_motion_request', 'P'), ('assigned_request', 'A'), ('cursor_request', 'C'), ('actual_Q_request', 'Q')]:
            a = r[key]
            assert a['operation'] == op and int(a['client_PID']) == pid and 0 < a['parent_send_before_ns'] <= a['parent_send_after_ns']
        closed = r['actual_client_closure']
        assert closed['disconnect']['event'] == 'client_disconnected' and closed['disconnect']['pid'] == pid and closed['disconnect']['uid'] == 1000
        assert r['actual_Q_request']['parent_send_before_ns'] <= closed['disconnect']['at_ns'] <= closed['checked_at_ns']
        assert r['witness_close'] == {'ok': True, 'released_handles': 2, 'held_handles': 0}
        for kind in ['window', 'cursor']:
            assert int(r[kind + '_retained_after_client_exit']['at_ns']) < r['original_witness_until_ns']
        # The only semantic expectation that may fail in a qualified RED epoch.
        correct = all(a['consumer_original_pattern_every_pixel'] and int(a['consumer_mismatches']) == 0
                      for a in [r['cursor_before'], r['cursor_after']])
        return {'qualified': True, 'cursor_consumer_pixels_correct': bool(correct),
                'classification': 'property-pass' if correct else 'actual-padded-cursor-consumer-row-red',
                'assigned_SHM_bytes_and_protocol_frame_callback_qualified': True,
                'retained_metadata': retained_identity,
                'GPU_upload_output_readback_or_presentation_qualified': False,
                'full_Hardware_or_software_cursor_path_qualified': False}
    except (AssertionError, KeyError, TypeError, ValueError, OverflowError):
        return {'qualified': False, 'classification': 'setup-or-closure-failure',
                'GPU_upload_output_readback_or_presentation_qualified': False,
                'full_Hardware_or_software_cursor_path_qualified': False}


if __name__ == '__main__':
    with open(sys.argv[1], encoding='utf-8') as source:
        outcome = classify(json.load(source))
    print(json.dumps(outcome, sort_keys=True))
    raise SystemExit(2 if not outcome['qualified'] else 0 if outcome['cursor_consumer_pixels_correct'] else 1)
