"""Explicit success-cleanup validation; no action or observation is performed here."""
import json
import consumer


def observe(returncode, stdout, receipt, *, executable, armed_at_ns, station, ap, kernel):
    result = {'schema': 1, 'qualified': False}
    try:
        consumer.selectors(station, ap)
        until = armed_at_ns + consumer.ACTION_NS
        consumer.need(type(stdout) is bytes and 0 < len(stdout) <= consumer.MAX_WIRE, 'disconnect wire bound')
        consumer.actual_receipt(receipt,
            argv=[executable, 'disconnect', str(station['ifindex']), str(until)],
            until=until, admission=armed_at_ns, code=0, byte_count=len(stdout), limit=consumer.MAX_WIRE)
        consumer.need(returncode == 0, 'actual disconnect exit')
        value = json.loads(stdout.decode(), object_pairs_hook=consumer.unique, parse_constant=consumer.no_constant)
        consumer.need(type(value) is dict and set(value) == {'schema', 'status', 'operation', 'ifindex',
            'original_until_ns', 'wiphy', 'wdev', 'started_boottime_ns', 'completed_boottime_ns'}, 'disconnect result shape')
        consumer.need(type(value['schema']) is int and value['schema'] == 1
            and value['status'] == 'disconnected' and value['operation'] == 'disconnect', 'disconnect result status')
        for key, expected in [('ifindex', station['ifindex']), ('wiphy', station['wiphy']),
                              ('wdev', station['wdev']), ('original_until_ns', until)]:
            consumer.need(type(value[key]) is int and value[key] == expected, 'disconnect identity: ' + key)
        start = consumer.integer(value['started_boottime_ns'], armed_at_ns, until - 1, 'disconnect start')
        done = consumer.integer(value['completed_boottime_ns'], start, until - 1, 'disconnect completion')
        consumer.need(done <= min(receipt['waited_at_boottime_ns'], receipt['stdout_EOF_at_ns']), 'disconnect completed after actual wait/EOF')
        consumer.need(kernel.get('kernel_observation_confirmed') is True and kernel.get('station_unassociated') is True
            and kernel.get('original_until_ns') == until and kernel.get('requested_station') == station
            and kernel.get('selected_AP') == ap, 'independent disconnection kernel readback')
        result.update(qualified=True, actual_result=value, actual_returncode=returncode,
                      original_until_ns=until, kernel=kernel)
    except (ValueError, TypeError, KeyError, UnicodeError, OverflowError, RecursionError) as error:
        result['unconfirmed_detail'] = str(error)
    return result
