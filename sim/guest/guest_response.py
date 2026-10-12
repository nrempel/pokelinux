"""Responses for the disposable guest test harness, not the product protocol."""
import traceback


def run_probe(identity, probe):
    try:
        return {'id': identity, 'outcome': 'OK', 'value': probe()}
    except (OSError, ValueError) as error:
        return {'id': identity, 'outcome': 'UNAVAILABLE',
                'error': type(error).__name__, 'calls': []}
    except Exception as error:
        traceback.print_exc()
        return {'id': identity, 'outcome': 'FAILED', 'error': type(error).__name__}
