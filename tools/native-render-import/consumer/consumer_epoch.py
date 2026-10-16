"""Join one frozen real-row oracle to the unchanged finite owner and expected RED/GREEN epoch."""
import json
from consumer_oracle import classify as semantic


def classify(result, actual_exit, epoch):
    assert epoch in ['RED', 'GREEN'] and result['schema'] == 1
    assert result['setup_failure'] is None and result['setup_qualified'] and result['cleanup_errors'] == []
    assert result['actual_owned_epoch_closed'] and result['ECHILD'] and result['all_original_waits_and_EOF_qualified']
    outcome = semantic(result)
    assert outcome['qualified'] and json.loads(json.dumps(outcome)) == result['oracle_classification']
    if epoch == 'RED':
        assert actual_exit == 1 and not outcome['cursor_consumer_pixels_correct'], 'RED requires actual consumer row mismatch after setup and closure'
    else:
        assert actual_exit == 0 and outcome['cursor_consumer_pixels_correct'], 'GREEN requires identical original-row expectation'
    return {'epoch': epoch, 'expected_epoch_qualified': True, 'actual_case_count': 1,
            'cursor_consumer_pixels_correct': outcome['cursor_consumer_pixels_correct'],
            'classification': outcome['classification'],
            'assigned_SHM_bytes_and_protocol_frame_callback_qualified': True,
            'Source_authority_GPU_output_present_or_phone_qualified': False}
