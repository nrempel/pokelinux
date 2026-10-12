"""Read Omarchy's public palette as bounded data; never execute theme files."""
import re
import tomllib
from pathlib import Path
DEFAULT={'background':'#101316','foreground':'#eceff1','accent':'#f2b755','red':'#9c3b3b'}
def palette(data):
    if len(data)>65536:raise ValueError('Theme too large')
    value=tomllib.loads(data.decode('utf-8'));result=dict(DEFAULT)
    for key,legacy in (('background','bg'),('foreground','fg'),('accent','color4'),('red','color1')):
        color=value.get(key,value.get(legacy,DEFAULT[key]))
        if not isinstance(color,str) or not re.fullmatch('#[0-9a-fA-F]{6}',color):raise ValueError('Invalid palette color')
        result[key]=color.lower()
    return result

def theme_path(config,state):
    new=Path(state)/'omarchy/current/theme/colors.toml'
    return new if new.is_file() else Path(config)/'omarchy/current/theme/colors.toml'

def contrast(color):
    channels=[int(color[i:i+2],16)/255 for i in (1,3,5)]
    linear=[v/12.92 if v<=.04045 else ((v+.055)/1.055)**2.4 for v in channels]
    light=sum(v*w for v,w in zip(linear,(.2126,.7152,.0722)))
    return '#000000' if (light+.05)/.05>=1.05/(light+.05) else '#ffffff'

def stylesheet(colors):
    return ('''
window { background: %(background)s; color: %(foreground)s; }
label { font-family: sans-serif; font-size: 16px; }
.title { font-size: 25px; font-weight: bold; }
.muted { color: %(foreground)s; }
button, entry { background: %(background)s; color: %(foreground)s; border: 1px solid %(foreground)s; box-shadow: none; text-shadow: none; }
entry { padding: 10px; font-size: 16px; }
entry:focus { border-color: %(accent)s; }
entry selection { background: %(accent)s; color: %(accent_text)s; }
button { min-height: 28px; padding: 10px 14px; border-radius: 6px; }
button.action { background: %(accent)s; color: %(accent_text)s; }
button.destructive { background: %(red)s; color: %(red_text)s; }
button:disabled { opacity: 0.45; }
''' % dict(colors,accent_text=contrast(colors['accent']),red_text=contrast(colors['red']))).encode()
