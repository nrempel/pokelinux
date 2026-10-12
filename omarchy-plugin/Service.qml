import QtQuick
import "qml"

PhoneService {
    id: root
    property string presentedCall: ""
    function presentIncoming() {
        var c=controller;
        if (!shell || !c.available || c.callState!==2 || !c.call || !c.value) return;
        var key=JSON.stringify([c.observation.epoch,c.value.generation,c.call.id]);
        if (key===presentedCall) return;
        if (shell.summon("nbrempel.phone","{}")) presentedCall=key;
    }
    onShellChanged: Qt.callLater(presentIncoming)
    Connections {
        target: root.controller
        function onObservationChanged() { Qt.callLater(root.presentIncoming); }
        function onObservationLiveChanged() { Qt.callLater(root.presentIncoming); }
    }
}
