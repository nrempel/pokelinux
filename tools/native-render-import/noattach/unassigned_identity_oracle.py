#!/usr/bin/env python3
"""Oracle for actual owned unassigned-surface runtime receipts, never synthetic RED.
Input records must be collected by the existing native owner, in one BOOTTIME epoch.
The stable identity expectation is unchanged for baseline and repaired candidate.
"""
import json
import sys


def classify(record):
    try:
        pid = int(record['client_pid'])
        surface_id = int(record['surface_id'])
        assert pid > 0 and surface_id > 0
        events = []
        for key, expected in [('initial_flushed', 'profile_unassigned_initial_flushed'),
                              ('initial_synced', 'profile_unassigned_initial_synced'),
                              ('no_attach_flushed', 'profile_unassigned_no_attach_flushed'),
                              ('no_attach_synced', 'profile_unassigned_no_attach_synced')]:
            event = record[key]
            assert event['event'] == expected and int(event['pid']) == pid and int(event['uid']) == 1000
            assert int(event['value']) == surface_id and int(event['at_ns']) > 0
            events.append(int(event['at_ns']))
        before, after = record['before'], record['after']
        retained_fields = ('resource_id', 'generation', 'commit_sequence', 'pixel_bytes')
        current_fields = ('current_resource_id', 'current_generation', 'current_commit_sequence', 'current_pixel_bytes')
        for row in [before, after]:
            assert row['ok'] is True and row['kind'] == 'unassigned'
            assert int(row['peer_pid']) == pid and int(row['uid_verified']) == 1000 and int(row['surface_id']) == surface_id
            assert row['live_checked'] is True and row['current_buffer_retained'] is True
            assert row['client_buffer_released'] is False
            assert row['original_pattern_every_pixel'] is True and row['current_original_pattern_every_pixel'] is True
            assert row['texture_ok'] is True  # object/GLuint validity only; no upload/presentation claim
            assert (int(row['width']), int(row['height']), int(row['stride']), int(row['pixel_bytes'])) == (64, 64, 256, 16384)
            assert int(row['held_handles']) == 1
            assert all(int(row[name]) > 0 for name in retained_fields + current_fields)
        assert events[0] <= events[1] <= int(before['at_ns']) <= events[2] <= events[3] <= int(after['at_ns'])
        original = tuple(int(before[name]) for name in retained_fields)
        assert tuple(int(after[name]) for name in retained_fields) == original
        assert tuple(int(before[name]) for name in current_fields) == original
        assert before['current_description_matches_retained'] is True
        close = record['witness_close']
        assert close['ok'] is True and int(close['released_handles']) == 1 and int(close['held_handles']) == 0
        assert record['actual_owned_epoch_closed'] is True  # existing owner wait/EOF/fd/runtime closure receipt
        current = tuple(int(after[name]) for name in current_fields)
        stable = current == original and after['current_description_matches_retained'] is True
        return {'qualified': True, 'identity_stable': stable,
                'classification': 'property-pass' if stable else 'actual-no-attach-identity-red',
                'before_current_identity': original, 'after_current_identity': current,
                'gpu_upload_or_presentation_qualified': False}
    except (AssertionError, KeyError, ValueError, TypeError):
        return {'qualified': False, 'classification': 'setup-or-closure-failure',
                'gpu_upload_or_presentation_qualified': False}


if __name__ == '__main__':
    with open(sys.argv[1], encoding='utf-8') as source:
        result = classify(json.load(source))
    print(json.dumps(result, sort_keys=True))
    raise SystemExit(2 if not result['qualified'] else 0 if result['identity_stable'] else 1)
