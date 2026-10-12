import QtQuick
import QtQuick.Controls

ApplicationWindow {
    id: root
    required property QtObject rpc
    required property QtObject power
    required property QtObject themeFiles
    required property string socketPath
    width: 390
    height: 844
    minimumWidth: 320
    minimumHeight: 480
    visible: true
    title: "Phone"
    color: palette.background
    PhoneTheme { id: palette; source: root.themeFiles }
    PhoneSession { id: session; objectName: "phoneSession"; rpc: root.rpc; socketPath: root.socketPath }
    PhoneActions { id: actions; rpc: root.rpc; session: session }
    PhoneLifecycle { power: root.power; session: session; actions: actions; rpc: root.rpc }
    PhoneObservations { session: session; actions: actions; active: root.visible && root.active }
    PhoneView { anchors.fill: parent; session: session; actions: actions; theme: palette }
}
