#!/usr/bin/env python3
"""Test-only GTK observer. Imports the installed production application unchanged."""
import hashlib,json,sys,time
from pathlib import Path
sys.path.insert(0,'/usr/lib/omarchy-phone')
from phone import PhoneApplication,Gtk,GLib,Gio
root=Path('/results');application=PhoneApplication(Path('/tmp/phone-client.json'));window=None;last=None;activations=0

def widgets(parent):
    yield parent
    if isinstance(parent,Gtk.Container):
        for child in parent.get_children():yield from widgets(child)

def poll():
    global last
    try:request=json.loads((root/'gui-request.json').read_text())
    except (FileNotFoundError,json.JSONDecodeError):return True
    if request['id']==last:return True
    last=request['id'];response={'id':last}
    if request['op']=='inspect':
        response.update(labels=[w.get_text() for w in widgets(window) if isinstance(w,Gtk.Label) and w.get_mapped()],
                        buttons=[{'label':w.get_label(),'enabled':w.get_sensitive()} for w in widgets(window) if isinstance(w,Gtk.Button) and w.get_mapped()],
                        dial_snapshot=window.model.dial_snapshot,dial_pending=window.model.dial_pending,dial_outcome=window.model.dial_outcome,number=window.number.get_text(),snapshot=window.model.snapshot,pending=window.model.pending,outcome=window.model.outcome,app_id=application.get_application_id(),windows=len(application.get_windows()),activations=activations,theme=window.theme_bytes.decode())
    elif request['op']=='launch':
        entry=Gio.DesktopAppInfo.new('omarchy-phone.desktop')
        assert entry is not None and entry.get_executable()=='omarchy-phone'
        response['launched']=entry.launch([],None)
    elif request['op']=='theme':
        import os
        path=Path(os.environ.get('XDG_STATE_HOME',str(Path.home()/'.local/state')))/'omarchy/current/theme/colors.toml'
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_text('background="#1a1b26"\nforeground="#a9b1d6"\naccent="#33cc99"\nred="#f7768e"\n')
        response['theme_changed']=True
    elif request['op']=='geometry':
        buttons=[w for w in widgets(window) if isinstance(w,(Gtk.Button,Gtk.Expander)) and w.get_label()==request['button'] and w.get_mapped() and w.get_sensitive()]
        if len(buttons)!=1:response['error']='Expected one enabled button'
        else:
            button=buttons[0]
            # GtkExpander responds on its arrow/label, not the blank header width.
            target=button.get_label_widget() if isinstance(button,Gtk.Expander) else button
            x,y=target.translate_coordinates(window,0,0)
            response['allocation']=[x,y,target.get_allocated_width(),target.get_allocated_height()]
            response['center']=[x+target.get_allocated_width()//2,y+target.get_allocated_height()//2]
    elif request['op']=='quit':window.destroy()
    else:response['error']='Unknown test operation'
    temp=root/'gui-response.tmp';temp.write_text(json.dumps(response));temp.replace(root/'gui-response.json')
    return True
def activated(app):
    global window,activations
    window=app.window;activations+=1
    if activations==1:
        GLib.timeout_add(100,poll)
        (root/'gui-ready.json').write_text(json.dumps({'installed_files':{name:hashlib.sha256((Path('/usr/lib/omarchy-phone')/name).read_bytes()).hexdigest() for name in ('phone.py','phone_model.py','phone_theme.py','omarchy_bridge.py','enroll.py')}}))
application.connect_after('activate',activated)
application.run(['omarchy-phone-test'])
