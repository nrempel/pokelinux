#!/usr/bin/env python3
"""Register package-owned Phone plugin files for the current Omarchy user."""
import argparse
import json
import os
from pathlib import Path

SOURCE = Path('/usr/share/omarchy-phone/plugin')


def owned_link(source, target):
    return target.is_symlink() and Path(os.readlink(target)) == source


def register(source, target):
    source = source.absolute()
    if json.loads((source/'manifest.json').read_text()).get('id') != 'nbrempel.phone':
        raise ValueError('Package source is not the Phone plugin')
    target.parent.mkdir(parents=True, exist_ok=True)
    if owned_link(source, target):
        return
    # symlink creation is atomic and refuses an existing file, directory or link.
    target.symlink_to(source, target_is_directory=True)


def unregister(source, target):
    source = source.absolute()
    if owned_link(source, target):
        target.unlink()
    elif target.exists() or target.is_symlink():
        raise FileExistsError(f'Refusing to remove user-owned path: {target}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=('register', 'unregister'))
    args = parser.parse_args()
    if os.geteuid() == 0:
        parser.error('Run as your desktop user, without sudo')
    # Omarchy itself uses HOME/.config, including when XDG_CONFIG_HOME differs.
    target = Path.home()/'.config/omarchy/plugins/nbrempel.phone'
    try:
        (register if args.action == 'register' else unregister)(SOURCE, target)
    except (OSError, ValueError) as error:
        parser.exit(1, f'{error}\n')
    print(f'Phone plugin {args.action} complete. Reload the Omarchy shell to apply.')


if __name__ == '__main__':
    main()
