#!/usr/bin/env python3
"""Native Wayland test window, with observable real GDK touch events."""
import json
import os
import gi
gi.require_version('Gtk', '3.0')
gi.require_version('Gdk', '3.0')
from gi.repository import Gdk, Gtk

state = {'colour': (1.0, 0.0, 0.0), 'ready': False}
window = Gtk.Window(title='Pixel graphics touch test')
window.fullscreen()
area = Gtk.DrawingArea()
area.add_events(Gdk.EventMask.TOUCH_MASK)


def record(kind):
    with open(os.environ['PIXEL_TOUCH_EVENTS'], 'a') as output:
        output.write(json.dumps({'event': kind}) + '\n')


def draw(widget, context):
    context.set_source_rgb(*state['colour'])
    context.paint()
    if not state['ready']:
        state['ready'] = True
        record('ready')


def touch(widget, event):
    if event.type == Gdk.EventType.TOUCH_BEGIN:
        state['colour'] = (0.0, 1.0, 0.0)
        record('touch_down')
    elif event.type == Gdk.EventType.TOUCH_END:
        state['colour'] = (0.0, 0.0, 1.0)
        record('touch_up')
    elif event.type == Gdk.EventType.TOUCH_CANCEL:
        record('touch_cancel')
    elif event.type == Gdk.EventType.TOUCH_UPDATE:
        record('touch_motion')
    area.queue_draw()
    return True


area.connect('draw', draw)
area.connect('touch-event', touch)
window.connect('destroy', Gtk.main_quit)
window.add(area)
window.show_all()
Gtk.main()
