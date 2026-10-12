import QtQuick
import Quickshell
import "../qml"

ShellRoot {
    id: test
    property int step: 0
    property int completions: 0
    function verify(value,message) { if (!value) throw new Error(message); }
    function next() {
        var commands=[
            ["/usr/bin/python3","-c","import json,sys;print(json.dumps({'argument':sys.argv[1]}))","a '$HOME' ; literal"],
            ["/usr/bin/python3","-c","print('not-json')"],
            ["/usr/bin/python3","-c","import sys;print('{}');sys.exit(3)"],
            ["/usr/bin/python3","-c","import time;time.sleep(10)"],
            ["/missing-phone-test-command"],
            ["/usr/bin/python3","-c","print('{\"recovered\":true}')"]
        ];
        if (step===commands.length) {
            verify(completions===6,"one completion per job");
            console.log("PHONE_PROCESS_CHECK_PASSED"); Qt.quit(); return;
        }
        verify(command.start(commands[step],step),"start accepted");
        verify(!command.start(commands[step],99),"overlap rejected");
    }
    JsonCommand {
        id: command
        timeoutMs: 1500
        onCompleted: function(context,ok,value) {
            test.verify(context===test.step,"matching job context");
            test.completions++;
            if (test.step===0) test.verify(ok && value.argument==="a '$HOME' ; literal","literal argv preserved");
            else if (test.step===5) test.verify(ok && value.recovered===true,"new job after failures");
            else test.verify(!ok && value===null,"failure stays a failure");
            test.step++; Qt.callLater(test.next);
        }
    }
    Timer { interval: 100; running: true; onTriggered: test.next() }
    Timer { interval: 20000; running: true; onTriggered: { console.log("PHONE_PROCESS_CHECK_TIMEOUT"); Qt.quit(); } }
}
