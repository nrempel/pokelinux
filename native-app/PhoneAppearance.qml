import QtQuick

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
    readonly property real fontScale: values.fontScale === undefined ? 1 : values.fontScale
    readonly property string fontFamily: values.fontFamily || "monospace"
    readonly property string monoFamily: fontFamily
    readonly property var fontTokens: values.fontTokens || ({caption:10,bodySmall:11,body:12,subtitle:13,title:14,heading:16,display:24,displayLarge:28})
    readonly property var spacingTokens: values.spacingTokens || ({controlGap:8,panelPadding:18,panelGap:14,rowGap:8,controlPaddingX:10,controlPaddingY:6})
    function fontPixel(token, fallback) {
        return fontTokens[token] === undefined ? Math.max(1,Math.round(fallback*fontScale)) : fontTokens[token]
    }
    function spacingPixel(token, fallback) {
        return spacingTokens[token] === undefined ? fallback : spacingTokens[token]
    }
    function refresh() {
        const next = source.available ? source.snapshot : null
        available = next !== null
        if (next) values = next
    }
    Component.onCompleted: refresh()
    property Connections sourceChanges: Connections {
        target: root.source
        function onChanged() { root.refresh() }
    }
}
