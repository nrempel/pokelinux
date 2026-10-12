#!/usr/bin/env python3
"""Native GTK phone controls for an explicitly paired Android bridge."""
import argparse
import json
import os
import tempfile
import time
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import gi
gi.require_version('Gtk','3.0')
from gi.repository import Gtk,GLib,Gdk,Gio
from omarchy_bridge import Client,canonical_number
from phone_model import PhoneModel,Unavailable,elapsed_time
from phone_theme import DEFAULT,palette,stylesheet,theme_path

class PhoneWindow(Gtk.Window):
    def __init__(self,config=None):
        super().__init__(title='Omarchy Phone')
        self.set_default_size(400,580);self.set_size_request(320,400)
        self.model=PhoneModel();self.pool=ThreadPoolExecutor(max_workers=3,thread_name_prefix='phone-bridge')
        self.polling=False;self.closed=False;self.epoch=0;self.rendered=None
        self.settings=Path(os.environ.get('XDG_CONFIG_HOME',str(Path.home()/'.config')))/'omarchy-phone'
        self.config=Path(config) if config else self.settings/'client.json'
        if config is None:
            try:self.config=Path(json.loads((self.settings/'settings.json').read_text())['config'])
            except (OSError,ValueError,KeyError,TypeError):pass
        self.css=Gtk.CssProvider();self.theme_bytes=None;self.theme_checked=-100
        Gtk.StyleContext.add_provider_for_screen(Gdk.Screen.get_default(),self.css,Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION)
        self.reload_theme()
        outer=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=16);outer.set_border_width(20);self.add(outer)
        title=Gtk.Label(label='Omarchy Phone',xalign=0);title.get_style_context().add_class('title');outer.pack_start(title,False,False,0)
        self.connection=Gtk.Label(label='Connecting…',xalign=0);self.connection.set_line_wrap(True);outer.pack_start(self.connection,False,False,0)
        self.notice=Gtk.Label(xalign=0);self.notice.set_line_wrap(True);self.notice.get_style_context().add_class('muted');outer.pack_start(self.notice,False,False,0)
        scroll=Gtk.ScrolledWindow();scroll.set_policy(Gtk.PolicyType.NEVER,Gtk.PolicyType.AUTOMATIC)
        self.calls=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=16)
        self.body=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=16);self.body.pack_start(self.calls,False,False,0);scroll.add(self.body);outer.pack_start(scroll,True,True,0)
        dial=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=8)
        number_label=Gtk.Label(label='Phone number',xalign=0);dial.pack_start(number_label,False,False,0)
        row=Gtk.Box(spacing=10)
        self.number=Gtk.Entry();self.number.set_input_purpose(Gtk.InputPurpose.PHONE);self.number.set_max_length(33)
        self.number.get_accessible().set_name('Phone number');self.number.connect('changed',lambda *_:self.render_dial())
        row.pack_start(self.number,True,True,0)
        self.dial_button=Gtk.Button(label='Call');self.dial_button.get_style_context().add_class('action');self.dial_button.connect('clicked',self.dial)
        row.pack_start(self.dial_button,False,False,0);dial.pack_start(row,False,False,0)
        self.dial_notice=Gtk.Label(xalign=0);self.dial_notice.set_line_wrap(True);dial.pack_start(self.dial_notice,False,False,0)
        keypad=Gtk.Expander(label='Keypad');grid=Gtk.Grid(column_spacing=8,row_spacing=8);grid.set_column_homogeneous(True)
        for i,key in enumerate(('1','2','3','4','5','6','7','8','9','+','0','Delete')):
            button=Gtk.Button(label=key);button.set_size_request(-1,48);button.connect('clicked',self.keypad,key);grid.attach(button,i%3,i//3,1,1)
        keypad.add(grid);dial.pack_start(keypad,False,False,0);self.body.pack_start(dial,False,False,0)
        self.configure=Gtk.Button(label='Choose desktop pairing file');self.configure.connect('clicked',self.choose);outer.pack_start(self.configure,False,False,0)
        self.connect('destroy',self.close)
        self.show_all();self.timer=GLib.timeout_add(500,self.tick);self.tick()

    def client(self,path):
        if path.stat().st_mode&0o022:raise ValueError('Pairing file is writable by others')
        return Client(json.loads(path.read_text()))

    def reload_theme(self):
        path=theme_path(Path(os.environ.get('XDG_CONFIG_HOME',str(Path.home()/'.config'))),Path(os.environ.get('XDG_STATE_HOME',str(Path.home()/'.local/state'))))
        try:
            with path.open('rb') as source:data=source.read(65537)
            colors=palette(data)
        except (OSError,ValueError):colors=DEFAULT
        data=stylesheet(colors)
        if data!=self.theme_bytes:self.css.load_from_data(data);self.theme_bytes=data

    def tick(self):
        if self.closed:return False
        if elapsed_time()-self.theme_checked>=2:
            self.reload_theme();self.theme_checked=elapsed_time()
        self.render()
        if not self.polling:
            self.polling=True;epoch=self.epoch;path=self.config;started=elapsed_time()
            future=self.pool.submit(self.poll_phone,path)
            future.add_done_callback(lambda completed:GLib.idle_add(self.observed,epoch,started,completed))
        return True

    def observed(self,epoch,started,future):
        self.polling=False
        if self.closed or epoch!=self.epoch:return False
        try:
            controls,dial=future.result()
            if controls is None:self.model.disconnect()
            else:self.model.observe(controls,started,elapsed_time())
            if dial is not None:self.model.observe_dial(dial,started,elapsed_time())
            else:self.model.dial_snapshot=None;self.model.dial_update=None
        except (OSError,ValueError):self.model.disconnect()
        self.render();return False

    def poll_phone(self,path):
        client=self.client(path);values=[]
        for operation in (client.controls,client.dial_status):
            try:values.append(operation())
            except (OSError,ValueError):values.append(None)
        return values

    def keypad(self,button,key):
        if self.model.dial_pending:return
        text=self.number.get_text();selection=self.number.get_selection_bounds()
        start,end=selection if selection else (self.number.get_position(),self.number.get_position())
        if key=='Delete':
            if start==end:start=max(0,start-1)
            replacement=''
        else:replacement=key
        self.number.set_text(text[:start]+replacement+text[end:]);self.number.set_position(start+len(replacement));self.number.grab_focus_without_selecting()

    def render_dial(self):
        try:canonical_number(self.number.get_text());valid=True
        except ValueError:valid=False
        self.dial_button.set_sensitive(valid and self.model.can_dial(elapsed_time()))
        self.number.set_sensitive(not self.model.dial_pending)
        self.configure.set_sensitive(not self.model.pending and not self.model.dial_pending)
        reason=self.model.dial_snapshot['reason'] if self.model.dial_snapshot else 'unavailable'
        if self.model.dial_pending:message='Requesting call…'
        elif self.model.dial_outcome=='outcome_unknown':message='Call request outcome unknown. Check the phone before calling again.'
        elif reason=='pending':message='Waiting for the previous call request to finish.'
        elif self.model.dial_outcome=='not_invoked':message='Call was not placed: '+self.model.dial_reason.replace('_',' ')+'.'
        elif reason=='disabled':message='Enable desktop outgoing calls in Android Bridge settings.'
        elif reason=='ready':message='' if valid or not self.number.get_text() else 'Use 3 to 32 digits, with an optional leading +.'
        elif reason=='busy':message='Finish the current call to place another.'
        elif reason=='account_unavailable':message='Select a calling account on Android.'
        else:message='Outgoing calls unavailable.'
        self.dial_notice.set_text(message)

    def dial(self,button):
        try:command=self.model.begin_dial(self.number.get_text(),elapsed_time())
        except (Unavailable,ValueError):self.render_dial();return
        self.epoch+=1;path=self.config;self.render_dial()
        future=self.pool.submit(lambda:self.client(path).dial(**command))
        future.add_done_callback(lambda completed:GLib.idle_add(self.dial_finished,completed))

    def dial_finished(self,future):
        if self.closed:return False
        try:response=future.result()
        except (OSError,ValueError):response=None
        self.epoch+=1;self.model.finish_dial(response);self.render();return False

    def render(self):
        self.render_dial()
        fresh=self.model.fresh(elapsed_time())
        signature=self.model.view_key(elapsed_time())
        if signature==self.rendered:return
        self.rendered=signature
        self.connection.set_text('Phone connected' if fresh else 'Phone unavailable')
        messages={'submitted':'Command submitted. Waiting for fresh phone state.',
                  'unknown':'Command outcome unknown. Check the current call state before another action.',
                  'rejected':'The phone rejected this command. Refreshing call state.'}
        self.notice.set_text('Sending command…' if self.model.pending else messages.get(self.model.outcome,''))
        for child in self.calls.get_children():self.calls.remove(child)
        snapshot=self.model.snapshot
        if not fresh or not snapshot:
            label=Gtk.Label(label='Connect the paired Android bridge to view calls.',xalign=0);label.set_line_wrap(True);self.calls.pack_start(label,False,False,0)
        elif not snapshot['calls']:
            label=Gtk.Label(label='Call details unavailable.',xalign=0);self.calls.pack_start(label,False,False,0)
        else:
            for call in snapshot['calls']:
                card=Gtk.Box(orientation=Gtk.Orientation.VERTICAL,spacing=10)
                state={1:'Dialing',2:'Incoming call',3:'On hold',4:'Call in progress',7:'Disconnected',9:'Connecting'}.get(call['state'],'Call state unavailable')
                card.pack_start(Gtk.Label(label=state,xalign=0),False,False,0)
                buttons=Gtk.Box(spacing=10)
                for action in call['actions']:
                    button=Gtk.Button(label={'answer':'Answer','reject':'Decline','disconnect':'End call'}[action])
                    button.get_style_context().add_class('action' if action=='answer' else 'destructive')
                    button.set_sensitive(fresh and not self.model.pending)
                    button.connect('clicked',self.command,snapshot['generation'],snapshot['revision'],call['id'],action)
                    buttons.pack_start(button,True,True,0)
                card.pack_start(buttons,False,False,0);self.calls.pack_start(card,False,False,0)
            if not snapshot['controls_allowed']:
                hint=Gtk.Label(label='Enable desktop call controls in Android Bridge settings.',xalign=0);hint.set_line_wrap(True);self.calls.pack_start(hint,False,False,0)
        self.calls.show_all()

    def command(self,button,generation,revision,call_id,action):
        try:command=self.model.begin(generation,revision,call_id,action,elapsed_time())
        except Unavailable:self.model.disconnect();self.render();return
        self.epoch+=1 # Invalidate polls started before this command.
        path=self.config;self.render()
        future=self.pool.submit(lambda:self.client(path).control(**command))
        future.add_done_callback(lambda completed:GLib.idle_add(self.finished,completed))

    def finished(self,future):
        if self.closed:return False
        try:response=future.result()
        except (OSError,ValueError):response=None
        self.epoch+=1 # Require an observation started after the command outcome.
        self.model.finish(response);self.render();return False

    def choose(self,button):
        if self.model.pending or self.model.dial_pending:return
        chooser=Gtk.FileChooserDialog(title='Choose client.json from desktop enrollment',parent=self,action=Gtk.FileChooserAction.OPEN)
        chooser.add_buttons('Cancel',Gtk.ResponseType.CANCEL,'Choose',Gtk.ResponseType.OK)
        filter_=Gtk.FileFilter();filter_.set_name('Desktop pairing configuration (*.json)');filter_.add_pattern('*.json');chooser.add_filter(filter_)
        if chooser.run()==Gtk.ResponseType.OK:
            self.config=Path(chooser.get_filename());self.epoch+=1;self.model.disconnect();self.rendered=None
            try:
                self.settings.mkdir(mode=0o700,parents=True,exist_ok=True)
                with tempfile.NamedTemporaryFile(mode='w',dir=self.settings,delete=False) as out:
                    json.dump({'config':str(self.config)},out);temporary=Path(out.name)
                temporary.replace(self.settings/'settings.json')
            except OSError:self.notice.set_text('Pairing file selected for this session. Settings could not be saved.')
            self.render()
        chooser.destroy()

    def close(self,*args):
        self.closed=True;GLib.source_remove(self.timer);self.pool.shutdown(wait=False,cancel_futures=True)
        application=self.get_application()
        if application is not None:application.quit()
        else:Gtk.main_quit()


class PhoneApplication(Gtk.Application):
    def __init__(self,config=None):
        super().__init__(application_id='org.omarchy.Phone',flags=Gio.ApplicationFlags.DEFAULT_FLAGS)
        self.config=config;self.window=None
    def do_activate(self):
        if self.window is None:
            self.window=PhoneWindow(self.config);self.add_window(self.window)
        self.window.present()

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--config',type=Path);args=parser.parse_args()
    return PhoneApplication(args.config).run([sys.argv[0]])

if __name__=='__main__':main()
