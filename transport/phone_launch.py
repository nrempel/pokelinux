#!/usr/bin/env python3
"""Open the native Phone panel without changing plugin or call permissions."""
import json
import subprocess
import sys
import time


class LaunchError(Exception):
    pass


def shell(*arguments):
    return subprocess.check_output(['omarchy-shell','shell',*arguments],text=True,
                                   stderr=subprocess.PIPE,timeout=5).strip()


def launch(invoke=shell, *, clock=time.monotonic, sleep=time.sleep):
    try:
        plugins=json.loads(invoke('listPlugins'))
        if not isinstance(plugins,list):
            raise ValueError('Invalid plugin list')
        phone=next((p for p in plugins if isinstance(p,dict) and p.get('id')=='nbrempel.phone'),None)
        if phone is None:
            raise LaunchError('Phone plugin is not registered. Install the native plugin package, then run omarchy-phone-plugin register and reload the Omarchy shell.')
        if phone.get('enabled') is not True:
            raise LaunchError('Phone plugin is disabled. Enable Phone in Omarchy plugin controls.')
        if invoke('summon','nbrempel.phone','{}')!='ok':
            raise LaunchError('The Omarchy shell could not open Phone. Check that the Phone plugin is enabled.')
        deadline=clock()+3
        while True:
            try:
                state=json.loads(invoke('call','nbrempel.phone','status',''))
            except ValueError:
                state=None  # The asynchronous QML loader may not be ready yet.
            if isinstance(state,dict) and state.get('opened') is True:
                return
            remaining=deadline-clock()
            if remaining<=0:
                raise LaunchError('Phone panel could not open. Reload the Omarchy shell and check the Phone plugin installation.')
            sleep(min(.2,remaining))
    except (OSError,subprocess.SubprocessError,ValueError) as error:
        raise LaunchError('Omarchy shell is unavailable. Start or reload the Omarchy shell, then open Phone again.') from error


def main():
    try:
        launch()
    except LaunchError as error:
        message=str(error)
        print(message,file=sys.stderr)
        try:
            subprocess.run(['notify-send','--app-name=Phone','--urgency=normal','Phone unavailable',message],
                           check=False,timeout=5,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        except (OSError,subprocess.SubprocessError):
            pass
        return 1
    return 0


if __name__=='__main__':
    sys.exit(main())
