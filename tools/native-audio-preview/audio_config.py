"""Fixture-only private graph config from independently frozen monitor fixture; no subprocess starts here."""
import json,os,pathlib

def configure():
    root = pathlib.Path('/tmp/pokelinux-native-audio')
    root.mkdir(mode=0o755)
    private = root / 'private'
    private.mkdir(mode=0o700)
    control = root / 'control'
    control.mkdir(mode=0o755)
    config = root / 'config'
    config.mkdir(mode=0o700)
    state = root / 'state'
    state.mkdir(mode=0o700)
    wp_config = config / 'wireplumber' / 'wireplumber.conf.d'
    wp_config.mkdir(parents=True)
    (wp_config / 'native-audio.conf').write_text('''wireplumber.profiles = {
      native-audio-test = {
        inherits = [ base ]
        metadata.sm-settings = required
        metadata.sm-objects = required
        policy.standard = disabled
        hardware.audio = disabled
        hardware.bluetooth = disabled
        hardware.video-capture = disabled
        support.logind = disabled
        support.portal-permissionstore = disabled
      }
    }
    ''')

    objects = ['''{ factory = spa-node-factory args = {
      factory.name = support.node.driver node.name = Test-Driver priority.driver = 20000
    } }''']
    for name, factory, media_class in [
        ('route.speaker', 'support.null-audio-sink', 'Audio/Sink'),
        ('route.earpiece', 'support.null-audio-sink', 'Audio/Sink'),
        ('route.capture', 'support.null-audio-sink', 'Audio/Sink'),
    ]:
        extra = '' if factory != 'audiotestsrc' else 'node.param.PortConfig = { direction = Output mode = dsp format = { mediaType = audio mediaSubtype = raw format = F32 rate = 48000 channels = 1 position = [ MONO ] } }'
        if factory == 'audiotestsrc':
            extra += ' node.param.Props = { waveType = 0 volume = 0.25 }'
        monitor = 'true' if name in ('route.speaker', 'route.earpiece') else 'false'
        objects.append('''{ factory = adapter args = {
            factory.name = "%s" node.name = "%s" media.class = "%s"
            audio.format = F32 audio.rate = 48000 audio.channels = 1 audio.position = [ MONO ]
            adapter.auto-port-config = { mode = dsp monitor = %s position = preserve }
            %s
        } }''' % (factory, name, media_class, monitor, extra))
    pw_config = config / 'pipewire.conf'
    pw_config.write_text('''context.properties = {
      core.daemon = true core.name = pokelinux-audio-test
      default.clock.rate = 48000 default.clock.quantum = 256
    }
    context.spa-libs = {
      audio.convert.* = audioconvert/libspa-audioconvert
      audiotestsrc = audiotestsrc/libspa-audiotestsrc
      support.* = support/libspa-support
    }
    context.modules = [
      { name = libpipewire-module-protocol-native }
      { name = libpipewire-module-spa-node-factory }
      { name = libpipewire-module-client-node }
      { name = libpipewire-module-adapter }
      { name = libpipewire-module-link-factory }
      { name = libpipewire-module-metadata }
      { name = libpipewire-module-access args = { access.socket = { pokelinux-audio-test = "unrestricted" pokelinux-audio-test-manager = "unrestricted" } } }
    ]
    context.objects = [\n''' + '\n'.join(objects) + '\n]\n')
    profile = config / 'profile.json'
    profile.write_text(json.dumps({'sources': {'media': 'source.media', 'call': 'source.call', 'capture': 'source.microphone'},
                                   'routes': [
                                       {'id':'media-z2','label':'Headphones (virtual)','target':'route.earpiece','roles':['media']},
                                       {'id':'call-only','label':'Call (virtual)','target':'route.earpiece','roles':['call']},
                                       {'id':'media-a7','label':'Speaker (virtual)','target':'route.speaker','roles':['media']},
                                       {'id':'capture-only','label':'Capture (virtual)','target':'route.capture','roles':['capture']}] }))
    env = dict(os.environ, XDG_RUNTIME_DIR=str(private), PIPEWIRE_RUNTIME_DIR=str(private),
               PIPEWIRE_REMOTE='pokelinux-audio-test-manager', XDG_CONFIG_HOME=str(config),
               XDG_STATE_HOME=str(state), DISABLE_RTKIT='1', PIPEWIRE_DEBUG='3')
    for p in [root,*root.rglob("*")]:os.chown(p,11040,11040)
    return root,private,control,profile,env
