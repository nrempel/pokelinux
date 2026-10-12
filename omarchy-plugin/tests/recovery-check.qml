import QtQuick
import Quickshell
import "../qml"
ShellRoot {
    id: test
    property int step: 0
    PhoneService {
        id: service
        clientCommand: ["/usr/bin/omarchy-phone-local","--socket","/tmp/phone-shell-fixture/runtime/phone.sock"]
    }
    Timer {
        interval: 50; running: true; repeat: true
        onTriggered: {
            var c=service.controller;
            if (test.step===0) {
                if (!('recoveryItems' in service)) { console.log('PHONE_RECOVERY_API_MISSING'); Qt.quit(); return; }
                if (service.recoveryItems.length!==1 || !service.canAcknowledge) return;
                if (!service.busy || service.requestAction('answer',c.contextKey)) throw new Error('recovery bypassed');
                if (!service.acknowledge(service.recoveryKey)) throw new Error('initial acknowledgment unavailable');
                test.step=1;
            } else if (test.step===1 && c.available && c.callState===2 && !service.busy) {
                var key=c.contextKey;
                if (!service.requestAction("answer",key)) throw new Error("answer not admitted");
                if (service.requestAction("answer",key)) throw new Error("overlapping user gesture admitted");
                test.step=2;
            } else if (test.step===2 && service.canAcknowledge && c.callState===4) {
                if (service.requestAction('disconnect',c.contextKey)) throw new Error('unknown bypassed');
                if (!service.acknowledge(service.recoveryKey)) throw new Error('lost reply acknowledgment unavailable');
                test.step=3;
            } else if (test.step===3 && c.available && c.callState===4 && !service.busy) {
                if (!service.requestAction("disconnect",c.contextKey)) throw new Error("end not admitted");
                test.step=4;
            } else if (test.step===4 && c.observationLive && c.callState===0 && !service.busy && c.lastOutcome==="submitted") {
                console.log("PHONE_SERVICE_CHECK_PASSED"); Qt.quit();
            }
        }
    }
    Timer { interval: 25000; running: true; onTriggered: { console.log("PHONE_SERVICE_CHECK_TIMEOUT",test.step); Qt.quit(); } }
}
