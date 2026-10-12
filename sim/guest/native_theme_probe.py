"""Verify the real Omarchy theme command using compositor pixels."""
from pathlib import Path
import time
import tomllib
from PIL import Image


def exercise(out, user, run, phone_window):
    selected = Path('/home/alarm/.local/state/omarchy/current/theme')
    run(['install', '-d', '-o', 'alarm', '-g', 'alarm',
         str(selected.parent)])
    before = phone_window()
    assert len(before) == 1
    identity = (before[0]['pid'], before[0]['address'])
    captures = {}
    for name, theme_name in (('dark', 'tokyo-night'), ('light', 'catppuccin-latte')):
        output = run(user(['omarchy-theme-set', theme_name]))
        (out / f'theme-command-{name}.log').write_text(output)
        assert selected.with_name('theme.name').read_text().strip() == theme_name
        palette = tomllib.loads((selected / 'colors.toml').read_text())
        background = palette['background']
        expected = tuple(bytes.fromhex(background[1:]))
        capture = out / f'phone-theme-{name}.png'
        deadline = time.monotonic() + 45
        while True:
            windows = phone_window()
            assert len(windows) == 1
            window = windows[0]
            assert (window['pid'], window['address']) == identity, 'Theme change restarted the app'
            run(user(['grim', str(capture)]))
            x, y = window['at']
            width, height = window['size']
            # Empty lower body of the app: avoid text, controls and window borders.
            with Image.open(capture) as image:
                sample = image.convert('RGB').crop((x + 20, y + height - 60,
                                                  x + width - 20, y + height - 20))
                colors = dict((color, count) for count, color in sample.getcolors(sample.width * sample.height))
                fraction = colors.get(expected, 0) / (sample.width * sample.height)
            if fraction > .95:
                break
            if time.monotonic() >= deadline:
                raise AssertionError(f'{name} theme not rendered: expected {expected}; pixels {fraction}')
            time.sleep(.5)
        captures[name] = {'theme': theme_name, 'background': background, 'matching_pixel_fraction': fraction,
                          'window_pid': identity[0], 'window_address': identity[1],
                          'screenshot': capture.name}
    return captures
