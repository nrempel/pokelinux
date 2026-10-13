import QtQuick
import Quickshell.Io
import qs.Commons as Commons
QtObject {
    property var shell: null
    property var pluginRegistry: null
    property var barWidgetRegistry: null
    property var pluginManifest: null
    property var appLibrary: null
    property IpcHandler observation: IpcHandler {
        target: "native-theme-oracle"
        function snapshot(): string {
            return JSON.stringify({background:String(Commons.Color.background),
                foreground:String(Commons.Color.foreground),accent:String(Commons.Color.accent),
                fontFamily:Commons.Style.resolvedFontFamily,baseSize:Commons.Style.font.baseSize,
                display:Commons.Style.font.display,displayLarge:Commons.Style.font.displayLarge,
                heading:Commons.Style.font.heading,title:Commons.Style.font.title,
                body:Commons.Style.font.body,subtitle:Commons.Style.font.subtitle,
                spacingScale:Commons.Style.spacing.scale,
                controlGap:Commons.Style.spacing.controlGap,panelPadding:Commons.Style.spacing.panelPadding})
        }
    }
}
