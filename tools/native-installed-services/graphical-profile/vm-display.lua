-- Explicit private virtual Pixel 10 deployment; Source policy is unchanged.
hl.monitor({ output="",mode="1080x2424@60",position="auto",scale=2.625 })
hl.permission("/usr/libexec/pokelinux-fixture/pixel10-raw-output-capture","screencopy","allow")
hl.permission([[/usr/libexec/pokelinux-fixture/pixel10-physical-size-adapter\.so]],"plugin","allow")
hl.config({debug={disable_scale_checks=true,disable_logs=false,disable_time=false},animations={enabled=false},decoration={blur={enabled=false},shadow={enabled=false}},misc={disable_hyprland_logo=true,disable_splash_rendering=true}})
hl.permission("^/usr/lib/pokelinux/session-source[.]so$","plugin","allow")
hl.plugin.load("/usr/lib/pokelinux/session-source.so")
hl.plugin.load("/usr/lib/pokelinux/audio-focus-observer.so")
hl.plugin.load("/usr/libexec/pokelinux-fixture/profile-ready-bootstrap.so")
