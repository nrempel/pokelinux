"""Read the live call collection from this Android test image's dumpsys."""
import re


def live_call_states(dump):
    section = re.search(r'^  mCalls:\s*\n(.*?)^  mCallAudioManager:', dump,
                        re.M | re.S)
    if section is None:
        raise ValueError('Telecom live call collection is missing')
    return re.findall(r'^\s*\[Call id=[^,]+, state=([A-Z_]+),', section[1], re.M)
