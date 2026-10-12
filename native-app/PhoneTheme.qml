import QtQuick
import "ThemePalette.js" as Palette

QtObject {
    id: root
    required property QtObject source
    property bool available: false
    property var values: ({background:"#101315",foreground:"#cacccc",surface:"#101315",
        border:"#cacccc",accent:"#cacccc",muted:"#707880",urgent:"#a55555",
        positive:"#cacccc",ink:"#101315",fontScale:1})
    readonly property color background: values.background
    readonly property color foreground: values.foreground
    readonly property color surface: values.surface
    readonly property color border: values.border
    readonly property color accent: values.accent
    readonly property color muted: values.muted
    readonly property color urgent: values.urgent
    readonly property color positive: values.positive
    readonly property color ink: values.ink
    readonly property real fontScale: values.fontScale
    readonly property string fontFamily: "monospace"
    readonly property string monoFamily: "monospace"
    function refresh() {
        const next = source.available ? Palette.palette(source.snapshot) : null
        available = next !== null
        if (next) values = next
    }
    Component.onCompleted: refresh()
    property Connections sourceChanges: Connections {
        target: root.source
        function onChanged() { root.refresh() }
    }
}
